//! RO:WHAT — CrabLink wallet display façade route.
//! RO:WHY — Browser extension needs a stable balance DTO while real `svc-wallet` proxying is wired.
//! RO:INTERACTS — `svc-gateway` `/wallet/:account/balance`, future `svc-wallet` `/v1/balance`.
//! RO:INVARIANTS — display-only; no wallet mutation; no ledger mutation; no fake positive balances.
//! RO:METRICS — route is covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — currently no downstream URL; future wallet URL should come from config.
//! RO:SECURITY — account label only; no capability or token is serialized.
//! RO:TEST — gateway proxy: `tests/identity_routes_proxy.rs`.

use axum::{extract::Path, Json};
use serde::Serialize;

const WALLET_BALANCE_SCHEMA: &str = "crablink.wallet.balance.v1";

/// Return a truthful dev wallet display response.
///
/// This is intentionally not ledger-backed yet. It returns zero and marks the
/// response as non-authoritative until the `svc-wallet` integration is wired.
pub async fn balance(Path(account): Path<String>) -> Json<WalletBalanceResponse> {
    Json(WalletBalanceResponse {
        schema: WALLET_BALANCE_SCHEMA,
        account,
        unit: "ROC",
        available_minor_units: "0".to_owned(),
        held_minor_units: "0".to_owned(),
        display: "0 ROC".to_owned(),
        as_of: None,
        ledger_backed: false,
        source: "omnigate_dev_wallet_view.v1",
        reason: Some("svc_wallet_integration_pending".to_owned()),
        warnings: vec![
            "display-only dev placeholder".to_owned(),
            "real balance must come from svc-wallet and ron-ledger".to_owned(),
            "do not treat this route as spend authority".to_owned(),
        ],
    })
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
