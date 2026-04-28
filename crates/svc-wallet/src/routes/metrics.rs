//! RO:WHAT — Prometheus text endpoint for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: PERF/RES/GOV. Wallet economic movement and rejects must be observable.
//! RO:INTERACTS — metrics::WalletMetrics, readiness::ReadinessGate.
//! RO:INVARIANTS — derivative counters only; never exposes accounts, memos, bearer tokens, or request bodies.
//! RO:METRICS — renders wallet_* series.
//! RO:CONFIG — none.
//! RO:SECURITY — text exposition has no secrets.
//! RO:TEST — metrics_rendered_by_metrics_module.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
};

use crate::routes::WalletState;

/// GET /metrics.
pub async fn metrics(State(state): State<WalletState>) -> impl IntoResponse {
    let _guard = state.metrics.begin_request();
    let body = state.metrics.render_prometheus(state.readiness.is_ready());
    state.metrics.inc_success();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}
