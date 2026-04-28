//! RO:WHAT — Versioned v1 wallet API router.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/DX. Groups stable balance, issue, transfer, burn, escrow, and receipt endpoints.
//! RO:INTERACTS — routes/v1 handlers, dto request/response types, WalletState.
//! RO:INVARIANTS — all mutation endpoints are idempotent; all v1 endpoints require read/write caps as appropriate.
//! RO:METRICS — child handlers record successes/rejects/op counters.
//! RO:CONFIG — child handlers validate against WalletConfig.
//! RO:SECURITY — no ambient authority; Authorization required by handlers.
//! RO:TEST — router is constructed by routes::tests::dev_state_builds_router and HTTP black-box tests.

pub mod balance;
pub mod burn;
pub mod escrow;
pub mod issue;
pub mod receipt;
pub mod transfer;

use axum::{routing::get, routing::post, Router};

use crate::routes::WalletState;

/// Build v1 router.
pub fn router() -> Router<WalletState> {
    Router::new()
        .route("/balance", get(balance::balance))
        .route("/issue", post(issue::issue))
        .route("/transfer", post(transfer::transfer))
        .route("/burn", post(burn::burn))
        .route("/hold", post(escrow::hold))
        .route("/capture", post(escrow::capture))
        .route("/release", post(escrow::release))
        .route("/tx/:txid", get(receipt::receipt))
}
