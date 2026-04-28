//! RO:WHAT — Axum route handlers for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/SEC. Thin adapters enforce caps/auth and call pure compute.
//! RO:INTERACTS — http DTOs, input resolvers, core compute, output intents/artifacts, metrics/readiness/security.
//! RO:INVARIANTS — auth before compute; no lock across await; idempotent epoch replay; quarantine before egress.
//! RO:METRICS — updates reward_runs_total, reward_compute_latency_seconds, rejected_total, ledger_intents_total.
//! RO:CONFIG — idempotency salt, amnesia artifact behavior, default policy, wallet issue path.
//! RO:SECURITY — requires Bearer dev or route scope token; never logs Authorization.
//! RO:TEST — integration/http_compute.rs and readiness.rs.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::bus::events::RewarderEvent;
use crate::core::{compute_manifest, run_key, ComputeInput};
use crate::http::dto::{ComputeEpochRequest, VersionResponse};
use crate::http::error::HttpError;
use crate::http::RewarderState;
use crate::inputs::{resolve_accounting_snapshot, resolve_reward_policy, ContentCid};
use crate::outputs::artifacts::maybe_write_manifest;
use crate::outputs::{DevWalletIssueClient, IntentResult, SettlementBatch, WalletIssueClient};
use crate::security::caps::{require_scope, Scope};
use crate::{Result, RewarderError};

/// Liveness.
pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// Readiness.
pub async fn readyz(State(state): State<RewarderState>) -> Response {
    let missing = state.health.missing();
    for cause in [
        "config_loaded",
        "ledger_ok",
        "policy_registry_ok",
        "queue_ok",
    ] {
        state.metrics.set_degraded(cause, missing.contains(&cause));
    }
    if missing.is_empty() {
        (StatusCode::OK, Json(state.health.snapshot())).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            [("Retry-After", "1")],
            Json(serde_json::json!({
                "degraded": true,
                "missing": missing,
                "retry_after": 1
            })),
        )
            .into_response()
    }
}

/// Metrics endpoint.
pub async fn metrics(State(state): State<RewarderState>) -> Response {
    match state.metrics.render() {
        Ok(body) => (StatusCode::OK, body).into_response(),
        Err(err) => HttpError::new(err, "metrics").into_response(),
    }
}

/// Build/version endpoint.
pub async fn version() -> impl IntoResponse {
    Json(VersionResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        features: vec![
            #[cfg(feature = "tls")]
            "tls",
            #[cfg(feature = "pq-hybrid")]
            "pq-hybrid",
            #[cfg(feature = "pq-sign")]
            "pq-sign",
        ],
    })
}

