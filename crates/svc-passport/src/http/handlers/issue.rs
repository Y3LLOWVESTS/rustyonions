//! RO:WHAT   Issue + admin plane + JWKS export.
//! RO:WHY    Unit-state Router: state via Extension(Arc<_>); metrics intact.

use axum::{response::IntoResponse, Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    error::Error,
    metrics::{OPS_TOTAL, OP_LATENCY},
    state::issuer::IssuerState,
};

/// Minimal health probe keeps behavior stable even if other parts evolve.
pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

/// GET /v1/keys  -> JWKS (OKP/Ed25519)
pub async fn keys(
    Extension(issuer): Extension<Arc<IssuerState>>,
) -> Result<impl IntoResponse, Error> {
    let jwks = issuer.jwks().await?;
    Ok(Json(jwks))
}

/// POST /v1/passport/issue
/// Body: arbitrary JSON payload to be signed
/// Returns: Envelope { alg, kid, msg_b64, sig_b64 }
pub async fn issue(
    Extension(issuer): Extension<Arc<IssuerState>>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, Error> {
    let _timer = OP_LATENCY.start_timer();

    let bytes = serde_json::to_vec(&payload).map_err(|_| Error::Malformed)?;
    let (kid, sig) = issuer.sign(&bytes).await?;
    let env = issuer.build_envelope(kid, bytes, sig);

    OPS_TOTAL
        .with_label_values(&["issue", "ok", "Ed25519"])
        .inc();

    Ok(Json(json!({
        "alg": env.alg,
        "kid": env.kid,
        "sig_b64": env.sig_b64,
        "msg_b64": env.msg_b64
    })))
}

/// POST /admin/rotate  -> { current_kid }
pub async fn rotate(
    Extension(issuer): Extension<Arc<IssuerState>>,
) -> Result<impl IntoResponse, Error> {
    let kid = issuer.kms.rotate().await.map_err(Error::Internal)?;
    Ok(Json(json!({ "current_kid": kid })))
}

/// GET /admin/attest  -> attestation doc
pub async fn attest(
    Extension(issuer): Extension<Arc<IssuerState>>,
) -> Result<impl IntoResponse, Error> {
    let view = issuer.kms.attest().await.map_err(Error::Internal)?;
    Ok(Json(view))
}
