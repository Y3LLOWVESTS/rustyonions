//! RO:WHAT  — v1 App plane stub router.
//! RO:WHY   — Reserve `/v1/app/*` and provide a simple health endpoint so we
//!            can start hanging real app-plane routes under this prefix.
//!
//! RO:INVARS
//!   - No business logic here: this module only defines HTTP shape.
//!   - It is safe to replace this stub with a `RonApp`-backed router later
//!     without breaking the `/v1/app/*` URL contract.
//!
//! In a later slice, this module will:
//!   - Host one or more `RonApp` implementations from `ron-app-sdk`.
//!   - Use `mount_app()` to bridge AppContract routes onto axum.
//!   - Enforce capability / auth extraction before invoking app handlers.

use axum::{routing::get, Json, Router};
use serde::Serialize;

/// Simple health payload for the app plane stub.
#[derive(Debug, Serialize)]
struct AppPlaneHealth {
    /// Always `true` for now; later we can surface per-app readiness.
    ok: bool,
    /// Human-readable note for debugging / integration tests.
    note: &'static str,
}

/// Build the `/v1/app/*` routing tree.
///
/// For now this only exposes:
///   - `GET /v1/app/healthz`
///
/// Later, this router will be replaced (or extended) to mount real RON apps
/// defined via the `ron-app-sdk` App Contract.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/healthz", get(get_health))
}

/// `GET /v1/app/healthz`
///
/// Minimal "is the app plane wired?" probe. This is mostly for integration
/// tests and manual curl checks while we bootstrap the App Plane.
async fn get_health() -> Json<AppPlaneHealth> {
    Json(AppPlaneHealth {
        ok: true,
        note: "app plane stub mounted",
    })
}
