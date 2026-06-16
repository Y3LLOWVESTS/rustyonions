//! RO:QUICKCHAIN-PREFLIGHT — wallet facade delegates to svc-wallet; no direct ledger mutation here.
//! RO:QUICKCHAIN-PREFLIGHT — ledger_backed: false for display-only dev fallback; real balance must come from svc-wallet and ron-ledger; do not treat this route as spend authority.
//! RO:WHAT — `CrabLink` wallet display and hold façade backed by `svc-wallet`.
//! RO:WHY — P12 Economics; Concerns: ECON/SEC/DX. Browser clients need stable wallet read/hold routes without direct `svc-wallet` topology.
//! RO:INTERACTS — `svc-gateway` `/wallet/*`, `svc-wallet` `/v1/balance` and `/v1/hold`, `CrabLink` prepare flows.
//! RO:INVARIANTS — `svc-wallet` remains mutation front-door; no ledger mutation here; integer minor units only; fail closed on bad hold input.
//! RO:METRICS — route is covered by `omnigate` HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_WALLET_BASE_URL`, `OMNIGATE_WALLET_BEARER`.
//! RO:SECURITY — backend bearer is not serialized; `x-ron-wallet-account` must match hold payer when present.
//! RO:TEST — gateway proxy tests plus live `CrabLink` manual wallet-hold smoke.

use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, time::Duration};

const WALLET_BALANCE_SCHEMA: &str = "crablink.wallet.balance.v1";
const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";
const DEFAULT_WALLET_BEARER: &str = "dev";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate wallet route reqwest client should build")
});

/// Return wallet balance display for `CrabLink`.
///
/// This route attempts to read through `svc-wallet`. If `svc-wallet` is not
/// available, it returns an explicit display-only placeholder with
/// `ledger_backed=false` instead of inventing a balance.
pub async fn balance(Path(account): Path<String>) -> Json<WalletBalanceResponse> {
    match fetch_wallet_balance(&account).await {
        Ok(view) => Json(view),
        Err(reason) => Json(WalletBalanceResponse {
            schema: WALLET_BALANCE_SCHEMA,
            account,
            unit: "ROC",
            available_minor_units: "0".to_owned(),
            held_minor_units: "0".to_owned(),
            display: "0 ROC".to_owned(),
            as_of: None,
            ledger_backed: false,
            source: "omnigate_dev_wallet_view.v1",
            reason: Some(reason.clone()),
            warnings: vec![
                "display-only dev fallback".to_owned(),
                "real balance must come from svc-wallet and ron-ledger".to_owned(),
                format!("svc_wallet_balance_unavailable:{reason}"),
                "do not treat this route as spend authority".to_owned(),
            ],
        }),
    }
}

