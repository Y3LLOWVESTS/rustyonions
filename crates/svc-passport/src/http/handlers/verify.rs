//! RO:WHAT — Verify one or many passports (batch coalescing policy).
//! RO:INVARIANTS — batch length bounded; same order responses; time checks before crypto.

use crate::{
    dto::verify::{
        VerifyBatchRequest, VerifyBatchResponse, VerifyRequest, VerifyResponse, VerifyResult,
    },
    error::Error,
    metrics::{BATCH_LEN, OPS_TOTAL, OP_LATENCY},
    state::issuer::IssuerState,
    verify::preflight,
    Config,
};
use axum::{extract::State, Json};
use std::{sync::Arc, time::Duration};

pub async fn verify_one(
    State((cfg, issuer, _)): State<(Config, Arc<IssuerState>, crate::health::Health)>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, Error> {
    let _t = OP_LATENCY.start_timer();
    let env = req.envelope;
    preflight::time_window(&cfg, &env)?;
    let ok = issuer.verify(&env).await?;
    OPS_TOTAL
        .with_label_values(&["verify", if ok { "ok" } else { "fail" }, "ed25519"])
        .inc();
    if ok {
        Ok(Json(VerifyResponse {
            ok: true,
            reason: None,
        }))
    } else {
        Err(Error::VerifyFailed)
    }
}

pub async fn verify_batch(
    State((cfg, issuer, _)): State<(Config, Arc<IssuerState>, crate::health::Health)>,
    Json(req): Json<VerifyBatchRequest>,
) -> Result<Json<VerifyBatchResponse>, Error> {
    let _t = OP_LATENCY.start_timer();
    let mut out = Vec::with_capacity(req.envelopes.len());
    let max = cfg.verify.max_batch.min(cfg.limits.max_batch);
    if req.envelopes.len() > max {
        return Err(Error::Malformed);
    }
    BATCH_LEN.observe(req.envelopes.len() as f64);

    // Preflight time checks (cheap) before crypto
    for env in &req.envelopes {
        if let Err(e) = preflight::time_window(&cfg, env) {
            out.push(VerifyResult {
                ok: false,
                reason: Some(format!("{e}")),
            });
        } else {
            out.push(VerifyResult {
                ok: false,
                reason: None,
            }); // placeholder; fill after verify
        }
    }

    // Build slice of those that passed preflight
    let mut idxs: Vec<usize> = out
        .iter()
        .enumerate()
        .filter_map(|(i, r)| if r.reason.is_none() { Some(i) } else { None })
        .collect();
    if !idxs.is_empty() {
        // Group by KID to enable batching per key
        let groups = issuer.group_by_kid(&req.envelopes, &idxs).await?;
        for (_kid, members) in groups {
            // Coalesce up to target_batch with a short wait to accumulate
            let target = cfg.verify.target_batch.max(1);
            let mut start = tokio::time::Instant::now();
            let wait = Duration::from_micros(cfg.verify.max_wait_us);
            let mut batch = members;
            while batch.len() < target && start.elapsed() < wait {
                tokio::task::yield_now().await;
            }
            let oks = issuer.verify_many(&req.envelopes, &batch).await?;
            for (pos, ok) in batch.into_iter().zip(oks.into_iter()) {
                out[pos] = if ok {
                    VerifyResult {
                        ok: true,
                        reason: None,
                    }
                } else {
                    VerifyResult {
                        ok: false,
                        reason: Some("verify_failed".into()),
                    }
                };
            }
        }
    }

    OPS_TOTAL
        .with_label_values(&["verify", "ok", "ed25519"])
        .inc_by(out.iter().filter(|r| r.ok).count() as u64);
    OPS_TOTAL
        .with_label_values(&["verify", "fail", "ed25519"])
        .inc_by(out.iter().filter(|r| !r.ok).count() as u64);
    Ok(Json(VerifyBatchResponse { results: out }))
}
