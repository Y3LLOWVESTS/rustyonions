//! RO:QUICKCHAIN-PREFLIGHT — quote is read-only; pay uses svc-wallet only; no direct ledger mutation; integer minor units only; wallet_receipt.
//! RO:WHAT — Paid named-site visit quote/pay routes for CrabLink.
//! RO:WHY — P12 Economics; Concerns: ECON/SEC/DX. Visitors must pay site owners through wallet/ledger truth before paid site render.
//! RO:INTERACTS — sites manifest/index helpers, svc-index, svc-storage, svc-wallet /v1/transfer, svc-gateway product proxy.
//! RO:INVARIANTS — quote is read-only; pay uses svc-wallet only; no direct ledger mutation; integer minor units only.
//! RO:METRICS — covered by omnigate HTTP middleware and downstream wallet metrics.
//! RO:CONFIG — OMNIGATE_SITE_VISIT_PRICE_MINOR, OMNIGATE_WALLET_BASE_URL, OMNIGATE_WALLET_BEARER.
//! RO:SECURITY — strict DTOs; route payout recipient must match manifest payout; hop-by-hop headers are filtered.
//! RO:TEST — manual: /sites/:name/visit/quote then /pay; future test target: omnigate site_visit_pay.

use axum::{
    body::Bytes,
    extract::Path,
    http::{header, HeaderMap, HeaderName, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, time::Duration};

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";
const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";
const DEFAULT_WALLET_BEARER: &str = "dev";
const DEFAULT_SITE_VISIT_PRICE_MINOR: &str = "10";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";
const DEFAULT_WALLET_NONCE: u64 = 1;
const SITE_VISIT_QUOTE_SCHEMA: &str = "omnigate.site-visit-quote.v1";
const SITE_VISIT_PAYMENT_SCHEMA: &str = "omnigate.site-visit-payment.v1";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate site_visit route reqwest client should build")
});

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteVisitQuoteRequest {
    #[serde(default)]
    site_name: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    crab_url: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    quantity: Option<u64>,
    #[serde(default)]
    payer_account: Option<String>,
    #[serde(default)]
    visitor_wallet_account: Option<String>,
    #[serde(default)]
    visitor_passport_subject: Option<String>,
    #[serde(default)]
    payer_passport_subject: Option<String>,
    #[serde(default)]
    recipient_account: Option<String>,
    #[serde(default)]
    max_amount_minor: Option<String>,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteVisitPayRequest {
    #[serde(default)]
    site_name: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    crab_url: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    action: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    quantity: Option<u64>,
    #[serde(default)]
    payer_account: Option<String>,
    #[serde(default)]
    visitor_wallet_account: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    visitor_passport_subject: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    payer_passport_subject: Option<String>,
    #[serde(default)]
    recipient_account: Option<String>,
    #[serde(default)]
    amount_minor: Option<String>,
    #[serde(default)]
    asset: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    quote_id: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    quote_hash: Option<String>,
    #[serde(default)]
    nonce: Option<u64>,
    #[allow(dead_code)]
    #[serde(default)]
    quote: Option<Value>,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestPointer {
    version: u16,
    name: String,
    manifest_cid: String,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    owner_wallet_account: Option<String>,
    updated_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestDocument {
    version: u16,
    site_name: String,
    root_document_cid: String,
    #[allow(dead_code)]
    #[serde(default)]
    asset_map: Value,
    #[allow(dead_code)]
    #[serde(default)]
    route_map: Value,
    #[serde(default)]
    owner: Option<SiteManifestOwner>,
    #[serde(default)]
    payout: Option<SiteManifestPayout>,
    #[serde(default)]
    metadata: Option<SiteManifestMetadata>,
    #[allow(dead_code)]
    #[serde(default)]
    provenance: Option<Value>,
    #[allow(dead_code)]
    #[serde(default)]
    storage: Option<Value>,
    #[allow(dead_code)]
    #[serde(default)]
    receipts: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestOwner {
    #[serde(default)]
    passport_subject: Option<String>,
    #[serde(default)]
    wallet_account: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestPayout {
    #[serde(default)]
    default_action: Option<String>,
    #[serde(default)]
    recipient_account: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    splits: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestMetadata {
    #[serde(default)]
    title: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    description: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct SiteVisitContext {
    site_name: String,
    manifest_cid: String,
    updated_at_ms: u64,
    root_document_cid: String,
    owner_passport_subject: Option<String>,
    owner_wallet_account: Option<String>,
    payout_action: String,
    payout_recipient_account: String,
    title: Option<String>,
}

#[derive(Debug)]
struct UpstreamBody {
    status: StatusCode,
    body: Bytes,
}

#[derive(Debug, Serialize)]
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

/// Quote the required ROC payment for a named site visit.
pub async fn site_visit_quote(
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request = match serde_json::from_slice::<SiteVisitQuoteRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_visit_quote_request",
                "site visit quote request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let site_name = match normalize_site_name(&name) {
        Ok(name) => name,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_name",
                "site_name is not a safe beta site name",
                false,
                reason,
            );
        }
    };

    if let Some(body_site_name) = request.site_name.as_deref() {
        if normalize_site_name(body_site_name).ok().as_deref() != Some(site_name.as_str()) {
            return problem(
                StatusCode::BAD_REQUEST,
                "site_visit_site_mismatch",
                "body site_name must match route site name",
                false,
                "site_mismatch",
            );
        }
    }

    let payer_account = match site_visit_payer_account(
        &headers,
        request.payer_account.as_deref(),
        request.visitor_wallet_account.as_deref(),
    ) {
        Some(account) => account,
        None => {
            return problem(
                StatusCode::BAD_REQUEST,
                "site_visit_missing_payer",
                "site visit quote requires a payer wallet account",
                false,
                "missing_payer_account",
            );
        }
    };

    if let Some(header_account) = grab(&headers, "x-ron-wallet-account") {
        if header_account != payer_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "wallet_account_mismatch",
                "x-ron-wallet-account must match site visit payer account",
                false,
                "payer_mismatch",
            );
        }
    }

    let ctx = match load_site_visit_context(&site_name, &headers).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    if ctx.payout_action != "site_visit" {
        return problem(
            StatusCode::BAD_REQUEST,
            "site_visit_not_payable",
            "site manifest payout action is not site_visit",
            false,
            "site_visit_not_payable",
        );
    }

    if let Some(request_recipient) = clean_optional(request.recipient_account.as_deref()) {
        if request_recipient != ctx.payout_recipient_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "site_visit_recipient_mismatch",
                "site visit recipient must match site manifest payout recipient",
                false,
                "recipient_mismatch",
            );
        }
    }

    let amount_minor = site_visit_price_minor();
    let quantity = request.quantity.unwrap_or(1).max(1);

    if let Some(max_amount_minor) = clean_optional(request.max_amount_minor.as_deref()) {
        let Ok(max_amount_minor) = normalize_minor_units_owned(&max_amount_minor) else {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_visit_max_amount",
                "max_amount_minor must be a positive integer string",
                false,
                "bad_max_amount_minor",
            );
        };

        if decimal_gt(&amount_minor, &max_amount_minor) {
            return problem(
                StatusCode::CONFLICT,
                "site_visit_quote_exceeds_max",
                "site visit quote exceeds caller max_amount_minor",
                false,
                "quote_exceeds_max",
            );
        }
    }

    let visitor_passport_subject = clean_optional(request.visitor_passport_subject.as_deref())
        .or_else(|| clean_optional(request.payer_passport_subject.as_deref()))
        .or_else(|| grab(&headers, "x-ron-passport"));
    let quote_id = site_visit_quote_id(
        &site_name,
        &payer_account,
        &ctx.payout_recipient_account,
        &amount_minor,
    );
    let quote_hash = simple_hex_hash(&quote_id);
    let idempotency_key = clean_optional(request.client_idempotency_key.as_deref())
        .or_else(|| grab(&headers, "idempotency-key"))
        .unwrap_or_else(|| {
            site_visit_idempotency_key(
                "quote",
                &site_name,
                &payer_account,
                &ctx.payout_recipient_account,
            )
        });

    let response = json!({
        "schema": SITE_VISIT_QUOTE_SCHEMA,
        "ok": true,
        "site_name": &site_name,
        "crab_url": format!("crab://{}", ctx.site_name),
        "action": "site_visit",
        "asset": DEFAULT_ASSET,
        "amount_minor": &amount_minor,
        "display_amount": format!("{} {}", amount_minor, DEFAULT_CURRENCY),
        "quantity": quantity,
        "payer_account": &payer_account,
        "visitor_wallet_account": &payer_account,
        "visitor_passport_subject": &visitor_passport_subject,
        "recipient_account": &ctx.payout_recipient_account,
        "quote_id": &quote_id,
        "quote_hash": &quote_hash,
        "client_idempotency_key": &idempotency_key,
        "expires_in_seconds": 300,
        "quote": {
            "schema": SITE_VISIT_QUOTE_SCHEMA,
            "site_name": &ctx.site_name,
            "crab_url": format!("crab://{}", ctx.site_name),
            "action": "site_visit",
            "asset": DEFAULT_ASSET,
            "amount_minor": &amount_minor,
            "display_amount": format!("{} {}", amount_minor, DEFAULT_CURRENCY),
            "quantity": quantity,
            "payer_account": &payer_account,
            "visitor_wallet_account": &payer_account,
            "visitor_passport_subject": &visitor_passport_subject,
            "recipient_account": &ctx.payout_recipient_account,
            "quote_id": &quote_id,
            "quote_hash": &quote_hash,
            "expires_in_seconds": 300,
            "policy": {
                "source": "omnigate.site_visit.v1",
                "fixed_dev_price_minor": &amount_minor,
                "price_env": "OMNIGATE_SITE_VISIT_PRICE_MINOR",
                "wallet_front_door": "svc-wallet /v1/transfer"
            },
            "site": {
                "manifest_cid": &ctx.manifest_cid,
                "root_document_cid": &ctx.root_document_cid,
                "owner_passport_subject": &ctx.owner_passport_subject,
                "owner_wallet_account": &ctx.owner_wallet_account,
                "title": &ctx.title,
                "updated_at_ms": ctx.updated_at_ms
            }
        },
        "next": {
            "pay": format!("/v1/sites/{}/visit/pay", ctx.site_name),
            "public_pay": format!("/sites/{}/visit/pay", ctx.site_name),
            "required": ["payer_account", "amount_minor", "recipient_account"]
        }
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Pay for a named site visit through `svc-wallet` transfer.
pub async fn site_visit_pay(Path(name): Path<String>, headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<SiteVisitPayRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_visit_pay_request",
                "site visit pay request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let site_name = match normalize_site_name(&name) {
        Ok(name) => name,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_name",
                "site_name is not a safe beta site name",
                false,
                reason,
            );
        }
    };

    if let Some(body_site_name) = request.site_name.as_deref() {
        if normalize_site_name(body_site_name).ok().as_deref() != Some(site_name.as_str()) {
            return problem(
                StatusCode::BAD_REQUEST,
                "site_visit_site_mismatch",
                "body site_name must match route site name",
                false,
                "site_mismatch",
            );
        }
    }

    let payer_account = match site_visit_payer_account(
        &headers,
        request.payer_account.as_deref(),
        request.visitor_wallet_account.as_deref(),
    ) {
        Some(account) => account,
        None => {
            return problem(
                StatusCode::BAD_REQUEST,
                "site_visit_missing_payer",
                "site visit payment requires a payer wallet account",
                false,
                "missing_payer_account",
            );
        }
    };

    if let Some(header_account) = grab(&headers, "x-ron-wallet-account") {
        if header_account != payer_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "wallet_account_mismatch",
                "x-ron-wallet-account must match site visit payer account",
                false,
                "payer_mismatch",
            );
        }
    }

    let ctx = match load_site_visit_context(&site_name, &headers).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    if ctx.payout_action != "site_visit" {
        return problem(
            StatusCode::BAD_REQUEST,
            "site_visit_not_payable",
            "site manifest payout action is not site_visit",
            false,
            "site_visit_not_payable",
        );
    }

    if payer_account == ctx.payout_recipient_account {
        return problem(
            StatusCode::BAD_REQUEST,
            "site_visit_self_payment_not_allowed",
            "site owner self-visit payment is not a wallet transfer",
            false,
            "self_payment_not_allowed",
        );
    }

    if let Some(request_recipient) = clean_optional(request.recipient_account.as_deref()) {
        if request_recipient != ctx.payout_recipient_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "site_visit_recipient_mismatch",
                "site visit recipient must match site manifest payout recipient",
                false,
                "recipient_mismatch",
            );
        }
    }

    let amount_minor = match clean_optional(request.amount_minor.as_deref()) {
        Some(amount_minor) => match normalize_minor_units_owned(&amount_minor) {
            Ok(amount_minor) => amount_minor,
            Err(_) => {
                return problem(
                    StatusCode::BAD_REQUEST,
                    "invalid_site_visit_amount",
                    "amount_minor must be a positive integer string",
                    false,
                    "bad_amount_minor",
                );
            }
        },
        None => site_visit_price_minor(),
    };

    let expected_amount_minor = site_visit_price_minor();
    if amount_minor != expected_amount_minor {
        return problem(
            StatusCode::CONFLICT,
            "site_visit_amount_mismatch",
            "amount_minor must match the current site_visit quote",
            false,
            "amount_mismatch",
        );
    }

    let asset = clean_optional(request.asset.as_deref())
        .unwrap_or_else(|| DEFAULT_ASSET.to_owned())
        .to_ascii_lowercase();
    if asset != DEFAULT_ASSET {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_site_visit_asset",
            "site_visit payment currently supports only roc",
            false,
            "asset_must_be_roc",
        );
    }

    let idempotency_key = clean_optional(request.client_idempotency_key.as_deref())
        .or_else(|| grab(&headers, "idempotency-key"))
        .unwrap_or_else(|| {
            site_visit_idempotency_key(
                "pay",
                &site_name,
                &payer_account,
                &ctx.payout_recipient_account,
            )
        });
    let nonce = request.nonce.unwrap_or_else(default_site_visit_nonce);

    let wallet_response = match send_wallet_site_visit_transfer(
        &headers,
        &payer_account,
        &ctx.payout_recipient_account,
        &amount_minor,
        nonce,
        &idempotency_key,
        &site_name,
    )
    .await
    {
        Ok(response) => response,
        Err(response) => return response,
    };

    let wallet_status = wallet_response.status;
    let wallet_body = wallet_response.body;
    let wallet_json = serde_json::from_slice::<Value>(&wallet_body).ok();

    if !wallet_status.is_success() {
        if wallet_status == StatusCode::CONFLICT {
            if let Some(expected_nonce) = wallet_json
                .as_ref()
                .and_then(|value| value_string(value, "message"))
                .and_then(|message| parse_expected_nonce(&message))
            {
                if expected_nonce != nonce {
                    match send_wallet_site_visit_transfer(
                        &headers,
                        &payer_account,
                        &ctx.payout_recipient_account,
                        &amount_minor,
                        expected_nonce,
                        &idempotency_key,
                        &site_name,
                    )
                    .await
                    {
                        Ok(retry_response) if retry_response.status.is_success() => {
                            return site_visit_payment_response(
                                StatusCode::OK,
                                &ctx,
                                &payer_account,
                                &amount_minor,
                                expected_nonce,
                                &idempotency_key,
                                serde_json::from_slice::<Value>(&retry_response.body).ok(),
                            );
                        }
                        Ok(retry_response) => {
                            return wallet_transfer_problem(
                                retry_response.status,
                                retry_response.body,
                            )
                        }
                        Err(response) => return response,
                    }
                }
            }
        }
        return wallet_transfer_problem(wallet_status, wallet_body);
    }

    site_visit_payment_response(
        StatusCode::OK,
        &ctx,
        &payer_account,
        &amount_minor,
        nonce,
        &idempotency_key,
        wallet_json,
    )
}

