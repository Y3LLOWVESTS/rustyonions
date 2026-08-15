//! RO:WHAT — Guarded local HTTP adapter for one Service Node QuickChain quorum signature.
//! RO:WHY — FINAL_BETA Phase 19 needs real macronode processes to expose their independently
//!          validated signature without granting the HTTP/admin plane quorum or finality authority.
//! RO:INTERACTS — AppState::runtime, canonical RocEpochTransitionIdentityV1, and the registered
//!                QuorumParticipationRuntime.
//! RO:INVARIANTS — caller supplies canonical identity only; transition hash is recomputed by the
//!                 participant; exactly one Service Node signature may be returned.
//! RO:SECURITY — admin guarded; unconfigured runtime fails closed; no quorum aggregation,
//!               checkpoint finalization, wallet mutation, ledger mutation, payout, receipt,
//!               paid unlock, or CrabLink finality authority.
//! RO:TEST — focused handler tests below plus auth middleware regression coverage.

#![forbid(unsafe_code)]

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ron_proto::RocEpochTransitionIdentityV1;
use serde_json::json;

use crate::{services::quorum_participation::QuorumParticipationRuntimeError, types::AppState};

/// POST /api/v1/quickchain/quorum/sign
///
/// The body is the canonical transition identity itself. There is no
/// caller-supplied transition-hash field. The registered Service Node
/// participant validates and hashes the identity locally before signing.
pub async fn sign(
    State(state): State<AppState>,
    Json(identity): Json<RocEpochTransitionIdentityV1>,
) -> Response {
    match state.runtime.sign_quorum_transition(&identity) {
        Ok(signature) => (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "status": "signed",
                "signature": signature,

                "serviceNodeSignatureCreated": true,

                "quorumAggregated": false,
                "quorumFinalized": false,
                "checkpointFinalized": false,

                "walletMutation": false,
                "ledgerMutation": false,
                "payoutExecuted": false,
                "receiptCreated": false,
                "confirmedRocReported": false,
                "paidUnlock": false,

                "finality": false,
                "crabLinkFinalityAuthority": false
            })),
        )
            .into_response(),

        Err(error) => error_response(error),
    }
}

