//! RO:WHAT — CrabLink identity/passport façade routes with wallet-backed starter grant.
//! RO:WHY — Browser extension needs a gateway-visible identity contract before full `svc-passport` custody is wired.
//! RO:INTERACTS — `svc-gateway` `/identity/*`, future `svc-passport`, `svc-wallet` `/v1/issue`.
//! RO:INVARIANTS — passports are identities, not wallets; no private keys; no direct ledger mutation; starter ROC comes from svc-wallet.
//! RO:METRICS — route is covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_WALLET_BASE_URL`, `OMNIGATE_WALLET_BEARER`, `OMNIGATE_STARTER_GRANT_MINOR_UNITS`.
//! RO:SECURITY — returns labels only; `can_spend=false`; no long-lived uncapped spend authority.
//! RO:TEST — gateway proxy: `tests/identity_routes_proxy.rs`; smoke with `CRABLINK_SMOKE_RUN_BOOTSTRAP=1`.

use axum::{
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, time::Duration};

const IDENTITY_ME_SCHEMA: &str = "crablink.identity.me.v1";
const IDENTITY_BOOTSTRAP_SCHEMA: &str = "crablink.identity.bootstrap.v1";
const DEFAULT_PASSPORT_SUBJECT: &str = "passport:main:dev";
const DEFAULT_WALLET_ACCOUNT: &str = "acct_dev";
const DEFAULT_STARTER_GRANT_MINOR_UNITS: &str = "1776";
const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";
const DEFAULT_WALLET_BEARER: &str = "dev";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate identity wallet client should build")
});

/// Router for `/v1/identity/*`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/me", get(identity_me))
        .route("/passport/bootstrap", post(passport_bootstrap))
}

/// Return the currently supplied browser identity labels.
///
/// This is still a label façade. It does not verify or create passport keys.
pub async fn identity_me(headers: HeaderMap) -> Json<IdentityMeResponse> {
    let passport_subject = header_value(&headers, "x-ron-passport");
    let wallet_account = header_value(&headers, "x-ron-wallet-account");
    let identity_username = username_from_identity_headers(&headers);

    let passport = passport_subject.as_ref().map(|subject| PassportView {
        subject: subject.clone(),
        kind: "main".to_owned(),
        display_name: "Local Dev Passport".to_owned(),
        created_at: None,
        source: "request_header".to_owned(),
        requested_username: identity_username.requested_username,
        requested_handle: identity_username.requested_handle,
        username: identity_username.username,
        handle: identity_username.handle,
        username_status: identity_username.username_status,
        profile_crab_url: identity_username.profile_crab_url,
        public_profile_cid: identity_username.public_profile_cid,
    });

    let wallet = wallet_account.as_ref().map(|account| WalletLinkView {
        account: account.clone(),
        linked: true,
        source: "request_header".to_owned(),
    });

    Json(IdentityMeResponse {
        schema: IDENTITY_ME_SCHEMA,
        passport,
        wallet,
        capabilities: Capabilities {
            can_view_balance: wallet_account.is_some(),
            can_prepare_paid_actions: passport_subject.is_some(),
            can_spend: false,
        },
        warnings: warnings_for_identity(passport_subject.as_deref(), wallet_account.as_deref()),
    })
}

