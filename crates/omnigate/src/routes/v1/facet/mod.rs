//! RO:WHAT   v1: /facet/{feed,media,graph} read-only stubs.
//! RO:WHY    Policy/capability probing + skeleton for hydration later.
//! RO:INVARS 200/JSON `{ ok: true }`, no leakage.

use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct FacetOk {
    pub ok: bool,
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/feed", get(feed))
        .route("/media", get(media))
        .route("/graph", get(graph))
}

pub async fn feed() -> Json<FacetOk> {
    Json(FacetOk { ok: true })
}

pub async fn media() -> Json<FacetOk> {
    Json(FacetOk { ok: true })
}

pub async fn graph() -> Json<FacetOk> {
    Json(FacetOk { ok: true })
}