async fn load_site_visit_context(
    site_name: &str,
    headers: &HeaderMap,
) -> Result<SiteVisitContext, Response> {
    let pointer = match fetch_site_pointer(site_name, headers).await {
        Ok(Some(pointer)) => pointer,
        Ok(None) => {
            return Err(problem(
                StatusCode::NOT_FOUND,
                "site_not_found",
                "site manifest pointer was not found",
                false,
                "site_not_found",
            ));
        }
        Err(response) => return Err(response),
    };

    if pointer.version != 1
        || pointer.name != site_name
        || !is_canonical_b3_cid(&pointer.manifest_cid)
    {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "site_pointer_invalid",
            "site pointer is invalid for paid site visit",
            true,
            "site_pointer_invalid",
        ));
    }

    let manifest = fetch_site_manifest(&pointer.manifest_cid, headers).await?;

    if manifest.version != 1
        || normalize_site_name(&manifest.site_name).ok().as_deref() != Some(site_name)
        || !is_canonical_b3_cid(&manifest.root_document_cid)
    {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_invalid",
            "site manifest is invalid for paid site visit",
            true,
            "site_manifest_invalid",
        ));
    }

    let owner_passport_subject = manifest
        .owner
        .as_ref()
        .and_then(|owner| owner.passport_subject.clone())
        .or(pointer.owner_passport_subject);
    let owner_wallet_account = manifest
        .owner
        .as_ref()
        .and_then(|owner| owner.wallet_account.clone())
        .or(pointer.owner_wallet_account);
    let payout_action = manifest
        .payout
        .as_ref()
        .and_then(|payout| payout.default_action.clone())
        .unwrap_or_else(|| "site_visit".to_owned());
    let payout_recipient_account = manifest
        .payout
        .as_ref()
        .and_then(|payout| payout.recipient_account.clone())
        .or_else(|| owner_wallet_account.clone())
        .and_then(|account| clean_optional(Some(&account)))
        .ok_or_else(|| {
            problem(
                StatusCode::BAD_GATEWAY,
                "site_visit_missing_recipient",
                "site manifest did not include a payout recipient account",
                true,
                "missing_recipient_account",
            )
        })?;
    let title = manifest
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.title.clone());

    Ok(SiteVisitContext {
        site_name: site_name.to_owned(),
        manifest_cid: pointer.manifest_cid,
        updated_at_ms: pointer.updated_at_ms,
        root_document_cid: manifest.root_document_cid,
        owner_passport_subject,
        owner_wallet_account,
        payout_action,
        payout_recipient_account,
        title,
    })
}

