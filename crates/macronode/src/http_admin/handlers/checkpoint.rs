//! RO:WHAT — Guarded HTTP adapter for exactly one checkpoint-validator signature.
//! RO:WHY — FINAL_BETA Phase 19 live multi-process proof needs each validator process to expose its own independent signature.
//! RO:INTERACTS — AppState::runtime and QuickChain checkpoint-validator signing contract.
//! RO:INVARIANTS — caller supplies only height + candidate hash; validator chain/epoch/identity/key remain process-bound.
//! RO:METRICS — existing admin HTTP accounting only.
//! RO:CONFIG — private-beta validator runtime must already be registered.
//! RO:SECURITY — admin guarded; no committee aggregation, threshold decision, checkpoint finalization, wallet/ledger mutation, or CrabLink authority.
//! RO:TEST — focused handler tests below plus auth middleware tests.

#![forbid(unsafe_code)]

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ron_proto::ContentId;
use serde::Deserialize;
use serde_json::json;

use crate::{
    services::checkpoint_validator_signing::CheckpointValidatorSigningRuntimeError, types::AppState,
};

/// Exact request for one process-local checkpoint-validator signature.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointSignRequest {
    /// Non-zero checkpoint height.
    height: u64,

    /// Deterministic unsigned checkpoint candidate hash.
    checkpoint_hash: ContentId,
}

/// POST /api/v1/quickchain/checkpoint/sign
///
/// The process-bound validator identity supplies chain, epoch, validator ID,
/// logical key ID, and signature algorithm. The caller cannot substitute
/// those bindings.
pub async fn sign(
    State(state): State<AppState>,
    Json(request): Json<CheckpointSignRequest>,
) -> Response {
    match state
        .runtime
        .sign_checkpoint_validator(request.height, &request.checkpoint_hash)
    {
        Ok(signature) => (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "status": "signed",
                "signature": signature,

                "checkpointValidatorSignatureCreated": true,

                "committeeAggregated": false,
                "committeeThresholdSatisfied": false,
                "checkpointFinalized": false,

                "walletMutation": false,
                "ledgerMutation": false,
                "payoutExecuted": false,
                "receiptCreated": false,
                "bridgeSettled": false,

                "finality": false,
                "crabLinkFinalityAuthority": false
            })),
        )
            .into_response(),

        Err(error) => error_response(error),
    }
}

