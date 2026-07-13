//! RO:WHAT — Loopback operator HTTP controls for persistence-review metadata.
//!
//! RO:WHY — BUILD_PLAN_Z Phase 11 requires real status, pending, submission,
//! and rejection controls before approval or pinning can be exposed safely.
//!
//! RO:INTERACTS — `RuntimeStatus`, `svc-storage::PersistenceCatalog`, exact B3
//! identifiers, canonical `AssetKind`, and the macronode admin router.
//!
//! RO:INVARIANTS — one shared process catalog; exact B3 only; bounded pending
//! lists; duplicate registration does not reset state; no fake durability.
//!
//! RO:SECURITY — guarded admin routes only; metadata workflow only; no durable
//! byte write, serve override, provider mutation, reward, wallet, or ledger
//! authority.
//!
//! RO:TEST — `tests/persistence_http.rs`.

#![forbid(unsafe_code)]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ron_policy::{B3Id, PersistencePolicy, PersistenceReviewLevel};
use ron_proto::asset::AssetKind;
use serde::Deserialize;
use serde_json::{json, Value};
use svc_storage::persistence_catalog::{
    PersistenceCandidate, PersistenceCatalogError, MAX_PERSISTENCE_REVIEW_ITEMS,
};

use crate::types::{AppState, ModerationRuntimeSnapshot};

const DEFAULT_PENDING_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterRequest {
    object: String,
    asset_kind: AssetKind,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectRequest {
    object: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingQuery {
    limit: Option<usize>,
}

/// Register one exact object as an amnesia-first persistence candidate.
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Response {
    let object = match parse_object(&request.object) {
        Ok(object) => object,
        Err(error) => return invalid_object_response(error),
    };

    let catalog = state.runtime.persistence_catalog();
    let changed = catalog.register(object.clone(), request.asset_kind);

    let Some(candidate) = catalog.status(&object) else {
        return internal_catalog_error("registered persistence candidate was not readable");
    };

    mutation_response(StatusCode::OK, "register", changed, &candidate)
}

/// Return one exact candidate's current persistence-review state.
pub async fn status(State(state): State<AppState>, Path(raw_object): Path<String>) -> Response {
    let object = match parse_object(&raw_object) {
        Ok(object) => object,
        Err(error) => return invalid_object_response(error),
    };

    match state.runtime.persistence_catalog().status(&object) {
        Some(candidate) => (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "candidate": candidate_value(&candidate),
                "durableBytesWritten": false,
                "walletMutation": false,
                "ledgerMutation": false
            })),
        )
            .into_response(),

        None => catalog_error_response(PersistenceCatalogError::NotFound { object }),
    }
}

/// Return a deterministic bounded list of undecided persistence candidates.
pub async fn pending(State(state): State<AppState>, Query(query): Query<PendingQuery>) -> Response {
    let limit = query.limit.unwrap_or(DEFAULT_PENDING_LIMIT);

    match state.runtime.persistence_catalog().list_pending(limit) {
        Ok(candidates) => {
            let items: Vec<Value> = candidates.iter().map(candidate_value).collect();

            (
                StatusCode::OK,
                Json(json!({
                    "version": 1,
                    "limit": limit,
                    "count": items.len(),
                    "maximumLimit": MAX_PERSISTENCE_REVIEW_ITEMS,
                    "items": items,
                    "durableBytesWritten": false,
                    "walletMutation": false,
                    "ledgerMutation": false
                })),
            )
                .into_response()
        }

        Err(err) => catalog_error_response(err),
    }
}

/// Submit one exact amnesia-first object for explicit review.
pub async fn submit(State(state): State<AppState>, Json(request): Json<ObjectRequest>) -> Response {
    let object = match parse_object(&request.object) {
        Ok(object) => object,
        Err(error) => return invalid_object_response(error),
    };

    match state
        .runtime
        .persistence_catalog()
        .submit_for_review(&object)
    {
        Ok(mutation) => mutation_response(
            StatusCode::OK,
            "submit_for_review",
            mutation.changed(),
            mutation.candidate(),
        ),

        Err(err) => catalog_error_response(err),
    }
}

