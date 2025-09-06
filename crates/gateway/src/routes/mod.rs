// crates/gateway/src/routes/mod.rs
#![forbid(unsafe_code)]

mod object;
pub mod readyz;
mod errors;
mod http_util;

use axum::{routing::get, Router};

/// Build a STATELESS router (Router<()>).
/// We inject AppState later at the server entry via a service wrapper.
pub fn router() -> Router<()> {
    Router::new()
        // GET + HEAD both hit serve_object (branch on Method inside)
        .route("/o/:addr/*tail", get(object::serve_object).head(object::serve_object))
        .route("/healthz", get(readyz::healthz))
        .route("/readyz", get(readyz::readyz))
}