fn error_response(error: QuorumParticipationRuntimeError) -> Response {
    let status = match &error {
        QuorumParticipationRuntimeError::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,

        QuorumParticipationRuntimeError::Participation(_) => StatusCode::UNPROCESSABLE_ENTITY,

        QuorumParticipationRuntimeError::AlreadyConfigured => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (
        status,
        Json(json!({
            "version": 1,
            "status": "rejected",
            "error": error.to_string(),

            "serviceNodeSignatureCreated": false,

            "quorumAggregated": false,
            "quorumFinalized": false,
            "checkpointFinalized": false,

            "walletMutation": false,
            "ledgerMutation": false,
            "payoutExecuted": false,
            "receiptCreated": false,
            "confirmedRocReported": false,
            "paidUnlock": false,

            "finality": false,
            "crabLinkFinalityAuthority": false
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use axum::{body::to_bytes, response::IntoResponse};
    use ron_kms::{memory_keystore, Keystore, Signer};
    use ron_proto::{
        ContentId, EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1,
        EpochRewardAllocationV1, RocEpochTransitionExpectationV1, EPOCH_REWARD_ALLOCATION_SCHEMA,
        ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA, ROC_EPOCH_TRANSITION_VERSION,
    };
    use serde_json::Value;

    use super::*;

    use crate::{
        bench::BenchManager,
        bus::NodeBus,
        config::Config,
        readiness::ReadyProbes,
        services::quorum_participation::{
            QuorumParticipationRuntime, ServiceNodeQuorumParticipant,
        },
        types::{OperatorState, RuntimeStatus},
    };

    const CHAIN_ID: &str = "rustyonions-dev";
    const EPOCH_ID: &str = "epoch:19";

    const SERVICE_NODE_ID: &str = "service_node:alpha";

    const LOGICAL_KEY_REF: &str = "key:phase19:alpha";

    fn cid(character: char) -> ContentId {
        format!("b3:{}", character.to_string().repeat(64))
            .parse()
            .expect("fixture content id must parse")
    }

    fn eligibility(service_node_id: &str, key_id: &str) -> EpochEligibilityV1 {
        EpochEligibilityV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,

            service_node_id: service_node_id.to_owned(),

            registry_entry_id: format!("registry:{service_node_id}"),

            reward_binding_id: format!("binding:{service_node_id}"),

            key_id: key_id.to_owned(),

            status: EpochEligibilityStatusV1::Eligible,
        }
    }

    fn threshold() -> EpochQuorumThresholdV1 {
        EpochQuorumThresholdV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,

            eligible_service_nodes: 3,

            quorum_bps: 6_666,

            minimum_signatures: 2,

            required_signatures: 2,
        }
    }

    fn expectation() -> RocEpochTransitionExpectationV1 {
        RocEpochTransitionExpectationV1 {
            schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),

            version: ROC_EPOCH_TRANSITION_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            epoch_id: EPOCH_ID.to_owned(),

            accounting_snapshot_hash: cid('a'),

            reward_plan_hash: cid('b'),

            policy_hash: cid('c'),

            economics_config_hash: cid('d'),

            registry_root: cid('e'),

            reward_binding_root: cid('f'),

            evidence_root: cid('1'),

            reward_cap_minor_units: "1000".to_owned(),

            threshold: threshold(),

            eligibilities: vec![
                eligibility(SERVICE_NODE_ID, LOGICAL_KEY_REF),
                eligibility("service_node:beta", "key:phase19:beta"),
                eligibility("service_node:gamma", "key:phase19:gamma"),
            ],
        }
    }

    fn allocation(
        allocation_id: &str,
        reward_plan_allocation_id: &str,
        service_node_id: &str,
        amount_minor_units: &str,
    ) -> EpochRewardAllocationV1 {
        EpochRewardAllocationV1 {
            schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),

            version: ROC_EPOCH_TRANSITION_VERSION,

            allocation_id: allocation_id.to_owned(),

            reward_plan_allocation_id: reward_plan_allocation_id.to_owned(),

            service_node_id: service_node_id.to_owned(),

            source_pool: "node_delivery".to_owned(),

            amount_minor_units: amount_minor_units.to_owned(),
        }
    }

    fn identity() -> RocEpochTransitionIdentityV1 {
        RocEpochTransitionIdentityV1::from_expectation_and_allocations(
            &expectation(),
            "1000",
            vec![
                allocation(
                    "allocation:alpha",
                    "reward_plan_allocation:alpha",
                    SERVICE_NODE_ID,
                    "400",
                ),
                allocation(
                    "allocation:beta",
                    "reward_plan_allocation:beta",
                    "service_node:beta",
                    "600",
                ),
            ],
        )
    }

    fn app_state(runtime: Arc<RuntimeStatus>) -> AppState {
        AppState {
            cfg: Arc::new(Config::default()),

            probes: Arc::new(ReadyProbes::new()),

            runtime,

            bus: NodeBus::new(),

            started_at: Instant::now(),

            bench: Arc::new(BenchManager::new("http://127.0.0.1:1".to_string())),

            operator: Arc::new(OperatorState::new(
                false,
                "http://127.0.0.1:5300".to_string(),
                Duration::from_secs(15 * 60),
            )),
        }
    }

    async fn response_json(response: Response) -> Value {
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("bounded quorum response body");

        serde_json::from_slice(&body).expect("quorum response JSON")
    }

    fn configured_runtime() -> Arc<RuntimeStatus> {
        let runtime = Arc::new(RuntimeStatus::new());

        let kms = Arc::new(memory_keystore());

        let key = kms
            .create_ed25519("service-node", "phase19-http-alpha")
            .expect("fixture quorum key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, SERVICE_NODE_ID, LOGICAL_KEY_REF, key);

        let signer: Arc<dyn Signer> = kms;

        runtime
            .register_quorum_participant(Arc::new(QuorumParticipationRuntime::new(
                participant,
                signer,
            )))
            .expect("fixture participant must register");

        runtime
    }

    #[tokio::test]
    async fn unconfigured_runtime_fails_closed_without_signature_or_authority() {
        let runtime = Arc::new(RuntimeStatus::new());

        let response = sign(State(app_state(runtime)), Json(identity()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response_json(response).await;

        assert_eq!(body["status"], "rejected");

        assert_eq!(body["serviceNodeSignatureCreated"], false);

        assert!(body.get("signature").is_none());

        assert_no_authority_claims(&body);
    }

    #[tokio::test]
    async fn configured_runtime_returns_exactly_one_service_node_signature() {
        let response = sign(State(app_state(configured_runtime())), Json(identity()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["status"], "signed");

        assert_eq!(body["serviceNodeSignatureCreated"], true);

        assert_eq!(body["signature"]["service_node_id"], SERVICE_NODE_ID);

        assert_eq!(body["signature"]["key_id"], LOGICAL_KEY_REF);

        assert_eq!(body["signature"]["chain_id"], CHAIN_ID);

        assert_eq!(body["signature"]["epoch_id"], EPOCH_ID);

        assert_eq!(
            body["signature"]["signature_wire"].as_str().map(str::len),
            Some(128)
        );

        assert_no_authority_claims(&body);
    }

    #[tokio::test]
    async fn invalid_identity_rejects_without_signature() {
        let mut invalid = identity();

        invalid.domain = "rustyonions.invalid-transition.v1".to_owned();

        let response = sign(State(app_state(configured_runtime())), Json(invalid))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = response_json(response).await;

        assert_eq!(body["status"], "rejected");

        assert_eq!(body["serviceNodeSignatureCreated"], false);

        assert!(body.get("signature").is_none());

        assert_no_authority_claims(&body);
    }

    fn assert_no_authority_claims(body: &Value) {
        for field in [
            "quorumAggregated",
            "quorumFinalized",
            "checkpointFinalized",
            "walletMutation",
            "ledgerMutation",
            "payoutExecuted",
            "receiptCreated",
            "confirmedRocReported",
            "paidUnlock",
            "finality",
            "crabLinkFinalityAuthority",
        ] {
            assert_eq!(
                body[field], false,
                "quorum-sign response must not claim authority in {field}"
            );
        }
    }
}