fn error_response(error: CheckpointValidatorSigningRuntimeError) -> Response {
    let status = match &error {
        CheckpointValidatorSigningRuntimeError::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,

        CheckpointValidatorSigningRuntimeError::Signing(_) => StatusCode::UNPROCESSABLE_ENTITY,

        CheckpointValidatorSigningRuntimeError::AlreadyConfigured => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };

    (
        status,
        Json(json!({
            "version": 1,
            "status": "rejected",
            "error": error.to_string(),

            "checkpointValidatorSignatureCreated": false,

            "committeeAggregated": false,
            "committeeThresholdSatisfied": false,
            "checkpointFinalized": false,

            "walletMutation": false,
            "ledgerMutation": false,
            "payoutExecuted": false,
            "receiptCreated": false,
            "bridgeSettled": false,

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
        quantum::SignatureAlg,
        quickchain::{
            QuickChainValidatorIdentityV1, QuickChainValidatorLifecycleStatusV1,
            QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        },
    };
    use serde_json::Value;

    use super::*;

    use crate::{
        bench::BenchManager,
        bus::NodeBus,
        config::Config,
        readiness::ReadyProbes,
        services::checkpoint_validator_signing::{
            CheckpointValidatorParticipant, CheckpointValidatorSigningRuntime,
        },
        types::{OperatorState, RuntimeStatus},
    };

    const CHAIN_ID: &str = "ron-devnet";

    const EPOCH_ID: &str = "epoch_phase19_checkpoint_http";

    fn cid(character: char) -> ContentId {
        format!("b3:{}", character.to_string().repeat(64),)
            .parse()
            .expect("fixture content id must parse")
    }

    fn identity() -> QuickChainValidatorIdentityV1 {
        QuickChainValidatorIdentityV1 {
            schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            epoch_id: EPOCH_ID.to_owned(),

            validator_id: "validator-http-alpha".to_owned(),

            passport_subject: "@validator-http-alpha".to_owned(),

            registry_entry_id: "registry:validator-http-alpha".to_owned(),

            key_id: "key:validator-http-alpha:001".to_owned(),

            capability_id: "cap:validator-http-alpha:verify:001".to_owned(),

            signature_algorithm: SignatureAlg::Ed25519,

            lifecycle_status: QuickChainValidatorLifecycleStatusV1::Active,

            not_before_ms: 1_800_000_000_000,

            expires_at_ms: 1_800_086_400_000,
        }
    }

    fn app_state(runtime: Arc<RuntimeStatus>) -> AppState {
        AppState {
            cfg: Arc::new(Config::default()),

            probes: Arc::new(ReadyProbes::new()),

            runtime,

            bus: NodeBus::new(),

            started_at: Instant::now(),

            bench: Arc::new(BenchManager::new("http://127.0.0.1:1".to_owned())),

            operator: Arc::new(OperatorState::new(
                false,
                "http://127.0.0.1:5300".to_owned(),
                Duration::from_secs(15 * 60),
            )),
        }
    }

    fn configured_runtime() -> Arc<RuntimeStatus> {
        let runtime = Arc::new(RuntimeStatus::new());

        let kms = Arc::new(memory_keystore());

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-http-alpha")
            .expect("fixture checkpoint validator key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(&identity(), key)
            .expect("fixture checkpoint validator participant must build");

        let signer: Arc<dyn Signer> = kms;

        runtime
            .register_checkpoint_validator_signer(Arc::new(CheckpointValidatorSigningRuntime::new(
                participant,
                signer,
            )))
            .expect("fixture checkpoint validator runtime must register");

        runtime
    }

    async fn response_json(response: Response) -> Value {
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("bounded checkpoint-sign response");

        serde_json::from_slice(&body).expect("checkpoint-sign response JSON")
    }

    fn assert_no_authority_claims(body: &Value) {
        assert_eq!(body["committeeAggregated"], false,);

        assert_eq!(body["committeeThresholdSatisfied"], false,);

        assert_eq!(body["checkpointFinalized"], false,);

        assert_eq!(body["walletMutation"], false,);

        assert_eq!(body["ledgerMutation"], false,);

        assert_eq!(body["finality"], false,);

        assert_eq!(body["crabLinkFinalityAuthority"], false,);
    }

    #[tokio::test]
    async fn unconfigured_checkpoint_runtime_fails_closed() {
        let response = sign(
            State(app_state(Arc::new(RuntimeStatus::new()))),
            Json(CheckpointSignRequest {
                height: 19,
                checkpoint_hash: cid('a'),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE,);

        let body = response_json(response).await;

        assert_eq!(body["status"], "rejected",);

        assert_eq!(body["checkpointValidatorSignatureCreated"], false,);

        assert!(body.get("signature",).is_none(),);

        assert_no_authority_claims(&body);
    }

    #[tokio::test]
    async fn configured_checkpoint_runtime_returns_exactly_one_signature() {
        let checkpoint_hash = cid('b');

        let response = sign(
            State(app_state(configured_runtime())),
            Json(CheckpointSignRequest {
                height: 19,
                checkpoint_hash: checkpoint_hash.clone(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK,);

        let body = response_json(response).await;

        assert_eq!(body["status"], "signed",);

        assert_eq!(body["checkpointValidatorSignatureCreated"], true,);

        assert_eq!(body["signature"]["validator_id"], "validator-http-alpha",);

        assert_eq!(body["signature"]["key_id"], "key:validator-http-alpha:001",);

        assert_eq!(body["signature"]["chain_id"], CHAIN_ID,);

        assert_eq!(body["signature"]["epoch_id"], EPOCH_ID,);

        assert_eq!(
            body["signature"]["checkpoint_hash"],
            checkpoint_hash.to_string(),
        );

        assert_eq!(
            body["signature"]["signature_wire"].as_str().map(str::len),
            Some(128),
        );

        assert_no_authority_claims(&body);
    }

    #[tokio::test]
    async fn zero_height_rejects_without_signature_or_finality_claim() {
        let response = sign(
            State(app_state(configured_runtime())),
            Json(CheckpointSignRequest {
                height: 0,
                checkpoint_hash: cid('c'),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY,);

        let body = response_json(response).await;

        assert_eq!(body["checkpointValidatorSignatureCreated"], false,);

        assert!(body.get("signature",).is_none(),);

        assert_no_authority_claims(&body);
    }
}
