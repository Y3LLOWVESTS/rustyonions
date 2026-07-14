//! RO:WHAT — Guarded HTTP surface for bounded Service Node moderation review.
//! RO:WHY — BUILD_PLAN_Z Phase 21 requires explicit review decisions without silent policy mutation.
//! RO:INTERACTS — RuntimeStatus moderation snapshot and ModerationReviewCatalog.
//! RO:INVARIANTS — exact B3 only; bounded reads; approve/reject changes review metadata only.
//! RO:SECURITY — all review routes require admin authentication and expose no raw credentials or user IPs.
//! RO:TEST — pure response-boundary test below and moderation-review catalog tests.

#![forbid(unsafe_code)]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ron_policy::B3Id;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    services::moderation_review::{
        ModerationReviewDecisionV1, ModerationReviewError, ModerationReviewItemV1,
        ModerationReviewReasonV1, ModerationReviewSourceV1, MAX_MODERATION_REVIEW_READ_ITEMS,
    },
    types::{AppState, ModerationRuntimeSnapshot},
};

const DEFAULT_PENDING_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitModerationReviewRequest {
    object: String,
    source: ModerationReviewSourceV1,
    reason: ModerationReviewReasonV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewDecisionRequest {
    sequence: u64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingQuery {
    limit: Option<usize>,
}

pub async fn submit(
    State(state): State<AppState>,
    Json(request): Json<SubmitModerationReviewRequest>,
) -> Response {
    let object = match parse_object(&request.object) {
        Ok(object) => object,
        Err(error) => {
            return invalid_object_response(error);
        }
    };

    let effective_policy = match state.runtime.persistence_moderation_policy() {
        Ok(policy) => policy,
        Err(snapshot) => {
            return moderation_unavailable_response(snapshot);
        }
    };

    let decision = effective_policy.evaluate(&object);

    match state.runtime.moderation_review_catalog().submit(
        object,
        request.source,
        request.reason,
        decision.reason.as_str().to_string(),
        decision.permits_serve(),
        now_ms(),
    ) {
        Ok(mutation) => mutation_response(
            if mutation.changed() {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            },
            "submit",
            mutation.changed(),
            mutation.item(),
        ),

        Err(error) => error_response(error),
    }
}

pub async fn status(State(state): State<AppState>, Path(sequence): Path<u64>) -> Response {
    match state.runtime.moderation_review_catalog().status(sequence) {
        Some(item) => (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "candidate": item,
                "policyMutation": false,
                "runtimeActivation": false,
                "storageDelete": false,
                "providerWithdrawal": false,
                "rewardFinality": false,
                "walletMutation": false,
                "ledgerMutation": false
            })),
        )
            .into_response(),

        None => error_response(ModerationReviewError::NotFound { sequence }),
    }
}

pub async fn pending(State(state): State<AppState>, Query(query): Query<PendingQuery>) -> Response {
    let limit = query.limit.unwrap_or(DEFAULT_PENDING_LIMIT);

    match state
        .runtime
        .moderation_review_catalog()
        .list_pending(limit)
    {
        Ok(items) => (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "limit": limit,
                "count": items.len(),
                "maximumLimit":
                    MAX_MODERATION_REVIEW_READ_ITEMS,
                "items": items,
                "policyMutation": false,
                "runtimeActivation": false,
                "storageDelete": false,
                "providerWithdrawal": false,
                "rewardFinality": false,
                "walletMutation": false,
                "ledgerMutation": false
            })),
        )
            .into_response(),

        Err(error) => error_response(error),
    }
}

pub async fn approve(
    State(state): State<AppState>,
    Json(request): Json<ReviewDecisionRequest>,
) -> Response {
    review(
        &state,
        request.sequence,
        ModerationReviewDecisionV1::Approve,
        "approve_for_escalation",
    )
}

pub async fn reject(
    State(state): State<AppState>,
    Json(request): Json<ReviewDecisionRequest>,
) -> Response {
    review(
        &state,
        request.sequence,
        ModerationReviewDecisionV1::Reject,
        "reject",
    )
}

