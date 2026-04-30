//! RO:WHAT   v1 API surface aggregator (health/ping + facet stubs + paid/crab/assets/sites/identity product routes).
//! RO:WHY    Keep top-level router slim; v1 evolves independently.
//! RO:INVARS Only DTO-stable shapes; never leak internals; no wallet/ledger mutation in BFF routes.

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
        .route("/sites/prepare", post(sites::site_prepare))
        .route("/sites", post(sites::site_create))
        .route("/sites/:name", get(sites::site_resolve))
        // WEB3_2 product-proof asset-page routes.
        .nest("/crab", crab::router())
        .route("/b3/:asset", get(crab::resolve_b3_asset))
}