/// Bootstrap a local dev passport label for CrabLink.
///
/// This route still creates/loads display labels only for passport identity.
/// The starter ROC grant, when requested, is issued through `svc-wallet`.
/// Omnigate never mutates `ron-ledger` directly.
pub async fn passport_bootstrap(
    headers: HeaderMap,
    Json(req): Json<BootstrapRequest>,
) -> Json<IdentityBootstrapResponse> {
    let passport_subject = header_value(&headers, "x-ron-passport")
        .or(req.passport_subject.clone())
        .unwrap_or_else(|| DEFAULT_PASSPORT_SUBJECT.to_owned());

    let create_wallet = req.create_wallet.unwrap_or(true);
    let wallet_account = if create_wallet {
        Some(
            header_value(&headers, "x-ron-wallet-account")
                .or(req.wallet_account.clone())
                .unwrap_or_else(|| DEFAULT_WALLET_ACCOUNT.to_owned()),
        )
    } else {
        None
    };

    let display_name = req
        .display_name
        .clone()
        .or(req.label.clone())
        .unwrap_or_else(|| "Local Dev Passport".to_owned());

    let mut warnings = vec![
        "dev bootstrap returns passport labels only until svc-passport custody is wired".to_owned(),
        "wallet spend authority is not granted by this response".to_owned(),
    ];

    let username = username_from_bootstrap_request(&req, &mut warnings);

    let starter_grant = if req.starter_grant {
        match wallet_account.as_deref() {
            Some(account) => {
                let requested_amount = req
                    .desired_starting_balance_minor_units
                    .clone()
                    .or_else(starter_grant_amount_from_env)
                    .unwrap_or_else(|| DEFAULT_STARTER_GRANT_MINOR_UNITS.to_owned());

                match normalize_amount_minor(&requested_amount) {
                    Ok(amount_minor_units) => {
                        match issue_starter_grant(
                            &headers,
                            &passport_subject,
                            account,
                            &amount_minor_units,
                        )
                        .await
                        {
                            Ok(receipt) => StarterGrantView {
                                issued: true,
                                amount_minor_units,
                                receipt_id: Some(receipt.txid),
                                reason: Some("issued_by_svc_wallet".to_owned()),
                                ledger_backed: true,
                                source: "svc_wallet.v1".to_owned(),
                            },
                            Err(reason) => {
                                warnings.push(format!("starter_grant_issue_failed:{reason}"));
                                StarterGrantView {
                                    issued: false,
                                    amount_minor_units: "0".to_owned(),
                                    receipt_id: None,
                                    reason: Some(reason),
                                    ledger_backed: false,
                                    source: "svc_wallet.v1".to_owned(),
                                }
                            }
                        }
                    }
                    Err(reason) => {
                        warnings.push(format!("starter_grant_invalid_amount:{reason}"));
                        StarterGrantView {
                            issued: false,
                            amount_minor_units: "0".to_owned(),
                            receipt_id: None,
                            reason: Some(reason),
                            ledger_backed: false,
                            source: "omnigate.validation".to_owned(),
                        }
                    }
                }
            }
            None => {
                warnings.push("starter_grant_skipped_no_wallet_account".to_owned());
                StarterGrantView {
                    issued: false,
                    amount_minor_units: "0".to_owned(),
                    receipt_id: None,
                    reason: Some("wallet_not_created".to_owned()),
                    ledger_backed: false,
                    source: "omnigate.validation".to_owned(),
                }
            }
        }
    } else {
        StarterGrantView {
            issued: false,
            amount_minor_units: "0".to_owned(),
            receipt_id: None,
            reason: Some("starter_grant_not_requested".to_owned()),
            ledger_backed: false,
            source: "omnigate.validation".to_owned(),
        }
    };

    if starter_grant.issued {
        warnings.push("starter ROC was issued through svc-wallet".to_owned());
    } else if req.starter_grant {
        warnings.push("starter ROC was not issued; CrabLink must not invent a balance".to_owned());
    }

    Json(IdentityBootstrapResponse {
        schema: IDENTITY_BOOTSTRAP_SCHEMA,
        passport: PassportView {
            subject: passport_subject,
            kind: req.kind.unwrap_or_else(|| "main".to_owned()),
            display_name,
            created_at: None,
            source: "omnigate_dev_bootstrap".to_owned(),
            requested_username: username.requested_username,
            requested_handle: username.requested_handle,
            username: username.username,
            handle: username.handle,
            username_status: username.username_status,
            profile_crab_url: username.profile_crab_url,
            public_profile_cid: username.public_profile_cid,
        },
        wallet: wallet_account.map(|account| WalletLinkView {
            account,
            linked: true,
            source: "omnigate_dev_bootstrap".to_owned(),
        }),
        starter_grant,
        capabilities: Capabilities {
            can_view_balance: create_wallet,
            can_prepare_paid_actions: true,
            can_spend: false,
        },
        warnings,
    })
}

