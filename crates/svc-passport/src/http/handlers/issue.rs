//! RO:WHAT   Issue + admin plane + JWKS export + max_msg_bytes enforcement.
//! RO:WHY    Enforce size caps early; keep stable problem docs; no secret leakage.
//! RO:INTERACTS  state::issuer (jwks/sign/verify), metrics.

use axum::{
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use base64::Engine as _; // base64 0.22 API for .encode()
use blake3;

use crate::{
    metrics::{OPS_TOTAL, OP_LATENCY},
    state::issuer::IssuerState,
};

#[derive(serde::Serialize)]
pub struct Problem<'a> {
    pub code: &'a str,
    pub message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

fn problem(
    status: StatusCode,
    code: &'static str,
    msg: &'static str,
) -> (StatusCode, Json<Problem<'static>>) {
    (
        status,
        Json(Problem {
            code,
            message: msg,
            retry_after_ms: None,
        }),
    )
}

/// Minimal health probe (kept here for symmetry with other services).
pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

/// GET /v1/keys  -> JWKS (OKP/Ed25519)
/// Adds correct media type, cache headers, and ETag; supports If-None-Match → 304.
pub async fn keys(
    Extension(issuer): Extension<Arc<IssuerState>>,
    req_headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Problem<'static>>)> {
    // Build JWKS (as serde_json::Value) and serialize exactly once.
    let jwks = issuer
        .jwks()
        .await
        .map_err(|_| problem(StatusCode::INTERNAL_SERVER_ERROR, "Internal", "JWKS failed"))?;
    let body = serde_json::to_vec(&jwks).map_err(|_| {
        problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal",
            "JWKS encode failed",
        )
    })?;

    // Strong ETag over exact bytes using BLAKE3.
    let etag_hex = blake3::hash(&body).to_hex().to_string();
    // Keep it opaque and quoted (per RFC 7232). Prefix optional; we can omit to keep it shorter.
    let etag_value = format!("\"{}\"", etag_hex);

    // Conditional GET
    if let Some(if_none_match) = req_headers.get(header::IF_NONE_MATCH) {
        if if_none_match
            .to_str()
            .ok()
            .filter(|v| *v == etag_value)
            .is_some()
        {
            let mut h = HeaderMap::new();
            h.insert(header::ETAG, HeaderValue::from_str(&etag_value).unwrap());
            h.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/jwk-set+json"),
            );
            let max_age = issuer.cfg.cache.jwks_ttl_s;
            h.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_str(&format!("public, max-age={}", max_age)).unwrap(),
            );
            OPS_TOTAL
                .with_label_values(&["keys", "not_modified", "n/a"])
                .inc();
            return Ok((StatusCode::NOT_MODIFIED, h).into_response());
        }
    }

    // Normal 200
    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/jwk-set+json"),
    );
    h.insert(header::ETAG, HeaderValue::from_str(&etag_value).unwrap());
    let max_age = issuer.cfg.cache.jwks_ttl_s;
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&format!("public, max-age={}", max_age)).unwrap(),
    );
    OPS_TOTAL.with_label_values(&["keys", "ok", "n/a"]).inc();
    Ok((h, body).into_response())
}

/// POST /v1/passport/issue
/// Body: arbitrary JSON payload to be signed
/// Returns: Envelope { alg, kid, msg_b64, sig_b64, aud? }
pub async fn issue(
    Extension(issuer): Extension<Arc<IssuerState>>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Problem<'static>>)> {
    let _timer = OP_LATENCY.start_timer();

    // Serialize to bytes to know exact size.
    let bytes = serde_json::to_vec(&payload).map_err(|_| {
        problem(
            StatusCode::BAD_REQUEST,
            "Malformed",
            "Body is not valid JSON",
        )
    })?;

    // Enforce max message size.
    let max = issuer.cfg.limits.max_msg_bytes;
    if bytes.len() > max {
        return Err(problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "MsgTooLarge",
            "Message exceeds limits.max_msg_bytes",
        ));
    }

    // Sign
    let (kid, sig) = issuer.sign(&bytes).await.map_err(|_| {
        problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal",
            "Signer failed",
        )
    })?;

    // If require_aud=true, include default audience = passport.issuer
    let maybe_aud = if issuer.cfg.security.require_aud {
        Some(issuer.cfg.passport.issuer.clone())
    } else {
        None
    };

    OPS_TOTAL
        .with_label_values(&["issue", "ok", "Ed25519"])
        .inc();

    let mut env = json!({
        "alg": "Ed25519",
        "kid": kid,
        "sig_b64": base64::engine::general_purpose::STANDARD.encode(&sig),
        "msg_b64": base64::engine::general_purpose::STANDARD.encode(&bytes)
    });

    if let Some(aud) = maybe_aud {
        env.as_object_mut()
            .expect("envelope object")
            .insert("aud".into(), json!(aud));
    }

    Ok(Json(env))
}

/// POST /admin/rotate  -> { current_kid }
pub async fn rotate(
    Extension(issuer): Extension<Arc<IssuerState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Problem<'static>>)> {
    let kid = issuer.kms.rotate().await.map_err(|_| {
        problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal",
            "Rotate failed",
        )
    })?;
    Ok(Json(json!({ "current_kid": kid })))
}

/// GET /admin/attest  -> attestation doc enriched with build + KID view
pub async fn attest(
    Extension(issuer): Extension<Arc<IssuerState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Problem<'static>>)> {
    let kms_view = issuer.kms.attest().await.map_err(|_| {
        problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal",
            "Attest failed",
        )
    })?;

    let current_kid = kms_view.get("current").cloned().unwrap_or(json!(null));
    let kids = kms_view
        .get("keys")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|k| k.get("kid").cloned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let build = json!({
        "service": "svc-passport",
        "version": env!("CARGO_PKG_VERSION"),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
    });

    Ok(Json(json!({
        "build": build,
        "current_kid": current_kid,
        "kids": kids,
        "kms": kms_view
    })))
}
