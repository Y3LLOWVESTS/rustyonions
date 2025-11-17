//! RO:WHAT — Facet host plumbing (foundation stub + demo facet).
//! RO:WHY  — Provide a stable `facets::mount(router)` hook that preserves
//!           router state and attaches facet routes under `/facets/...`.
//! RO:INTERACTS — Called from `app::build_router` to attach facet routes.
//! RO:INVARIANTS — Preserve the router's state type `S` unchanged and keep
//!                 facet mounting cheap when only the demo facet is enabled.
//! RO:TEST — Exercised by `tests/facets_proxy.rs`.

use axum::{routing::get, Json, Router};
use serde_json::json;

/// Simple demo facet handler to prove Micronode can host facets.
///
/// Response shape: `{ "facet": "demo", "ok": true }`.
async fn demo_facet_ping() -> Json<serde_json::Value> {
    Json(json!({
        "facet": "demo",
        "ok": true
    }))
}

/// Mount facet routes onto the given router.
///
/// For now this mounts a single demo facet at `/facets/demo/ping`. The
/// function is generic over the router state `S` so we can call it with
/// `Router<AppState>` without forcing the state back to `()`.
pub fn mount<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.route("/facets/demo/ping", get(demo_facet_ping))
}