/// Create a wallet hold through `svc-wallet`.
///
/// Public path through gateway:
///
/// ```text
/// POST /wallet/hold
/// ```
///
/// Omnigate path:
///
/// ```text
/// POST /v1/wallet/hold
/// ```
///
/// `svc-wallet` target:
///
/// ```text
/// POST /v1/hold
/// ```
///
/// Request body intentionally mirrors `svc-wallet` `TransferRequest` shape:
///
/// ```json
/// {
///   "from": "acct_dev",
///   "to": "escrow_paid_write",
///   "asset": "roc",
///   "amount_minor": "121927",
///   "nonce": 1,
///   "memo": "crablink image upload hold"
/// }
/// ```
pub async fn hold(headers: HeaderMap, Json(request): Json<WalletHoldRequest>) -> Response {
    let request = match normalize_hold_request(request) {
        Ok(request) => request,
        Err(problem) => return problem.into_response(),
    };

    if let Some(header_account) = header_value(&headers, "x-ron-wallet-account") {
        if header_account != request.from {
            return wallet_hold_problem(
                StatusCode::BAD_REQUEST,
                "wallet_account_mismatch",
                "x-ron-wallet-account must match wallet hold payer account",
                false,
                "payer_mismatch",
            )
            .into_response();
        }
    }

    let idem = header_value(&headers, "idempotency-key")
        .or_else(|| request.idempotency_key.clone())
        .unwrap_or_else(|| {
            format!(
                "crablink-hold:{}:{}:{}:{}",
                request.from, request.to, request.amount_minor, request.nonce
            )
        });

    let mut wallet_request = request.clone();
    wallet_request.idempotency_key = Some(idem.clone());

    let url = format!("{}/v1/hold", wallet_base_url());

    let mut builder = HTTP_CLIENT
        .post(url)
        .bearer_auth(wallet_bearer())
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .header("idempotency-key", idem);

    if let Some(correlation_id) = header_value(&headers, "x-correlation-id") {
        builder = builder.header("x-correlation-id", correlation_id);
    }

    if let Some(request_id) = header_value(&headers, "x-request-id") {
        builder = builder.header("x-request-id", request_id);
    }

    let response = match builder.json(&wallet_request).send().await {
        Ok(response) => response,
        Err(err) => {
            return problem(
                StatusCode::BAD_GATEWAY,
                "wallet_hold_unavailable",
                "svc-wallet hold route is unavailable",
                true,
                leak_safe_reason(&err.to_string()),
            )
        }
    };

    let status =
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body = match response.text().await {
        Ok(body) => body,
        Err(err) => {
            return problem(
                StatusCode::BAD_GATEWAY,
                "wallet_hold_read_failed",
                "failed to read svc-wallet hold response",
                true,
                leak_safe_reason(&err.to_string()),
            )
        }
    };

    let mut out = Response::new(axum::body::Body::from(body));
    *out.status_mut() = status;
    out.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json"
            .parse()
            .expect("static content-type should parse"),
    );
    out
}

/// Balance DTO consumed by `CrabLink`.
#[derive(Debug, Clone, Serialize)]
pub struct WalletBalanceResponse {
    pub schema: &'static str,
    pub account: String,
    pub unit: &'static str,
    pub available_minor_units: String,
    pub held_minor_units: String,
    pub display: String,
    pub as_of: Option<String>,
    pub ledger_backed: bool,
    pub source: &'static str,
    pub reason: Option<String>,
    pub warnings: Vec<String>,
}

/// Hold request accepted by `omnigate` and forwarded to `svc-wallet`.
///
/// This intentionally stays close to `svc-wallet::TransferRequest` so that
/// `omnigate` remains a façade/proxy instead of a second wallet implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletHoldRequest {
    pub from: String,
    pub to: String,
    pub asset: String,
    pub amount_minor: String,
    pub nonce: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct WalletHoldProblem {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
}

impl WalletHoldProblem {
    fn into_response(self) -> Response {
        problem(
            self.status,
            self.code,
            self.message,
            self.retryable,
            self.reason,
        )
    }
}

async fn fetch_wallet_balance(account: &str) -> Result<WalletBalanceResponse, String> {
    let url = format!(
        "{}/v1/balance?account={}&asset=roc",
        wallet_base_url(),
        percent_encode(account)
    );

    let response = HTTP_CLIENT
        .get(url)
        .bearer_auth(wallet_bearer())
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|err| leak_safe_reason(&err.to_string()).to_owned())?;

    if !response.status().is_success() {
        return Err(format!("wallet_http_{}", response.status().as_u16()));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|err| leak_safe_reason(&err.to_string()).to_owned())?;

    let wallet_account = value_string(&value, "account").unwrap_or_else(|| account.to_owned());

    let available_minor_units = value_string(&value, "amount_minor")
        .or_else(|| value_string(&value, "available_minor_units"))
        .ok_or_else(|| "wallet_missing_amount_minor".to_owned())
        .and_then(|value| {
            normalize_minor_units(&value)
                .map(str::to_owned)
                .map_err(ToOwned::to_owned)
        })?;

    let held_minor_units = value_string(&value, "held_minor_units")
        .map(|value| normalize_minor_units(&value).map(str::to_owned))
        .transpose()
        .map_err(ToOwned::to_owned)?
        .unwrap_or_else(|| "0".to_owned());

    let as_of = value_string(&value, "as_of").or_else(|| value_string(&value, "as_of_height"));

    Ok(WalletBalanceResponse {
        schema: WALLET_BALANCE_SCHEMA,
        account: wallet_account,
        unit: "ROC",
        available_minor_units: available_minor_units.clone(),
        held_minor_units,
        display: format_roc_display(&available_minor_units),
        as_of,
        ledger_backed: true,
        source: "svc_wallet.v1",
        reason: None,
        warnings: Vec::new(),
    })
}

