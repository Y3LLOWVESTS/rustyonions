//! RO:WHAT — CrabLink wallet display route backed by svc-wallet when available.
//! RO:WHY — Browser extension needs a stable balance DTO while preserving svc-wallet as mutation/read front-door.
//! RO:INTERACTS — `svc-gateway` `/wallet/:account/balance`, `svc-wallet` `/v1/balance`.
//! RO:INVARIANTS — no wallet mutation here; no ledger mutation here; ledger-backed only when svc-wallet succeeds.
//! RO:METRICS — route is covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_WALLET_BASE_URL`, `OMNIGATE_WALLET_BEARER`.
//! RO:SECURITY — account label only; no capability or token is serialized.
//! RO:TEST — gateway proxy: `tests/identity_routes_proxy.rs`; smoke via CrabLink gateway script.

use axum::{extract::Path, Json};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
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

/// Return wallet balance display for CrabLink.
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

/// Balance DTO consumed by CrabLink.
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

async fn fetch_wallet_balance(account: &str) -> Result<WalletBalanceResponse, String> {
    if account.trim().is_empty() {
        return Err("empty_wallet_account".to_owned());
    }

    let url = format!(
        "{}/v1/balance?account={}&asset=roc",
        wallet_base_url().trim_end_matches('/'),
        percent_encode_component(account)
    );

    let response = HTTP_CLIENT
        .get(url)
        .timeout(Duration::from_secs(5))
        .header("Authorization", format!("Bearer {}", wallet_bearer()))
        .send()
        .await
        .map_err(|err| format!("svc_wallet_unreachable:{err}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("svc_wallet_body_read_failed:{err}"))?;

    if !status.is_success() {
        return Err(format!("svc_wallet_http_{}:{text}", status.as_u16()));
    }

    let value: Value =
        serde_json::from_str(&text).map_err(|err| format!("svc_wallet_json_failed:{err}"))?;

    let account_from_wallet = string_field(&value, "account").unwrap_or_else(|| account.to_owned());
    let amount_minor = amount_minor_from_value(value.get("amount_minor"))
        .ok_or_else(|| "svc_wallet_balance_missing_amount_minor".to_owned())?;

    let as_of = value
        .get("as_of_height")
        .and_then(Value::as_u64)
        .map(|height| height.to_string());

    Ok(WalletBalanceResponse {
        schema: WALLET_BALANCE_SCHEMA,
        account: account_from_wallet,
        unit: "ROC",
        available_minor_units: amount_minor.clone(),
        held_minor_units: "0".to_owned(),
        display: format!("{amount_minor} ROC"),
        as_of,
        ledger_backed: true,
        source: "svc_wallet.v1",
        reason: None,
        warnings: Vec::new(),
    })
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

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn amount_minor_from_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => normalize_unsigned_decimal(value),
        Value::Number(value) => normalize_unsigned_decimal(&value.to_string()),
        _ => None,
    }
}

fn normalize_unsigned_decimal(value: &str) -> Option<String> {
    let trimmed = value.trim();

    if trimmed.is_empty() || !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let normalized = trimmed.trim_start_matches('0');

    if normalized.is_empty() {
        Some("0".to_owned())
    } else {
        Some(normalized.to_owned())
    }
}

fn percent_encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_amount_normalization_is_integer_only() {
        assert_eq!(
            normalize_unsigned_decimal("001776").as_deref(),
            Some("1776")
        );
        assert_eq!(normalize_unsigned_decimal("0").as_deref(), Some("0"));
        assert!(normalize_unsigned_decimal("17.76").is_none());
    }

    #[test]
    fn percent_encoding_keeps_safe_account_chars() {
        assert_eq!(percent_encode_component("acct_dev"), "acct_dev");
        assert_eq!(percent_encode_component("acct:dev"), "acct%3Adev");
    }
}