async fn fetch_site_pointer(
    site_name: &str,
    headers: &HeaderMap,
) -> Result<Option<SiteManifestPointer>, Response> {
    let route = format!("/v1/index/sites/{site_name}/manifest");
    let upstream_url = format!("{}{}", index_base_url().trim_end_matches('/'), route);
    let mut req_builder = HTTP_CLIENT.get(upstream_url);

    for (name, value) in headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index site pointer upstream unavailable",
                true,
                "index_connect",
            ));
        }
    };

    if upstream_res.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !upstream_res.status().is_success() {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "index_site_pointer_rejected",
            "index rejected site pointer lookup",
            upstream_res.status().as_u16() >= 500,
            "index_site_pointer_rejected",
        ));
    }

    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index site pointer upstream unavailable",
                true,
                "index_read",
            ));
        }
    };

    serde_json::from_slice::<SiteManifestPointer>(&body)
        .map(Some)
        .map_err(|_| {
            problem(
                StatusCode::BAD_GATEWAY,
                "index_site_pointer_bad_json",
                "index site pointer response was not valid JSON",
                true,
                "index_site_pointer_bad_json",
            )
        })
}

async fn fetch_site_manifest(
    manifest_cid: &str,
    headers: &HeaderMap,
) -> Result<SiteManifestDocument, Response> {
    let route = format!("/o/{manifest_cid}");
    let upstream_url = format!("{}{}", storage_base_url().trim_end_matches('/'), route);
    let mut req_builder = HTTP_CLIENT.get(upstream_url);

    for (name, value) in headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage site manifest upstream unavailable",
                true,
                "storage_connect",
            ));
        }
    };

    if upstream_res.status() == StatusCode::NOT_FOUND {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_missing",
            "site manifest object was not found in storage",
            true,
            "site_manifest_missing",
        ));
    }

    if !upstream_res.status().is_success() {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "storage_site_manifest_rejected",
            "storage rejected site manifest fetch",
            upstream_res.status().as_u16() >= 500,
            "storage_site_manifest_rejected",
        ));
    }

    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage site manifest upstream unavailable",
                true,
                "storage_read",
            ));
        }
    };

    serde_json::from_slice::<SiteManifestDocument>(&body).map_err(|_| {
        problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_bad_json",
            "site manifest object was not valid JSON",
            true,
            "site_manifest_bad_json",
        )
    })
}

