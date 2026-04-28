//! RO:WHAT — Small runtime helpers for svc-wallet startup and graceful shutdown.
//! RO:WHY  — Pillar 12; Concerns: RES/GOV. Service startup should be explicit, bounded, and shutdown-aware.
//! RO:INTERACTS — main.rs, routes::WalletState, readiness.
//! RO:INVARIANTS — Ctrl-C drops readiness before listener shutdown; no background mutation hidden here.
//! RO:METRICS — none directly.
//! RO:CONFIG — WalletState::dev currently provides the Phase 2 dev runtime.
//! RO:SECURITY — no secrets.
//! RO:TEST — compile-time via binary build; integration smoke later.

use crate::{errors::WalletResult, routes::WalletState};

/// Build Phase 2 local runtime state.
pub fn build_dev_state() -> WalletResult<WalletState> {
    WalletState::dev()
}

/// Await a shutdown signal.
pub async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!(error = %err, "failed to listen for ctrl-c");
    }
}
