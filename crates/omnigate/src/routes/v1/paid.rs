//! RO:QUICKCHAIN-PREFLIGHT — prepare/estimate are read-only; write is proxy-only; no wallet, ledger, accounting, or storage mutation here; wallet receipt verification; capture/release.
//! RO:WHAT — v1 paid-access routes for WEB3 product UX.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Omnigate exposes paid estimate/write/prepare BFF routes.
//! RO:INTERACTS — `svc-storage` `/paid/o/estimate` and `/paid/o`, `svc-gateway` paid routes.
//! RO:INVARIANTS — prepare/estimate are read-only; write is proxy-only; no wallet, ledger, accounting, or storage mutation here.
//! RO:METRICS — route is covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_STORAGE_BASE_URL` or `OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL`.
//! RO:SECURITY — forwards selected request headers only; skips hop-by-hop headers and host.
//! RO:TEST — `tests/paid_storage_estimate_proxy.rs`, `tests/paid_storage_write_proxy.rs`, `tests/paid_storage_prepare.rs`.

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderName, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const PREPARE_SCHEMA: &str = "omnigate.paid-object-prepare.v1";
const DEFAULT_ACTION: &str = "paid_storage_put";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate paid route reqwest client should build")
});

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaidObjectPrepareRequest {
    bytes: u64,
    #[serde(default)]
    payer_account: Option<String>,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    asset_kind: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    expected_asset_cid: Option<String>,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct PaidObjectPrepareResponse {
    schema: &'static str,
    action: String,
    asset: String,
    bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    asset_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_asset_cid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_passport_subject: Option<String>,
    storage_estimate: Value,
    wallet_hold: WalletHoldTemplate,
    submit: SubmitTemplate,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WalletHoldTemplate {
    required: bool,
    action: String,
    currency: &'static str,
    amount_minor: String,
    minimum_hold_minor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payer_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key_hint: Option<String>,
    capability: WalletCapabilityHint,
}

#[derive(Debug, Serialize)]
struct WalletCapabilityHint {
    required_action: &'static str,
    resource: &'static str,
    audience: &'static str,
    recommended_ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
struct SubmitTemplate {
    method: &'static str,
    gateway_path: &'static str,
    omnigate_path: &'static str,
    storage_path: &'static str,
    required_headers: Vec<&'static str>,
    optional_headers: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct UpstreamProblem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

#[derive(Debug, Serialize)]
struct StorageEstimateRejectedProblem {
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
    storage_status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_error: Option<Value>,
}

/// Router for `/v1/paid/*`.
///
/// Mounted from `routes::v1::router()` as:
/// `nest("/paid", paid::router())`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/o/prepare", post(prepare))
        .route("/o/estimate", get(estimate))
        .route("/o", post(write).put(write))
}

/// Prepare a paid object write.
///
/// This is a product/BFF preflight route:
///
/// ```text
/// client/gateway
/// → omnigate /v1/paid/o/prepare
/// → svc-storage /paid/o/estimate
/// → stable prepare DTO with wallet hold template
/// ```
///
/// This route intentionally does not call wallet, create a hold, capture,
/// release, write bytes, write index pointers, or mutate ledger/accounting.
pub async fn prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<PaidObjectPrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return prepare_problem(
                StatusCode::BAD_REQUEST,
                "invalid_prepare_request",
                "prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.bytes == 0 {
        return prepare_problem(
            StatusCode::BAD_REQUEST,
            "invalid_prepare_request",
            "bytes must be greater than zero",
            false,
            "invalid_bytes",
        );
    }

    let storage_estimate = match fetch_storage_estimate(request.bytes, headers).await {
        Ok(storage_estimate) => storage_estimate,
        Err(response) => return response,
    };

    let action =
        value_string(&storage_estimate, "action").unwrap_or_else(|| DEFAULT_ACTION.to_owned());
    let asset =
        value_string(&storage_estimate, "asset").unwrap_or_else(|| DEFAULT_ASSET.to_owned());

    let Some(amount_minor) = value_string(&storage_estimate, "amount_minor")
        .or_else(|| value_string(&storage_estimate, "amount_minor_units"))
        .or_else(|| value_string(&storage_estimate, "amount"))
    else {
        return prepare_problem(
            StatusCode::BAD_GATEWAY,
            "storage_estimate_missing_amount",
            "storage estimate did not include an amount",
            true,
            "storage_estimate_missing_amount",
        );
    };

    let minimum_hold_minor = value_string(&storage_estimate, "minimum_hold_minor")
        .or_else(|| value_string(&storage_estimate, "minimum_hold_minor_units"))
        .unwrap_or_else(|| amount_minor.clone());

    let idempotency_key_hint = request
        .client_idempotency_key
        .clone()
        .or_else(|| Some(format!("prepare:{action}:{}", request.bytes)));

    let response = PaidObjectPrepareResponse {
        schema: PREPARE_SCHEMA,
        action: action.clone(),
        asset,
        bytes: request.bytes,
        asset_kind: request.asset_kind,
        content_type: request.content_type,
        expected_asset_cid: request.expected_asset_cid,
        owner_passport_subject: request.owner_passport_subject,
        storage_estimate,
        wallet_hold: WalletHoldTemplate {
            required: true,
            action,
            currency: DEFAULT_CURRENCY,
            amount_minor,
            minimum_hold_minor,
            payer_account: request.payer_account,
            idempotency_key_hint,
            capability: WalletCapabilityHint {
                required_action: "wallet.hold",
                resource: "paid_storage_put",
                audience: "svc-wallet",
                recommended_ttl_seconds: 300,
            },
        },
        submit: SubmitTemplate {
            method: "POST",
            gateway_path: "/paid/o",
            omnigate_path: "/v1/paid/o",
            storage_path: "/paid/o",
            required_headers: vec!["Authorization", "Idempotency-Key", "x-ron-wallet-hold-txid"],
            optional_headers: vec![
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-permission",
                "x-ron-spend-limit",
                "x-correlation-id",
                "x-request-id",
            ],
        },
        warnings: Vec::new(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Proxy `GET /v1/paid/o/estimate?bytes=N` to `svc-storage /paid/o/estimate`.
///
/// This endpoint is intentionally side-effect free. It does not create wallet
/// holds, mutate ledger state, write object bytes, export accounting events, or
/// resolve asset manifests.
pub async fn estimate(uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/paid/o/estimate", uri.query());
    proxy_to_storage(
        Method::GET,
        &upstream_path,
        headers,
        Bytes::new(),
        "storage estimate upstream unavailable",
    )
    .await
}

/// Proxy `POST/PUT /v1/paid/o` to `svc-storage /paid/o`.
///
/// This is intentionally proxy-only. The actual paid-write behavior stays in
/// `svc-storage`, including body hashing, wallet receipt verification,
/// capture/release, usage event creation, and accounting export.
pub async fn write(method: Method, headers: HeaderMap, body: Bytes) -> Response {
    proxy_to_storage(
        method,
        "/paid/o",
        headers,
        body,
        "storage paid route upstream unavailable",
    )
    .await
}

async fn fetch_storage_estimate(bytes: u64, headers: HeaderMap) -> Result<Value, Response> {
    let upstream_path = format!("/paid/o/estimate?bytes={bytes}");
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let mut req_builder = HTTP_CLIENT.get(&upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(upstream_problem(
                "storage estimate upstream unavailable",
                "storage_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body_bytes = match upstream_res.bytes().await {
        Ok(body_bytes) => body_bytes,
        Err(_) => {
            return Err(upstream_problem(
                "storage estimate upstream unavailable",
                "storage_read",
            ));
        }
    };

    let parsed = serde_json::from_slice::<Value>(&body_bytes).ok();

    if !status.is_success() {
        let storage_error = parsed.or_else(|| {
            Some(Value::String(
                String::from_utf8_lossy(&body_bytes).to_string(),
            ))
        });

        return Err((
            status,
            Json(StorageEstimateRejectedProblem {
                code: "storage_estimate_rejected",
                message: "storage estimate rejected prepare request",
                retryable: status.as_u16() >= 500,
                reason: "storage_estimate_rejected",
                storage_status: status.as_u16(),
                storage_error,
            }),
        )
            .into_response());
    }

    let Some(parsed) = parsed else {
        return Err(prepare_problem(
            StatusCode::BAD_GATEWAY,
            "storage_estimate_bad_json",
            "storage estimate response was not valid JSON",
            true,
            "storage_bad_json",
        ));
    };

    Ok(parsed)
}

async fn proxy_to_storage(
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
    unavailable_message: &'static str,
) -> Response {
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let reqwest_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(method) => method,
        Err(_) => return upstream_problem(unavailable_message, "bad_method"),
    };

    let mut req_builder = HTTP_CLIENT.request(reqwest_method, &upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.body(body).send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => return upstream_problem(unavailable_message, "storage_connect"),
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    let body_bytes = match upstream_res.bytes().await {
        Ok(body_bytes) => body_bytes,
        Err(_) => return upstream_problem(unavailable_message, "storage_read"),
    };

    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() = status;

    for (name, value) in &upstream_headers {
        if should_copy_response_header(name) {
            response.headers_mut().insert(name.clone(), value.clone());
        }
    }

    response
}

fn with_query(path: &str, query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{path}?{query}"),
        _ => path.to_string(),
    }
}

fn storage_base_url() -> String {
    std::env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| std::env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_string())
}

fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || super::header_policy::is_allowed_ron_context_header(name)
        || name.as_str() == "x-correlation-id"
        || name.as_str() == "x-request-id"
        || name.as_str() == "idempotency-key"
}

fn should_copy_response_header(name: &HeaderName) -> bool {
    name != header::TRANSFER_ENCODING
        && name != header::CONTENT_LENGTH
        && name != header::CONNECTION
}

fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    name == header::HOST
        || name == header::CONNECTION
        || name == header::PROXY_AUTHORIZATION
        || name == header::TE
        || name == header::TRAILER
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn upstream_problem(message: &'static str, reason: &'static str) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(UpstreamProblem {
            code: "upstream_unavailable",
            message,
            retryable: true,
            reason,
        }),
    )
        .into_response()
}

fn prepare_problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> Response {
    (
        status,
        Json(UpstreamProblem {
            code,
            message,
            retryable,
            reason,
        }),
    )
        .into_response()
}
