//! RO:WHAT   v1 API surface aggregator for health, facets, paid routes, crab assets, sites, identity, and wallet façade routes.
//! RO:WHY    P6/P7/P12; Concerns: DX/SEC/ECON. Keep top-level routing slim while exposing stable product contracts.
//! RO:INTERACTS — routes/v1/* modules, svc-gateway product proxy, CrabLink extension.
//! RO:INVARIANTS — DTO-stable shapes; no ledger mutation here; wallet mutations are proxied only through svc-wallet.
//! RO:METRICS — route-specific middleware wraps this subtree from the app bootstrap.
//! RO:CONFIG — child modules read their own env/config knobs.
//! RO:SECURITY — no ambient authority; child routes enforce/forward capability context.
//! RO:TEST — omnigate route tests plus svc-gateway proxy tests and CrabLink smoke scripts.

pub mod app;
pub mod assets;
pub mod crab;
pub mod dht;
pub mod facet;
pub mod identity;
pub mod index;
pub mod mailbox;
pub mod objects;
pub mod paid;
pub mod sites;
pub mod wallet;

use axum::{
    routing::{get, post},
    Router,
};

/// Compose the whole v1 subtree.
///
/// Mount with:
///
/// ```ignore
/// .nest("/v1", routes::v1::router())
/// ```
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(index::router()) // includes /ping and /index/healthz
        .nest("/objects", objects::router())
        .nest("/mailbox", mailbox::router())
        .nest("/dht", dht::router())
        .nest("/facet", facet::router())
        .nest("/app", app::router())
        .nest("/paid", paid::router())
        .nest("/assets", assets::router())
        .nest("/identity", identity::router())
        .route("/wallet/:account/balance", get(wallet::balance))
        .route("/wallet/hold", post(wallet::hold))
        .route("/sites/prepare", post(sites::site_prepare))
        .route("/sites", post(sites::site_create))
        .route("/sites/:name", get(sites::site_resolve))
        // WEB3_2 product-proof asset-page routes.
        .nest("/crab", crab::router())
        .route("/b3/:asset", get(crab::resolve_b3_asset))
}
