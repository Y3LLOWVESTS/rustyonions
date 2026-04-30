//! RO:WHAT — Dev-safe CrabLink identity/passport façade routes.
//! RO:WHY — Browser extension needs a gateway-visible identity contract before full `svc-passport` custody is wired.
//! RO:INTERACTS — `svc-gateway` `/identity/*`, future `svc-passport`, future `svc-wallet`.
//! RO:INVARIANTS — passports are identities, not wallets; no private keys; no wallet/ledger mutation; starter grant is not faked.
//! RO:METRICS — route is covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — currently header/body driven; future service URLs should come from config.
//! RO:SECURITY — returns labels only; `can_spend=false`; no long-lived uncapped spend authority.
//! RO:TEST — gateway proxy: `tests/identity_routes_proxy.rs`; route can be smoke-tested with CrabLink.

use axum::{
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

const IDENTITY_ME_SCHEMA: &str = "crablink.identity.me.v1";
const IDENTITY_BOOTSTRAP_SCHEMA: &str = "crablink.identity.bootstrap.v1";
const DEFAULT_PASSPORT_SUBJECT: &str = "passport:main:dev";
const DEFAULT_WALLET_ACCOUNT: &str = "acct_dev";

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
/// This is a dev-safe façade. It does not verify or create passport keys.
pub async fn identity_me(headers: HeaderMap) -> Json<IdentityMeResponse> {
    let passport_subject = header_value(&headers, "x-ron-passport");
    let wallet_account = header_value(&headers, "x-ron-wallet-account");

    let passport = passport_subject.as_ref().map(|subject| PassportView {
        subject: subject.clone(),
        kind: "main".to_owned(),
        display_name: "Local Dev Passport".to_owned(),
        created_at: None,
        source: "request_header".to_owned(),
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
/// This returns labels only. It does not create private keys, does not issue ROC,
/// and does not call `svc-wallet`.
pub async fn passport_bootstrap(
    headers: HeaderMap,
    Json(req): Json<BootstrapRequest>,
) -> Json<IdentityBootstrapResponse> {
    let passport_subject = header_value(&headers, "x-ron-passport")
        .or(req.passport_subject)
        .unwrap_or_else(|| DEFAULT_PASSPORT_SUBJECT.to_owned());

    let create_wallet = req.create_wallet.unwrap_or(true);
    let wallet_account = if create_wallet {
        Some(
            header_value(&headers, "x-ron-wallet-account")
                .or(req.wallet_account)
                .unwrap_or_else(|| DEFAULT_WALLET_ACCOUNT.to_owned()),
        )
    } else {
        None
    };

    let display_name = req
        .display_name
        .or(req.label)
        .unwrap_or_else(|| "Local Dev Passport".to_owned());

    Json(IdentityBootstrapResponse {
        schema: IDENTITY_BOOTSTRAP_SCHEMA,
        passport: PassportView {
            subject: passport_subject,
            kind: req.kind.unwrap_or_else(|| "main".to_owned()),
            display_name,
            created_at: None,
            source: "omnigate_dev_bootstrap".to_owned(),
        },
        wallet: wallet_account.map(|account| WalletLinkView {
            account,
            linked: true,
            source: "omnigate_dev_bootstrap".to_owned(),
        }),
        starter_grant: StarterGrantView {
            issued: false,
            amount_minor_units: "0".to_owned(),
            receipt_id: None,
            reason: Some("svc_wallet_integration_pending".to_owned()),
        },
        capabilities: Capabilities {
            can_view_balance: create_wallet,
            can_prepare_paid_actions: true,
            can_spend: false,
        },
        warnings: vec![
            "dev bootstrap returns labels only".to_owned(),
            "starter ROC is not issued until svc-wallet integration is wired".to_owned(),
            "wallet spend authority is not granted by this response".to_owned(),
        ],
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
/// A dev bootstrap route must not fake issuance. Real issuance belongs to
/// `svc-wallet` and `ron-ledger`.
#[derive(Debug, Clone, Serialize)]
pub struct StarterGrantView {
    pub issued: bool,
    pub amount_minor_units: String,
    pub receipt_id: Option<String>,
    pub reason: Option<String>,
}

/// Capability summary for UI decisions.
#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    pub can_view_balance: bool,
    pub can_prepare_paid_actions: bool,
    pub can_spend: bool,
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

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
