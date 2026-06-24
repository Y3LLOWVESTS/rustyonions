#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 2 Round 1 boundary tests for svc-wallet and read-only verifier replay artifacts.
//! RO:WHY — svc-wallet may provide backend-derived receipt evidence, but it must not become verifier, finality, committee, or settlement authority.
//! RO:INTERACTS — svc_wallet::quickchain projection, ron-ledger read-only replay verifier, ron-proto verifier replay DTOs.
//! RO:INVARIANTS — wallet receipts stay accepted-only evidence; verifier replay results are diagnostic only; no quorum, signing, bridge, staking, or external settlement.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — client-supplied verifier/committee fields are rejected; wallet projection never grants spend/finality authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase2_replay_boundary.

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use ron_ledger::quickchain::{
    build_tree_material_batch, compute_tree_root_from_batch, verify_replay_bundle_read_only,
    QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        QuickChainTreeMaterialKindV1, QuickChainVerifierCheckStatusV1,
        QuickChainVerifierReplayBundleV1, QuickChainVerifierReplayStatusV1, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
        QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1,
        QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA,
    },
    ContentId,
};
use serde_json::{json, Value};
use svc_wallet::{
    dto::{
        requests::AmountMinor,
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjectionContext,
        QuickChainWalletReceiptStatus,
    },
    routes::{self, WalletState},
    util::blake3_receipt::finalize_receipt,
};
use tower::ServiceExt;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0002";
const RECEIPT_SORT_KEY_HEX: &str = "726563656970743a3030303030303031";

fn app() -> Router {
    let state = WalletState::dev().expect("dev wallet state should build");
    routes::router(state)
}

fn json_post_request(path: &str, idempotency_key: &str, body: Value) -> Request<Body> {
    let encoded = serde_json::to_vec(&body).expect("JSON body should encode");

    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(encoded))
        .expect("POST request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let body = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("response body should read")
        .to_vec();

    (status, body)
}

fn dummy_wallet_receipt_for_phase2_replay() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_phase2_replay_wallet_receipt".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_phase2_alice".to_string()),
        to: Some("acct_phase2_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(42),
        nonce: Some(7),
        idem: "idem_phase2_wallet_receipt".to_string(),
        ts: 1_777_400_000_000,
        ledger_seq_start: Some(100),
        ledger_seq_end: Some(101),
        ledger_root: "11".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy wallet receipt should hash")
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "svc-wallet projection must not expose Phase 2 verifier authority key `{forbidden}`"
                );
                assert_no_key(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key(nested, forbidden);
            }
        }
        _ => {}
    }
}

#[test]
fn wallet_receipt_evidence_can_be_read_only_replayed_without_wallet_becoming_verifier() {
    let receipt = dummy_wallet_receipt_for_phase2_replay();

    let context = QuickChainWalletReceiptProjectionContext::accepted(
        CHAIN_ID,
        "op:wallet:transfer:phase2-replay-boundary",
    )
    .expect("explicit wallet projection context should validate");

    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("backend-derived wallet receipt should project");

    assert_eq!(
        projection.settlement_status,
        QuickChainWalletReceiptStatus::Accepted
    );

    let receipt_hash: ContentId = projection
        .receipt_hash
        .parse()
        .expect("wallet receipt hash must be canonical b3 ContentId");

    let material = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::Receipts,
        vec![QuickChainTreeMaterialProjectionItem::new(
            RECEIPT_SORT_KEY_HEX,
            QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
            receipt_hash,
        )],
    )
    .expect("receipt material should build from wallet receipt evidence");

    let expected_root =
        compute_tree_root_from_batch(&material).expect("receipt root should compute in ron-ledger");

    let bundle = QuickChainVerifierReplayBundleV1 {
        schema: QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        replay_algorithm: QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1.to_string(),
        material_batches: vec![material],
        expected_roots: vec![expected_root],
        inclusion_proofs: Vec::new(),
    };

    bundle
        .validate()
        .expect("read-only verifier replay bundle should validate");

    let first = verify_replay_bundle_read_only(&bundle)
        .expect("first read-only verifier replay should succeed");
    let second = verify_replay_bundle_read_only(&bundle)
        .expect("second read-only verifier replay should succeed");

    assert_eq!(first, second);
    assert_eq!(first.status, QuickChainVerifierReplayStatusV1::Verified);
    assert_eq!(first.root_checks.len(), 1);
    assert_eq!(
        first.root_checks[0].status,
        QuickChainVerifierCheckStatusV1::Verified
    );
    assert_eq!(first.proof_checks.len(), 0);

    let projection_json =
        serde_json::to_value(&projection).expect("wallet projection should serialize");

    for forbidden in [
        "replay_algorithm",
        "material_batches",
        "expected_roots",
        "inclusion_proofs",
        "root_checks",
        "proof_checks",
        "quorum_certificate",
        "committee_signature",
        "validator_signature",
        "fork_choice",
        "finality",
        "bridge",
        "external_settlement",
    ] {
        assert_no_key(&projection_json, forbidden);
    }
}

#[tokio::test]
async fn wallet_live_routes_reject_phase2_verifier_authority_poison_fields() {
    let poisoned_issue_body = json!({
        "to": "acct_phase2_poison",
        "asset": "roc",
        "amount_minor": "9",
        "memo": null,
        "replay_algorithm": "read_only_root_and_proof_replay_v1",
        "material_batches": [],
        "expected_roots": [],
        "root_checks": [],
        "proof_checks": [],
        "quorum_certificate": "client-must-not-supply",
        "committee_signature": "client-must-not-supply",
        "fork_choice": "client-must-not-supply",
        "finality": "client-must-not-supply"
    });

    let (status, _body) = send(
        app(),
        json_post_request(
            "/v1/issue",
            "idem_phase2_wallet_poison_reject",
            poisoned_issue_body,
        ),
    )
    .await;

    assert!(
        status.is_client_error(),
        "svc-wallet live mutation routes must reject client-supplied Phase 2 verifier authority fields"
    );
    assert_ne!(status, StatusCode::OK);
}