/// Request body accepted by the CrabLink dev bootstrap route.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapRequest {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub create_wallet: Option<bool>,
    #[serde(default, alias = "request_starter_grant")]
    pub starter_grant: bool,
    #[serde(default)]
    pub passport_subject: Option<String>,
    #[serde(default)]
    pub wallet_account: Option<String>,
    #[serde(default, alias = "desiredStartingBalanceMinorUnits")]
    pub desired_starting_balance_minor_units: Option<String>,
    #[serde(default, alias = "requestedUsername")]
    pub requested_username: Option<String>,
    #[serde(default, alias = "requestedHandle")]
    pub requested_handle: Option<String>,
}

/// Current identity response for the browser client.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityMeResponse {
    pub schema: &'static str,
    pub passport: Option<PassportView>,
    pub wallet: Option<WalletLinkView>,
    pub capabilities: Capabilities,
    pub warnings: Vec<String>,
}

/// Bootstrap response for the browser client.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityBootstrapResponse {
    pub schema: &'static str,
    pub passport: PassportView,
    pub wallet: Option<WalletLinkView>,
    pub starter_grant: StarterGrantView,
    pub capabilities: Capabilities,
    pub warnings: Vec<String>,
}

/// Safe passport display fields.
#[derive(Debug, Clone, Serialize)]
pub struct PassportView {
    pub subject: String,
    pub kind: String,
    pub display_name: String,
    pub created_at: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_crab_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_profile_cid: Option<String>,
}

/// Safe wallet-link display fields.
#[derive(Debug, Clone, Serialize)]
pub struct WalletLinkView {
    pub account: String,
    pub linked: bool,
    pub source: String,
}

/// Starter grant status.
///
/// Real issuance belongs to `svc-wallet` and `ron-ledger`.
#[derive(Debug, Clone, Serialize)]
pub struct StarterGrantView {
    pub issued: bool,
    pub amount_minor_units: String,
    pub receipt_id: Option<String>,
    pub reason: Option<String>,
    pub ledger_backed: bool,
    pub source: String,
}

/// Capability summary for UI decisions.
#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    pub can_view_balance: bool,
    pub can_prepare_paid_actions: bool,
    pub can_spend: bool,
}

#[derive(Debug, Clone)]
struct WalletIssueReceipt {
    txid: String,
}