/// Approve one exact candidate through canonical persistence and
/// moderation policy.
///
/// The operator action permits the candidate's registered canonical asset
/// category for this review. Canonical moderation still wins over that local
/// approval. This changes eligibility metadata only and does not write bytes
/// to a durable backend.
pub async fn approve(
    State(state): State<AppState>,
    Json(request): Json<ObjectRequest>,
) -> Response {
    let object = match parse_object(&request.object) {
        Ok(object) => object,
        Err(error) => return invalid_object_response(error),
    };

    let catalog = state.runtime.persistence_catalog();

    let Some(candidate) = catalog.status(&object) else {
        return catalog_error_response(PersistenceCatalogError::NotFound { object });
    };

    let moderation = match state.runtime.persistence_moderation_policy() {
        Ok(policy) => policy,
        Err(snapshot) => {
            return moderation_unavailable_response(snapshot);
        }
    };

    let policy = operator_approval_policy(candidate.asset_kind());

    match catalog.approve(
        &object,
        &policy,
        &moderation,
        PersistenceReviewLevel::ModerationApproved,
    ) {
        Ok(mutation) => mutation_response(
            StatusCode::OK,
            "approve",
            mutation.changed(),
            mutation.candidate(),
        ),

        Err(err) => catalog_error_response(err),
    }
}

/// Reject persistence for one exact local review candidate.
///
/// Rejection moves the candidate into `operator_blocked`. It does not modify
/// canonical moderation policy and does not claim that object serving stopped.
pub async fn reject(State(state): State<AppState>, Json(request): Json<ObjectRequest>) -> Response {
    let object = match parse_object(&request.object) {
        Ok(object) => object,
        Err(error) => return invalid_object_response(error),
    };

    match state.runtime.persistence_catalog().reject(&object) {
        Ok(mutation) => mutation_response(
            StatusCode::OK,
            "reject",
            mutation.changed(),
            mutation.candidate(),
        ),

        Err(err) => catalog_error_response(err),
    }
}

fn operator_approval_policy(asset_kind: AssetKind) -> PersistencePolicy {
    let mut policy = PersistencePolicy::default();

    // The explicit operator approval supplies category permission for this
    // candidate only. Moderation approval remains the required review level.
    let _newly_allowed = policy.allow_asset_kind(asset_kind);

    policy
}

fn parse_object(raw: &str) -> Result<B3Id, String> {
    raw.parse::<B3Id>().map_err(|err| err.to_string())
}

fn moderation_unavailable_response(snapshot: ModerationRuntimeSnapshot) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "status": "moderation_unavailable",
            "moderationState": snapshot.state,
            "moderationSource": snapshot.source,
            "error":
                "effective moderation policy is unavailable; approval failed closed",
            "changed": false,
            "durableBytesWritten": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn invalid_object_response(error: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "status": "invalid_object",
            "error": error,
            "changed": false,
            "durableBytesWritten": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn candidate_value(candidate: &PersistenceCandidate) -> Value {
    json!({
        "object": candidate.object().as_str(),
        "assetKind": candidate.asset_kind(),
        "state": candidate.state().as_str(),
        "durableStorageEligible":
            candidate.is_durable_storage_eligible()
    })
}

fn mutation_response(
    status: StatusCode,
    action: &'static str,
    changed: bool,
    candidate: &PersistenceCandidate,
) -> Response {
    (
        status,
        Json(json!({
            "version": 1,
            "action": action,
            "changed": changed,
            "candidate": candidate_value(candidate),
            "durableBytesWritten": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn catalog_error_response(err: PersistenceCatalogError) -> Response {
    let status = match &err {
        PersistenceCatalogError::NotFound { .. } => StatusCode::NOT_FOUND,
        PersistenceCatalogError::InvalidLimit { .. } => StatusCode::BAD_REQUEST,
        PersistenceCatalogError::Transition(_) | PersistenceCatalogError::Policy(_) => {
            StatusCode::CONFLICT
        }
    };

    (
        status,
        Json(json!({
            "status": "rejected",
            "error": err.to_string(),
            "changed": false,
            "durableBytesWritten": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn internal_catalog_error(message: &'static str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "status": "internal_error",
            "error": message,
            "changed": false,
            "durableBytesWritten": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}