fn site_visit_payment_response(
    status: StatusCode,
    ctx: &SiteVisitContext,
    payer_account: &str,
    amount_minor: &str,
    nonce: u64,
    idempotency_key: &str,
    wallet_receipt: Option<Value>,
) -> Response {
    let txid = wallet_receipt
        .as_ref()
        .and_then(|value| value_string(value, "txid"));
    let receipt_hash = wallet_receipt
        .as_ref()
        .and_then(|value| value_string(value, "receipt_hash"));

    let response = json!({
        "schema": SITE_VISIT_PAYMENT_SCHEMA,
        "ok": true,
        "site_name": &ctx.site_name,
        "crab_url": format!("crab://{}", ctx.site_name),
        "action": "site_visit",
        "asset": DEFAULT_ASSET,
        "amount_minor": amount_minor,
        "payer_account": payer_account,
        "visitor_wallet_account": payer_account,
        "recipient_account": &ctx.payout_recipient_account,
        "nonce": nonce,
        "txid": &txid,
        "receipt_hash": &receipt_hash,
        "wallet_receipt": &wallet_receipt,
        "payment": {
            "schema": SITE_VISIT_PAYMENT_SCHEMA,
            "site_name": &ctx.site_name,
            "crab_url": format!("crab://{}", ctx.site_name),
            "action": "site_visit",
            "asset": DEFAULT_ASSET,
            "amount_minor": amount_minor,
            "payer_account": payer_account,
            "recipient_account": &ctx.payout_recipient_account,
            "nonce": nonce,
            "txid": &txid,
            "receipt_hash": &receipt_hash,
            "status": "paid",
            "wallet_receipt": &wallet_receipt
        },
        "receipt": {
            "kind": "site_visit",
            "wallet_txid": &txid,
            "wallet_receipt_hash": &receipt_hash,
            "idempotency_key": idempotency_key,
            "manifest_cid": &ctx.manifest_cid,
            "root_document_cid": &ctx.root_document_cid,
            "paid_at_ms": now_ms()
        }
    });

    (status, Json(response)).into_response()
}

