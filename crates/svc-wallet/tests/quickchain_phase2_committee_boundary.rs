#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 2 Round 2 committee-readiness boundary tests for svc-wallet.
//! RO:WHY — svc-wallet may emit backend-derived wallet receipt evidence, but it must not become a committee member, attestation signer, quorum authority, fork-choice authority, or settlement layer.
//! RO:INTERACTS — wallet receipt projection, live wallet mutation routes, svc-wallet source boundary.
//! RO:INVARIANTS — wallet remains mutation front-door only; attestations/quorum/finality are not accepted from clients or produced by wallet.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — rejects Phase 2 Round 2 committee/quorum poison fields and prevents validator-economy creep.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase2_committee_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
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

const COMMITTEE_AUTHORITY_KEYS: &[&str] = &[
    "committee_member_id",
    "committee_epoch",
    "committee_round",
    "committee_signature",
    "committee_signatures",
    "signed_verification_attestation",
    "verification_attestation",
    "attestation_signature",
    "attestation_public_key",
    "attestation_weight",
    "quorum_certificate",
    "quorum_threshold",
    "quorum_reached",
    "validator_signature",
    "validator_set",
    "validator_index",
    "fork_choice",
    "double_attestation_evidence",
    "equivocation_evidence",
    "bonded_stake",
    "stake_weight",
    "slash_evidence",
    "slashing",
    "external_anchor",
    "external_settlement",
    "bridge_finality",
    "settlement_finality",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "target")
        {
            continue;
        }

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn app() -> Router {
    let state = WalletState::dev().expect("dev wallet state should build");
    routes::router(state)
}

fn json_post_request(path: &str, idempotency_key: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(
            serde_json::to_vec(&body).expect("JSON body should encode"),
        ))
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

fn dummy_wallet_receipt_for_committee_boundary() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_phase2_round2_committee_wallet_receipt".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_phase2_round2_alice".to_string()),
        to: Some("acct_phase2_round2_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(77),
        nonce: Some(9),
        idem: "idem_phase2_round2_wallet_receipt".to_string(),
        ts: 1_777_500_000_000,
        ledger_seq_start: Some(200),
        ledger_seq_end: Some(201),
        ledger_root: "22".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy wallet receipt should hash")
}

fn assert_no_key_recursive(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    key != forbidden,
                    "svc-wallet projection must not expose Phase 2 Round 2 committee authority key `{forbidden}`"
                );
                assert_no_key_recursive(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key_recursive(nested, forbidden);
            }
        }
        _ => {}
    }
}

#[test]
fn wallet_receipt_projection_has_no_committee_attestation_or_quorum_authority() {
    let receipt = dummy_wallet_receipt_for_committee_boundary();

    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "ron-devnet",
        "op:wallet:transfer:phase2-round2-committee-boundary",
    )
    .expect("explicit wallet projection context should validate");

    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("backend-derived wallet receipt should project");

    assert_eq!(
        projection.settlement_status,
        QuickChainWalletReceiptStatus::Accepted
    );
    assert_eq!(
        projection.operation_id,
        "op:wallet:transfer:phase2-round2-committee-boundary"
    );

    let projection_json =
        serde_json::to_value(&projection).expect("wallet projection should serialize");

    for forbidden in COMMITTEE_AUTHORITY_KEYS {
        assert_no_key_recursive(&projection_json, forbidden);
    }

    let encoded = serde_json::to_string(&projection).expect("wallet projection should encode");
    for forbidden in [
        "quorum_certificate",
        "committee_signature",
        "signed_verification_attestation",
        "fork_choice",
        "bonded_stake",
        "slash_evidence",
        "external_settlement",
        "bridge_finality",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "wallet projection JSON must not contain Phase 2 Round 2 committee/finality authority vocabulary: {forbidden}"
        );
    }
}

#[tokio::test]
async fn wallet_mutation_routes_reject_committee_attestation_poison_fields() {
    let poisoned_issue_body = json!({
        "to": "acct_phase2_round2_committee_poison",
        "asset": "roc",
        "amount_minor": "13",
        "memo": null,
        "committee_member_id": "validator-alpha",
        "committee_epoch": "epoch_0003",
        "committee_round": 1,
        "signed_verification_attestation": {
            "schema": "quickchain.committee-attestation.v1",
            "signature": "client-must-not-supply"
        },
        "quorum_certificate": "client-must-not-supply",
        "quorum_threshold": 2,
        "quorum_reached": true,
        "validator_signature": "client-must-not-supply",
        "fork_choice": "client-must-not-supply",
        "bonded_stake": "1000000",
        "slash_evidence": "client-must-not-supply",
        "settlement_finality": "client-must-not-supply",
        "external_settlement": "client-must-not-supply"
    });

    let (status, _body) = send(
        app(),
        json_post_request(
            "/v1/issue",
            "idem_phase2_round2_wallet_committee_poison_reject",
            poisoned_issue_body,
        ),
    )
    .await;

    assert!(
        status.is_client_error(),
        "svc-wallet live mutation routes must reject client-supplied Phase 2 Round 2 committee/quorum authority fields"
    );
    assert_ne!(status, StatusCode::OK);
}

#[test]
fn wallet_source_does_not_implement_committee_or_validator_economy_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainCommittee",
            "CommitteeAttestation",
            "SignedVerificationAttestation",
            "QuorumCertificate",
            "committee_member_id",
            "committee_epoch",
            "committee_round",
            "signed_verification_attestation",
            "verification_attestation",
            "attestation_signature",
            "quorum_certificate",
            "quorum_threshold",
            "quorum_reached",
            "validator_set",
            "validator_signature",
            "fork_choice",
            "double_attestation_evidence",
            "equivocation_evidence",
            "bonded_stake",
            "stake_weight",
            "slash_evidence",
            "bridge_finality",
            "settlement_finality",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not implement Phase 2 Round 2 committee/validator-economy authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
