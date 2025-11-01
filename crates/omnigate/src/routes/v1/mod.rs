//! RO:WHAT   v1 API surface aggregator (health/ping + facet stubs).
//! RO:WHY    Keep top-level router slim; v1 evolves independently.
//! RO:INVARS Only DTO-stable shapes; never leak internals.

pub mod dht;
pub mod facet;
pub mod index;
pub mod mailbox;
pub mod objects;

use axum::Router;

/// Compose the whole v1 subtree.
///
/// Mount with:
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
}