async fn send_wallet_site_visit_transfer(
    headers: &HeaderMap,
    payer_account: &str,
    recipient_account: &str,
    amount_minor: &str,
    nonce: u64,
    idempotency_key: &str,
    site_name: &str,
) -> Result<UpstreamBody, Response> {
    let url = format!("{}/v1/transfer", wallet_base_url());
    let body = json!({
        "from": payer_account,
        "to": recipient_account,
        "asset": DEFAULT_ASSET,
        "amount_minor": amount_minor,
        "nonce": nonce,
        "idempotency_key": idempotency_key,
        "memo": format!("crablink site_visit crab://{site_name}"),
    });

    let mut builder = HTTP_CLIENT
        .post(url)
        .bearer_auth(wallet_bearer())
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .header("idempotency-key", idempotency_key)
        .json(&body);

    if let Some(correlation_id) = grab(headers, "x-correlation-id") {
        builder = builder.header("x-correlation-id", correlation_id);
    }

    if let Some(request_id) = grab(headers, "x-request-id") {
        builder = builder.header("x-request-id", request_id);
    }

    let upstream_res = match builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "wallet_transfer_unavailable",
                "svc-wallet transfer route is unavailable",
                true,
                "wallet_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "wallet_transfer_read_failed",
                "failed to read svc-wallet transfer response",
                true,
                "wallet_read",
            ));
        }
    };

    Ok(UpstreamBody { status, body })
}

