//! RO:WHAT   Verify endpoints (single + batch) with alg/aud checks and batch/size limits.
//! RO:WHY    Tighten negative paths per Beta criteria; stable, minimal error surface.
//! RO:INVARS No secret leakage; constant-time verify in issuer; early rejects pre-crypto.

use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

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

#[derive(Deserialize, Clone)]
pub struct Envelope {
    pub alg: String,
    pub kid: String,
    pub msg_b64: String,
    pub sig_b64: String,
    #[serde(default)]
    pub aud: Option<String>,
}

// Accept either {"items":[...]} or a bare array [...]
#[derive(Deserialize)]
#[serde(untagged)]
pub enum BatchBody {
    WithItems { items: Vec<Envelope> },
    Array(Vec<Envelope>),
}

fn alg_is_supported(alg: &str) -> bool {
    // Beta: only Ed25519
    alg == "Ed25519"
}

fn audience_check(
    required: bool,
    got: &Option<String>,
) -> Result<(), (StatusCode, Json<Problem<'static>>)> {
    if !required {
        return Ok(());
    }
    let Some(_got_aud) = got.as_ref().filter(|s| !s.is_empty()) else {
        return Err(problem(
            StatusCode::BAD_REQUEST,
            "BadAudience",
            "aud is required by policy",
        ));
    };
    Ok(())
}

fn kid_check(kid: &str) -> Result<(), (StatusCode, Json<Problem<'static>>)> {
    if kid.trim().is_empty() {
        return Err(problem(
            StatusCode::BAD_REQUEST,
            "BadKid",
            "kid must be non-empty",
        ));
    }
    Ok(())
}

pub async fn verify(
    Extension(issuer): Extension<Arc<IssuerState>>,
    Json(env): Json<Envelope>,
) -> Result<impl IntoResponse, (StatusCode, Json<Problem<'static>>)> {
    let _timer = OP_LATENCY.start_timer();

    if !alg_is_supported(&env.alg) {
        return Err(problem(
            StatusCode::BAD_REQUEST,
            "BadAlg",
            "Unsupported alg",
        ));
    }
    kid_check(&env.kid)?;
    audience_check(issuer.cfg.security.require_aud, &env.aud)?;

    // Decode
    let msg = B64.decode(&env.msg_b64).map_err(|_| {
        problem(
            StatusCode::BAD_REQUEST,
            "Malformed",
            "msg_b64 decode failed",
        )
    })?;
    let sig = B64.decode(&env.sig_b64).map_err(|_| {
        problem(
            StatusCode::BAD_REQUEST,
            "Malformed",
            "sig_b64 decode failed",
        )
    })?;

    // Verify
    let ok = issuer.verify(&env.kid, &msg, &sig).await.map_err(|_| {
        problem(
            StatusCode::BAD_REQUEST,
            "Malformed",
            "Invalid envelope or signature",
        )
    })?;

    OPS_TOTAL
        .with_label_values(&["verify", if ok { "ok" } else { "fail" }, "Ed25519"])
        .inc();
    Ok(Json(json!(ok)))
}

pub async fn verify_batch(
    Extension(issuer): Extension<Arc<IssuerState>>,
    Json(body): Json<BatchBody>,
) -> Result<impl IntoResponse, (StatusCode, Json<Problem<'static>>)> {
    let _timer = OP_LATENCY.start_timer();

    // Extract items accepting either shape
    let items: Vec<Envelope> = match body {
        BatchBody::WithItems { items } => items,
        BatchBody::Array(items) => items,
    };

    // Cap size
    let max_batch = issuer.cfg.limits.max_batch;
    if items.len() > max_batch {
        return Err(problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "BatchTooLarge",
            "items exceeds limits.max_batch",
        ));
    }

    let mut results = Vec::with_capacity(items.len());
    for env in items.iter() {
        if !alg_is_supported(&env.alg) {
            return Err(problem(
                StatusCode::BAD_REQUEST,
                "BadAlg",
                "Unsupported alg in batch",
            ));
        }
        kid_check(&env.kid)?;
        audience_check(issuer.cfg.security.require_aud, &env.aud)?;

        let msg = match B64.decode(&env.msg_b64) {
            Ok(v) => v,
            Err(_) => {
                return Err(problem(
                    StatusCode::BAD_REQUEST,
                    "Malformed",
                    "msg_b64 decode failed",
                ));
            }
        };
        let sig = match B64.decode(&env.sig_b64) {
            Ok(v) => v,
            Err(_) => {
                return Err(problem(
                    StatusCode::BAD_REQUEST,
                    "Malformed",
                    "sig_b64 decode failed",
                ));
            }
        };

        let ok = issuer.verify(&env.kid, &msg, &sig).await.map_err(|_| {
            problem(
                StatusCode::BAD_REQUEST,
                "Malformed",
                "Invalid envelope or signature",
            )
        })?;
        results.push(ok);
    }

    let any_ok = results.iter().any(|&v| v);
    OPS_TOTAL
        .with_label_values(&[
            "verify_batch",
            if any_ok { "ok" } else { "fail" },
            "Ed25519",
        ])
        .inc();
    Ok(Json(json!(results)))
}
