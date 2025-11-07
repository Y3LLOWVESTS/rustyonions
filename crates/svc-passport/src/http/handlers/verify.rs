//! RO:WHAT   Verify endpoints: single and batch (Envelope-based).
//! RO:WHY    Unit-state Router: pull IssuerState via Extension(Arc<_>); keep metrics.
//! RO:INVARS No secret leakage; errors mapped by Error; low-cardinality labels.

use axum::{http::StatusCode, Extension, Json};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::Arc;

use crate::{
    dto::verify::Envelope,
    error::Error,
    metrics::{BATCH_LEN, OPS_TOTAL, OP_LATENCY},
    state::issuer::IssuerState,
};

/// POST /v1/passport/verify  Envelope -> bool
pub async fn verify_one(
    Extension(issuer): Extension<Arc<IssuerState>>,
    Json(env): Json<Envelope>,
) -> Result<(StatusCode, Json<bool>), Error> {
    let _t = OP_LATENCY.start_timer();

    let msg = STANDARD
        .decode(&env.msg_b64)
        .map_err(|_| Error::Malformed)?;
    let sig = STANDARD
        .decode(&env.sig_b64)
        .map_err(|_| Error::Malformed)?;
    let ok = issuer.verify(&env.kid, &msg, &sig).await?;

    OPS_TOTAL
        .with_label_values(&["verify", if ok { "ok" } else { "bad_sig" }, "Ed25519"])
        .inc();

    Ok((StatusCode::OK, Json(ok)))
}

/// POST /v1/passport/verify_batch  [Envelope] -> [bool]
pub async fn verify_batch(
    Extension(issuer): Extension<Arc<IssuerState>>,
    Json(envs): Json<Vec<Envelope>>,
) -> Result<(StatusCode, Json<Vec<bool>>), Error> {
    let _t = OP_LATENCY.start_timer();
    BATCH_LEN.observe(envs.len() as f64);

    let mut out = Vec::with_capacity(envs.len());
    for env in &envs {
        let msg = STANDARD
            .decode(&env.msg_b64)
            .map_err(|_| Error::Malformed)?;
        let sig = STANDARD
            .decode(&env.sig_b64)
            .map_err(|_| Error::Malformed)?;
        let ok = issuer.verify(&env.kid, &msg, &sig).await?;
        out.push(ok);
    }

    OPS_TOTAL
        .with_label_values(&["verify_batch", "ok", "Ed25519"])
        .inc();

    Ok((StatusCode::OK, Json(out)))
}