fn wallet_transfer_problem(status: StatusCode, body: Bytes) -> Response {
    let retryable = status.as_u16() >= 500 || status == StatusCode::TOO_MANY_REQUESTS;
    let wallet_error = serde_json::from_slice::<Value>(&body).unwrap_or_else(|_| {
        json!({
            "message": String::from_utf8_lossy(&body).to_string()
        })
    });

    (
        status,
        Json(json!({
            "code": "wallet_site_visit_transfer_rejected",
            "message": "svc-wallet rejected site visit transfer",
            "retryable": retryable,
            "reason": "wallet_transfer_rejected",
            "wallet_status": status.as_u16(),
            "wallet_error": wallet_error
        })),
    )
        .into_response()
}

fn site_visit_payer_account(
    headers: &HeaderMap,
    payer_account: Option<&str>,
    visitor_wallet_account: Option<&str>,
) -> Option<String> {
    clean_optional(payer_account)
        .or_else(|| clean_optional(visitor_wallet_account))
        .or_else(|| grab(headers, "x-ron-wallet-account"))
}

fn site_visit_price_minor() -> String {
    env::var("OMNIGATE_SITE_VISIT_PRICE_MINOR")
        .ok()
        .and_then(|value| normalize_minor_units_owned(&value).ok())
        .filter(|value| value != "0")
        .unwrap_or_else(|| DEFAULT_SITE_VISIT_PRICE_MINOR.to_owned())
}