fn normalize_hold_request(
    mut request: WalletHoldRequest,
) -> Result<WalletHoldRequest, WalletHoldProblem> {
    request.from = clean_required(request.from, "from")?;
    request.to = clean_required(request.to, "to")?;
    request.asset = clean_required(request.asset, "asset")?.to_ascii_lowercase();
    request.amount_minor = clean_required(request.amount_minor, "amount_minor")?;

    if request.asset != "roc" {
        return Err(wallet_hold_problem(
            StatusCode::BAD_REQUEST,
            "invalid_wallet_hold_asset",
            "wallet hold asset must be roc",
            false,
            "asset_must_be_roc",
        ));
    }

    let normalized_amount = match normalize_minor_units(&request.amount_minor) {
        Ok(value) => value.to_owned(),
        Err(_) => {
            return Err(wallet_hold_problem(
                StatusCode::BAD_REQUEST,
                "invalid_wallet_hold_amount",
                "wallet hold amount_minor must be a positive integer string",
                false,
                "bad_amount_minor",
            ))
        }
    };

    if normalized_amount == "0" {
        return Err(wallet_hold_problem(
            StatusCode::BAD_REQUEST,
            "invalid_wallet_hold_amount",
            "wallet hold amount_minor must be greater than zero",
            false,
            "zero_amount_minor",
        ));
    }

    request.amount_minor = normalized_amount;

    request.memo = request
        .memo
        .map(|memo| memo.trim().to_owned())
        .filter(|memo| !memo.is_empty());

    request.idempotency_key = request
        .idempotency_key
        .map(|idem| idem.trim().to_owned())
        .filter(|idem| !idem.is_empty());

    Ok(request)
}

fn clean_required(value: String, field: &'static str) -> Result<String, WalletHoldProblem> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err(wallet_hold_problem(
            StatusCode::BAD_REQUEST,
            "invalid_wallet_hold_request",
            "wallet hold request is missing a required text field",
            false,
            field,
        ));
    }

    Ok(value)
}

fn wallet_hold_problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> WalletHoldProblem {
    WalletHoldProblem {
        status,
        code,
        message,
        retryable,
        reason,
    }
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

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn normalize_minor_units(value: &str) -> Result<&str, &'static str> {
    let value = value.trim();

    if value.is_empty() {
        return Err("empty_integer");
    }

    if !value.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err("not_integer");
    }

    let trimmed = value.trim_start_matches('0');

    if trimmed.is_empty() {
        Ok("0")
    } else {
        Ok(trimmed)
    }
}

fn format_roc_display(value: &str) -> String {
    format!("{value} ROC")
}

fn percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());

    for byte in input.bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');

        if is_unreserved {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

fn leak_safe_reason(raw: &str) -> &'static str {
    let raw = raw.to_ascii_lowercase();

    if raw.contains("timeout") {
        "timeout"
    } else if raw.contains("connect") || raw.contains("connection") {
        "connect"
    } else if raw.contains("decode") || raw.contains("json") {
        "bad_json"
    } else {
        "wallet_upstream"
    }
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
        Json(json!({
            "code": code,
            "message": message,
            "retryable": retryable,
            "reason": reason
        })),
    )
        .into_response()
}