/// Compute rewards for an epoch.
pub async fn compute_epoch(
    State(state): State<RewarderState>,
    Path(epoch_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ComputeEpochRequest>,
) -> Response {
    let corr_id = corr_id(&headers);
    let started = Instant::now();
    let result = compute_epoch_inner(&state, &epoch_id, &headers, req).await;
    state
        .metrics
        .observe_compute_seconds(started.elapsed().as_secs_f64());

    match result {
        Ok(manifest) => (StatusCode::OK, Json(manifest)).into_response(),
        Err(err) => {
            state.metrics.inc_reject(err.reason());
            if matches!(err, RewarderError::Quarantined(_)) {
                state.metrics.inc_run("quarantined");
            }
            HttpError::new(err, corr_id).into_response()
        }
    }
}

async fn compute_epoch_inner(
    state: &RewarderState,
    epoch_id: &str,
    headers: &HeaderMap,
    req: ComputeEpochRequest,
) -> Result<crate::outputs::RewardManifest> {
    require_scope(headers, Scope::Run)?;
    validate_epoch_id(epoch_id)?;

    let ComputeEpochRequest {
        inputs_cid,
        policy_id,
        policy_hash,
        dry_run,
        notes: _notes,
        snapshot,
        policy,
    } = req;

    let cid = ContentCid::parse(&inputs_cid)?;
    if policy_hash != policy_hash.to_ascii_lowercase() {
        return Err(RewarderError::BadRequest(
            "policy_hash must be lowercase canonical b3 hex".into(),
        ));
    }

    let key = run_key(
        epoch_id,
        &policy_hash,
        cid.as_str(),
        &state.config.rewarder.idempotency_salt,
    );
    let epoch_key = epoch_id.to_owned();

    if let Some(existing) = state.manifests.get(&epoch_key) {
        if existing.run_key == key && (dry_run || existing.ledger.result != "dry_run") {
            return Ok(existing);
        }
        if existing.run_key != key {
            return Err(RewarderError::Conflict(
                "epoch already computed with different policy_hash or inputs_cid".into(),
            ));
        }
        // If the only existing record is a dry-run manifest and this request is production,
        // continue so the run can be promoted into a real settlement intent.
    }

    let permit = state
        .gates
        .compute()
        .try_acquire_owned()
        .map_err(|_| RewarderError::Busy("compute workers exhausted".into()))?;

    let snapshot = resolve_accounting_snapshot(&cid, snapshot)?;
    let policy = resolve_reward_policy(policy, &policy_id, &policy_hash)?;

    let compute_input = ComputeInput {
        epoch_id: epoch_key.clone(),
        inputs_cid: cid,
        policy,
        snapshot,
        dry_run,
        idempotency_salt: state.config.rewarder.idempotency_salt.clone(),
    };

    // Validate pure economic path before any wallet/ledger egress can happen.
    let validated = compute_manifest(compute_input.clone(), IntentResult::DryRun)?;
    let settlement = SettlementBatch::from_manifest(&validated)?;
    state.metrics.inc_planned_intents(settlement.intents.len());

    let wallet = DevWalletIssueClient::new(
        state.intents.clone(),
        state.config.ingress.wallet_issue_path.clone(),
    );
    let egress = wallet.emit_issue_batch(&settlement, dry_run)?.result;
    state.metrics.inc_intent(egress.as_str());

    state.bus.publish(RewarderEvent::RunStarted {
        epoch_id: epoch_key.clone(),
        run_key: key.clone(),
    });

    let manifest = compute_manifest(compute_input, egress)?;
    drop(permit);

    maybe_write_manifest(&state.config, &manifest)?;
    state.metrics.inc_run(manifest.status.as_str());

    state.bus.publish(RewarderEvent::RunCompleted {
        epoch_id: epoch_key.clone(),
        run_key: manifest.run_key.clone(),
        status: manifest.status.as_str().into(),
    });

    state.manifests.insert(epoch_key, manifest.clone());
    Ok(manifest)
}

/// Fetch a sealed manifest.
pub async fn get_epoch(
    State(state): State<RewarderState>,
    Path(epoch_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let corr_id = corr_id(&headers);
    match require_scope(&headers, Scope::Inspect)
        .and_then(|()| validate_epoch_id(&epoch_id))
        .and_then(|()| {
            state
                .manifests
                .get(&epoch_id)
                .ok_or_else(|| RewarderError::NotFound("epoch manifest not found".into()))
        }) {
        Ok(manifest) => (StatusCode::OK, Json(manifest)).into_response(),
        Err(err) => {
            state.metrics.inc_reject(err.reason());
            HttpError::new(err, corr_id).into_response()
        }
    }
}

/// Preview wallet issue requests for a computed epoch.
///
/// This is deliberately read-only: it derives deterministic wallet `/v1/issue` request DTOs from
/// the sealed manifest and does not emit or commit anything.
pub async fn get_settlement(
    State(state): State<RewarderState>,
    Path(epoch_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let corr_id = corr_id(&headers);
    match require_scope(&headers, Scope::Inspect)
        .and_then(|()| validate_epoch_id(&epoch_id))
        .and_then(|()| {
            state
                .manifests
                .get(&epoch_id)
                .ok_or_else(|| RewarderError::NotFound("epoch manifest not found".into()))
        })
        .and_then(|manifest| {
            let settlement = SettlementBatch::from_manifest(&manifest)?;
            let wallet = DevWalletIssueClient::new(
                state.intents.clone(),
                state.config.ingress.wallet_issue_path.clone(),
            );
            wallet.preview_issue_batch(&settlement)
        }) {
        Ok(batch) => (StatusCode::OK, Json(batch)).into_response(),
        Err(err) => {
            state.metrics.inc_reject(err.reason());
            HttpError::new(err, corr_id).into_response()
        }
    }
}

fn validate_epoch_id(epoch_id: &str) -> Result<()> {
    if epoch_id.is_empty() || epoch_id.len() > 128 {
        return Err(RewarderError::BadRequest(
            "epoch_id must be 1..=128 bytes".into(),
        ));
    }
    if !epoch_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.')
    {
        return Err(RewarderError::BadRequest(
            "epoch_id contains unsupported characters".into(),
        ));
    }
    Ok(())
}

fn corr_id(headers: &HeaderMap) -> String {
    headers
        .get("x-corr-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            format!("corr-{millis}")
        })
}