fn review(
    state: &AppState,
    sequence: u64,
    decision: ModerationReviewDecisionV1,
    action: &'static str,
) -> Response {
    match state
        .runtime
        .moderation_review_catalog()
        .review(sequence, decision, now_ms())
    {
        Ok(mutation) => {
            mutation_response(StatusCode::OK, action, mutation.changed(), mutation.item())
        }

        Err(error) => error_response(error),
    }
}

fn mutation_response(
    status: StatusCode,
    action: &'static str,
    changed: bool,
    item: &ModerationReviewItemV1,
) -> Response {
    (status, Json(mutation_value(action, changed, item))).into_response()
}

fn mutation_value(action: &'static str, changed: bool, item: &ModerationReviewItemV1) -> Value {
    json!({
        "version": 1,
        "action": action,
        "changed": changed,
        "candidate": item,
        "policyMutation": false,
        "runtimeActivation": false,
        "storageDelete": false,
        "providerWithdrawal": false,
        "rewardFinality": false,
        "walletMutation": false,
        "ledgerMutation": false
    })
}

fn parse_object(raw: &str) -> Result<B3Id, String> {
    raw.parse::<B3Id>().map_err(|error| error.to_string())
}

fn invalid_object_response(error: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "status": "invalid_object",
            "error": error,
            "changed": false,
            "policyMutation": false,
            "runtimeActivation": false,
            "storageDelete": false,
            "providerWithdrawal": false,
            "rewardFinality": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn moderation_unavailable_response(snapshot: ModerationRuntimeSnapshot) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "status": "moderation_unavailable",
            "moderationState": snapshot.state,
            "moderationSource": snapshot.source,
            "error":
                "effective moderation policy is unavailable; review submission failed closed",
            "changed": false,
            "policyMutation": false,
            "runtimeActivation": false,
            "storageDelete": false,
            "providerWithdrawal": false,
            "rewardFinality": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn error_response(error: ModerationReviewError) -> Response {
    let status = match &error {
        ModerationReviewError::InvalidReadLimit { .. } => StatusCode::BAD_REQUEST,

        ModerationReviewError::NotFound { .. } => StatusCode::NOT_FOUND,

        ModerationReviewError::ConflictingPendingReview { .. }
        | ModerationReviewError::TransitionConflict { .. } => StatusCode::CONFLICT,

        ModerationReviewError::CapacityReached { .. } => StatusCode::TOO_MANY_REQUESTS,

        ModerationReviewError::ZeroCapacity | ModerationReviewError::SequenceExhausted => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };

    (
        status,
        Json(json!({
            "status": "rejected",
            "error": error.to_string(),
            "changed": false,
            "policyMutation": false,
            "runtimeActivation": false,
            "storageDelete": false,
            "providerWithdrawal": false,
            "rewardFinality": false,
            "walletMutation": false,
            "ledgerMutation": false
        })),
    )
        .into_response()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::moderation_review::{ModerationReviewSourceV1, ModerationReviewStateV1};

    #[test]
    fn mutation_response_never_claims_policy_or_economic_authority() {
        let item = ModerationReviewItemV1 {
            sequence: 1,
            object: "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            source: ModerationReviewSourceV1::OperatorReport,
            reason: ModerationReviewReasonV1::AbuseReport,
            effective_policy_reason: "no_rule".to_string(),
            currently_permits_serve: true,
            state: ModerationReviewStateV1::ApprovedForEscalation,
            submitted_at_ms: 1,
            reviewed_at_ms: Some(2),
        };

        let value = mutation_value("approve_for_escalation", true, &item);

        assert_eq!(value["changed"], true);
        assert_eq!(value["policyMutation"], false);
        assert_eq!(value["runtimeActivation"], false);
        assert_eq!(value["storageDelete"], false);
        assert_eq!(value["providerWithdrawal"], false);
        assert_eq!(value["rewardFinality"], false);
        assert_eq!(value["walletMutation"], false);
        assert_eq!(value["ledgerMutation"], false);
    }
}
