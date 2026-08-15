// crates/macronode/src/http_admin/router.rs
//! RO:WHAT — Router builder for Macronode admin plane.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};

use crate::{
    http_admin::middleware::{auth, rate_limit, request_accounting, request_id, timeout},
    readiness::ReadyProbes,
    types::AppState,
};

pub fn build_router(state: AppState) -> Router {
    let probes: Arc<ReadyProbes> = state.probes.clone();

    let base = Router::new()
        .route(
            "/version",
            get(crate::http_admin::handlers::version::handler),
        )
        .route(
            "/healthz",
            get(crate::http_admin::handlers::healthz::handler),
        )
        .route(
            "/readyz",
            get(move || {
                let probes = probes.clone();
                crate::http_admin::handlers::readyz::handler(probes)
            }),
        )
        .route(
            "/metrics",
            get(crate::http_admin::handlers::metrics::handler),
        )
        .route(
            "/api/v1/status",
            get(crate::http_admin::handlers::status::handler),
        )
        // Storage / databases (read-only inventory; safe facts only).
        .route(
            "/api/v1/storage/summary",
            get(crate::http_admin::handlers::storage::storage_summary),
        )
        .route(
            "/api/v1/storage/databases",
            get(crate::http_admin::handlers::storage::storage_databases),
        )
        .route(
            "/api/v1/storage/databases/:name",
            get(crate::http_admin::handlers::storage::storage_database_detail),
        )
        // System summary (CPU/RAM + optional network rate).
        .route(
            "/api/v1/system/summary",
            get(crate::http_admin::handlers::system_summary::handler),
        )
        // System net accounting (bytes+req rollups + chart series).
        .route(
            "/api/v1/system/net/accounting",
            get(crate::http_admin::handlers::system_net_accounting::handler),
        )
        // Benchmarks (node-executed; safe bounded runs).
        .route(
            "/api/v1/bench/run",
            post(crate::http_admin::handlers::bench::run),
        )
        .route(
            "/api/v1/bench/runs/:run_id",
            get(crate::http_admin::handlers::bench::status),
        )
        .route(
            "/api/v1/bench/runs/:run_id/result",
            get(crate::http_admin::handlers::bench::result),
        )
        // Local service-node admin controls used by crabnode.
        .route(
            "/api/v1/admin/ui/enable",
            post(crate::http_admin::handlers::admin::enable_web),
        )
        .route(
            "/api/v1/admin/ui/disable",
            post(crate::http_admin::handlers::admin::disable_web),
        )
        .route(
            "/api/v1/admin/setup-token",
            post(crate::http_admin::handlers::admin::setup_token),
        )
        .route(
            "/api/v1/admin/setup-token/consume",
            post(crate::http_admin::handlers::admin::consume_setup_token),
        )
        // Process-local persistence-review controls. These routes mutate
        // eligibility metadata only and do not claim durable byte writes.
        .route(
            "/api/v1/persistence/register",
            post(crate::http_admin::handlers::persistence::register),
        )
        .route(
            "/api/v1/persistence/status/:object",
            get(crate::http_admin::handlers::persistence::status),
        )
        .route(
            "/api/v1/persistence/pending",
            get(crate::http_admin::handlers::persistence::pending),
        )
        .route(
            "/api/v1/persistence/submit",
            post(crate::http_admin::handlers::persistence::submit),
        )
        .route(
            "/api/v1/persistence/approve",
            post(crate::http_admin::handlers::persistence::approve),
        )
        .route(
            "/api/v1/persistence/reject",
            post(crate::http_admin::handlers::persistence::reject),
        )
        // Bounded moderation-review queue. Approve/reject changes
        // review metadata only and never edits the effective policy.
        .route(
            "/api/v1/moderation/review/submit",
            post(crate::http_admin::handlers::moderation_review::submit),
        )
        .route(
            "/api/v1/moderation/review/status/:sequence",
            get(crate::http_admin::handlers::moderation_review::status),
        )
        .route(
            "/api/v1/moderation/review/pending",
            get(crate::http_admin::handlers::moderation_review::pending),
        )
        .route(
            "/api/v1/moderation/review/approve",
            post(crate::http_admin::handlers::moderation_review::approve),
        )
        .route(
            "/api/v1/moderation/review/reject",
            post(crate::http_admin::handlers::moderation_review::reject),
        )
        // Exact local storage/provider pruning. This route reports each
        // mutation step independently and claims no network-wide deletion.
        .route(
            "/api/v1/moderation/prune",
            post(crate::http_admin::handlers::prune::handler),
        )
        // Local reward-recipient operator controls used by crabnode.
        .route(
            "/api/v1/rewards/status",
            get(crate::http_admin::handlers::rewards::status),
        )
        .route(
            "/api/v1/rewards/bind",
            post(crate::http_admin::handlers::rewards::bind),
        )
        .route(
            "/api/v1/rewards/rotate",
            post(crate::http_admin::handlers::rewards::rotate),
        )
        // FINAL_BETA Phase 19: one guarded Service Node quorum
        // signature only. The handler cannot aggregate quorum, finalize a
        // checkpoint, execute a payout, or mutate wallet/ledger state.
        .route(
            "/api/v1/quickchain/quorum/sign",
            post(crate::http_admin::handlers::quorum::sign),
        )
        // FINAL_BETA Phase 19: one guarded checkpoint-validator
        // signature only. No committee aggregation or finality occurs here.
        .route(
            "/api/v1/quickchain/checkpoint/sign",
            post(crate::http_admin::handlers::checkpoint::sign),
        )
        // Control plane actions.
        .route(
            "/api/v1/reload",
            post(crate::http_admin::handlers::reload::handler),
        )
        .route(
            "/api/v1/shutdown",
            post(crate::http_admin::handlers::shutdown::handler),
        )
        .route(
            "/api/v1/debug/crash",
            post(crate::http_admin::handlers::debug_crash::handler),
        )
        .with_state(state);

    // Middleware stack:
    //
    // NOTE: Router::layer wraps the router; the last .layer() is the outermost.
    //
    // We insert request_accounting so it can count requests (low-cardinality) for rollups/series.
    base.layer(from_fn(rate_limit::layer))
        .layer(from_fn(auth::layer)) // only applies to guarded paths
        .layer(from_fn(timeout::layer))
        .layer(from_fn(request_accounting::layer))
        .layer(from_fn(request_id::layer))
}