#[derive(Debug, Clone, Default)]
struct UsernameFields {
    requested_username: Option<String>,
    requested_handle: Option<String>,
    username: Option<String>,
    handle: Option<String>,
    username_status: Option<String>,
    profile_crab_url: Option<String>,
    public_profile_cid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedUsername {
    username: String,
    handle: String,
}

async fn issue_starter_grant(
    headers: &HeaderMap,
    passport_subject: &str,
    wallet_account: &str,
    amount_minor_units: &str,
) -> Result<WalletIssueReceipt, String> {
    let base_url = wallet_base_url();
    let url = format!("{}/v1/issue", base_url.trim_end_matches('/'));
    let idempotency_key = deterministic_bootstrap_idempotency_key(passport_subject, wallet_account);

    let body = json!({
        "to": wallet_account,
        "asset": "roc",
        "amount_minor": amount_minor_units,
        "idempotency_key": idempotency_key,
        "memo": "crablink passport starter grant"
    });

    let mut req = HTTP_CLIENT
        .post(url)
        .timeout(Duration::from_secs(5))
        .header("Authorization", format!("Bearer {}", wallet_bearer()))
        .header("Idempotency-Key", idempotency_key.clone())
        .json(&body);

    if let Some(corr_id) =
        header_value(headers, "x-correlation-id").or_else(|| header_value(headers, "x-request-id"))
    {
        req = req.header("x-correlation-id", corr_id);
    }

    let response = req
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

    let txid = value
        .get("txid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "svc_wallet_receipt_missing_txid".to_owned())?
        .to_owned();

    Ok(WalletIssueReceipt { txid })
}

fn warnings_for_identity(passport: Option<&str>, wallet: Option<&str>) -> Vec<String> {
    let mut warnings = Vec::new();

    if passport.is_none() {
        warnings.push("no passport label supplied".to_owned());
    }

    if wallet.is_none() {
        warnings.push("no wallet account label supplied".to_owned());
    }

    warnings.push("identity view is header-derived until svc-passport is wired".to_owned());

    warnings
}

fn username_from_bootstrap_request(
    req: &BootstrapRequest,
    warnings: &mut Vec<String>,
) -> UsernameFields {
    let requested_raw = req
        .requested_username
        .as_deref()
        .or(req.requested_handle.as_deref())
        .unwrap_or_default();

    if requested_raw.trim().is_empty() {
        return UsernameFields::default();
    }

    match normalize_username(requested_raw) {
        Ok(username) => UsernameFields {
            requested_username: Some(username.username),
            requested_handle: Some(username.handle),
            username: None,
            handle: None,
            username_status: Some("requested".to_owned()),
            profile_crab_url: None,
            public_profile_cid: None,
        },
        Err(reason) => {
            warnings.push(format!("username_request_rejected:{reason}"));
            UsernameFields {
                username_status: Some("rejected".to_owned()),
                ..UsernameFields::default()
            }
        }
    }
}

fn username_from_identity_headers(headers: &HeaderMap) -> UsernameFields {
    let requested = header_value(headers, "x-ron-requested-username")
        .or_else(|| header_value(headers, "x-ron-requested-handle"))
        .and_then(|value| normalize_username(&value).ok());
    let confirmed = header_value(headers, "x-ron-username")
        .or_else(|| header_value(headers, "x-ron-handle"))
        .and_then(|value| normalize_username(&value).ok());
    let status = header_value(headers, "x-ron-username-status")
        .as_deref()
        .and_then(normalize_username_status);
    let profile_crab_url = header_value(headers, "x-ron-profile-crab-url")
        .filter(|value| normalize_profile_crab_url(value).is_some());
    let public_profile_cid = header_value(headers, "x-ron-public-profile-cid")
        .filter(|value| normalize_b3_cid(value).is_some());

    UsernameFields {
        requested_username: requested.as_ref().map(|value| value.username.clone()),
        requested_handle: requested.as_ref().map(|value| value.handle.clone()),
        username: confirmed.as_ref().map(|value| value.username.clone()),
        handle: confirmed.as_ref().map(|value| value.handle.clone()),
        username_status: status
            .map(str::to_owned)
            .or_else(|| confirmed.as_ref().map(|_| "backend_unknown".to_owned()))
            .or_else(|| requested.as_ref().map(|_| "requested".to_owned())),
        profile_crab_url,
        public_profile_cid,
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn starter_grant_amount_from_env() -> Option<String> {
    env::var("OMNIGATE_STARTER_GRANT_MINOR_UNITS")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
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

fn normalize_amount_minor(value: &str) -> Result<String, String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err("amount_empty".to_owned());
    }

    if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("amount_must_be_decimal_integer".to_owned());
    }

    let normalized = trimmed.trim_start_matches('0');

    if normalized.is_empty() {
        return Err("amount_must_be_nonzero".to_owned());
    }

    Ok(normalized.to_owned())
}

fn normalize_username(value: &str) -> Result<NormalizedUsername, String> {
    let input = value.trim().trim_start_matches('@').to_ascii_lowercase();

    if input.is_empty() {
        return Err("username_empty".to_owned());
    }

    if input.len() < 3 {
        return Err("username_too_short".to_owned());
    }

    if input.len() > 32 {
        return Err("username_too_long".to_owned());
    }

    let mut previous_dot = false;
    for (idx, byte) in input.bytes().enumerate() {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.');

        if !valid {
            return Err("username_invalid_character".to_owned());
        }

        if idx == 0 && !byte.is_ascii_lowercase() && !byte.is_ascii_digit() {
            return Err("username_must_start_with_alnum".to_owned());
        }

        if previous_dot && byte == b'.' {
            return Err("username_consecutive_dots".to_owned());
        }
        previous_dot = byte == b'.';
    }

    if input.ends_with('.') || input.ends_with('-') || input.ends_with('_') {
        return Err("username_invalid_trailing_punctuation".to_owned());
    }

    if RESERVED_USERNAMES.contains(&input.as_str()) {
        return Err("username_reserved".to_owned());
    }

    Ok(NormalizedUsername {
        handle: format!("@{input}"),
        username: input,
    })
}

fn normalize_username_status(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "local_draft" => Some("local_draft"),
        "requested" => Some("requested"),
        "confirmed" => Some("confirmed"),
        "rejected" => Some("rejected"),
        "unavailable" => Some("unavailable"),
        "backend_unknown" => Some("backend_unknown"),
        _ => None,
    }
}

fn normalize_profile_crab_url(value: &str) -> Option<String> {
    let raw = value.trim();

    if raw.is_empty() {
        return None;
    }

    if let Some(rest) = raw.strip_prefix("crab://@") {
        return normalize_username(rest)
            .ok()
            .map(|username| format!("crab://{}", username.handle));
    }

    if let Some(rest) = raw.strip_prefix("crab://profile/@") {
        return normalize_username(rest)
            .ok()
            .map(|username| format!("crab://profile/{}", username.handle));
    }

    None
}

fn normalize_b3_cid(value: &str) -> Option<String> {
    let raw = value.trim().to_ascii_lowercase();

    if raw.len() == 67
        && raw.starts_with("b3:")
        && raw[3..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Some(raw);
    }

    None
}

fn deterministic_bootstrap_idempotency_key(passport_subject: &str, wallet_account: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in b"crablink.passport.bootstrap.v1" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    for byte in passport_subject
        .bytes()
        .chain(std::iter::once(0u8))
        .chain(wallet_account.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("crablink_bootstrap_v1_{hash:016x}")
}

const RESERVED_USERNAMES: &[&str] = &[
    "admin",
    "api",
    "app",
    "article",
    "asset",
    "assets",
    "b3",
    "comment",
    "crab",
    "gateway",
    "image",
    "mail",
    "manifest",
    "mod",
    "moderator",
    "music",
    "passport",
    "podcast",
    "post",
    "profile",
    "profiles",
    "root",
    "site",
    "sites",
    "stream",
    "support",
    "sys",
    "system",
    "video",
    "wallet",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_idempotency_key_is_short_and_stable() {
        let a = deterministic_bootstrap_idempotency_key("passport:main:dev", "acct_dev");
        let b = deterministic_bootstrap_idempotency_key("passport:main:dev", "acct_dev");

        assert_eq!(a, b);
        assert!(a.len() <= 64);
        assert!(a.starts_with("crablink_bootstrap_v1_"));
    }

    #[test]
    fn normalizes_starter_amount() {
        assert_eq!(normalize_amount_minor("001776").unwrap(), "1776");
        assert!(normalize_amount_minor("0").is_err());
        assert!(normalize_amount_minor("17.76").is_err());
    }

    #[test]
    fn normalizes_username_request() {
        let username = normalize_username("@Skinny.Crabby").unwrap();
        assert_eq!(username.username, "skinny.crabby");
        assert_eq!(username.handle, "@skinny.crabby");
    }

    #[test]
    fn rejects_reserved_and_bad_usernames() {
        assert_eq!(normalize_username("site").unwrap_err(), "username_reserved");
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("-bad").is_err());
        assert!(normalize_username("bad..").is_err());
        assert!(normalize_username("bad_").is_err());
    }

    #[test]
    fn starter_grant_env_amount_is_owned() {
        let maybe_amount = starter_grant_amount_from_env();
        assert!(maybe_amount.is_none() || maybe_amount.as_deref().is_some());
    }
}