fn default_site_visit_nonce() -> u64 {
    env::var("OMNIGATE_SITE_VISIT_NONCE")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|nonce| *nonce >= DEFAULT_WALLET_NONCE)
        .unwrap_or(DEFAULT_WALLET_NONCE)
}

fn wallet_base_url() -> String {
    env::var("OMNIGATE_WALLET_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BASE_URL.to_owned())
}

fn wallet_bearer() -> String {
    env::var("OMNIGATE_WALLET_BEARER")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BEARER.to_owned())
}

fn storage_base_url() -> String {
    env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_owned())
}

fn index_base_url() -> String {
    env::var("OMNIGATE_INDEX_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_INDEX_BASE_URL.to_owned())
}

fn site_visit_quote_id(
    site_name: &str,
    payer_account: &str,
    recipient_account: &str,
    amount_minor: &str,
) -> String {
    let seed = format!("site_visit:{site_name}:{payer_account}:{recipient_account}:{amount_minor}");
    format!("site-visit-{}", simple_hex_hash(&seed))
}

fn site_visit_idempotency_key(
    scope: &str,
    site_name: &str,
    payer_account: &str,
    recipient_account: &str,
) -> String {
    let seed = format!("site_visit:{scope}:{site_name}:{payer_account}:{recipient_account}");
    format!("site-visit-{scope}-{}", simple_hex_hash(&seed))
}

fn simple_hex_hash(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn normalize_site_name(input: &str) -> Result<String, &'static str> {
    if input.chars().any(char::is_control) {
        return Err("control_character");
    }

    let name = input.trim().to_ascii_lowercase();

    if name.is_empty() {
        return Err("empty_name");
    }
    if name == "." || name == ".." {
        return Err("dot_name");
    }
    if name.contains("..") {
        return Err("double_dot");
    }
    if name.starts_with('.') || name.ends_with('.') {
        return Err("edge_dot");
    }
    if name.contains('/') || name.contains('\\') || name.contains('@') || name.contains(' ') {
        return Err("unsafe_character");
    }
    if name.len() > 253 {
        return Err("name_too_long");
    }
    if !name
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
    {
        return Err("unsupported_character");
    }

    Ok(name)
}

fn is_canonical_b3_cid(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("b3:") else {
        return false;
    };

    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn normalize_minor_units_owned(value: &str) -> Result<String, &'static str> {
    let value = value.trim();
    if value.is_empty() || !value.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err("not_integer");
    }
    let trimmed = value.trim_start_matches('0');
    if trimmed.is_empty() {
        return Err("zero_amount");
    }
    Ok(trimmed.to_owned())
}

fn decimal_gt(left: &str, right: &str) -> bool {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    left.len() > right.len() || (left.len() == right.len() && left > right)
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_expected_nonce(message: &str) -> Option<u64> {
    let marker = "expected ";
    let start = message.find(marker)? + marker.len();
    let rest = &message[start..];
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse::<u64>().ok()
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

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 1;
    };

    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> Response {
    (
        status,
        Json(Problem {
            code,
            message,
            retryable,
            reason,
        }),
    )
        .into_response()
}
