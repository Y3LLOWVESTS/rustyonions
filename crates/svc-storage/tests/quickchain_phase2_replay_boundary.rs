//! RO:WHAT — Phase 2 Round 1 verifier-artifact boundary tests for svc-storage.
//! RO:WHY — Confirms storage may retain/read replay bundles by b3 without becoming verifier/finality authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, storage::MemoryStorage, paid/accounting storage paths.
//! RO:INVARIANTS — b3 proves bytes only; storage has no mutation authority over replay outcome or paid unlock.
//! RO:METRICS — none; functional storage and source/docs boundary tests.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks quorum, fork-choice, validator-signature, bridge, staking, and settlement creep.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase2_replay_boundary.

use std::{fs, path::PathBuf};

use axum::body::Bytes;
use svc_storage::storage::{MemoryStorage, Storage};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = crate_dir().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
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

fn read_sources(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n")
}

fn cid_for(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn assert_canonical_b3(value: &str) {
    let Some(hex) = value.strip_prefix("b3:") else {
        panic!("cid must start with b3: {value}");
    };

    assert_eq!(hex.len(), 64, "cid must carry a 64-nybble BLAKE3 digest");
    assert!(
        hex.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "cid must be lowercase hex: {value}"
    );
}

fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context} must preserve required marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, context: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{context} must not contain forbidden authority marker: {forbidden}"
    );
}

#[test]
fn docs_name_phase2_round1_storage_verifier_artifact_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 1 verifier artifact / read-only replication",
        "storage may retain read-only verifier artifact bytes by canonical b3",
        "artifact cids are byte references, not verifier authority",
        "b3 proves bytes, not balance truth",
        "storage cannot decide quorum",
        "storage cannot sign committee votes",
        "storage cannot claim fork choice",
        "storage cannot claim finality",
        "storage cannot unlock paid content from cache alone",
        "wallet/ledger receipts remain backend truth",
        "quickchain_phase2_replay_boundary",
    ] {
        assert_contains(&doc, required, "svc-storage quickchain-preflight.md");
    }
}

#[tokio::test]
async fn storage_can_archive_and_retrieve_read_only_verifier_artifacts_by_b3() {
    let store = MemoryStorage::new();

    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.replay-bundle.v1","purpose":"read_only_verifier_input"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.replay-result.v1","purpose":"read_only_verifier_output"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.verifier-diagnostic.v1","purpose":"mismatch_report_only"}"#,
        ),
    ];

    for body in artifacts {
        let cid = cid_for(body.as_ref());
        assert_canonical_b3(&cid);

        store
            .put(&cid, body.clone())
            .await
            .expect("read-only verifier artifact bytes should store by b3 cid");

        assert!(
            store
                .exists(&cid)
                .await
                .expect("exists check should succeed"),
            "stored verifier artifact should be discoverable by exact b3 cid"
        );

        let head = store.head(&cid).await.expect("head should succeed");
        assert_eq!(head.len, body.len() as u64);
        assert_eq!(
            head.etag,
            format!("\"{}\"", blake3::hash(body.as_ref()).to_hex())
        );

        let full = store
            .get_full(&cid)
            .await
            .expect("full verifier artifact read should succeed");
        assert_eq!(full, body);

        let (prefix, total) = store
            .get_range(&cid, 0, 3)
            .await
            .expect("range verifier artifact read should succeed");
        assert_eq!(total, body.len() as u64);
        assert_eq!(&prefix[..], &body[..4]);
    }
}

#[test]
fn storage_replay_artifact_paths_do_not_become_verifier_or_paid_unlock_authority() {
    let source = strip_line_comments(&read_sources(&[
        "src/storage/cas.rs",
        "src/storage/mod.rs",
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/paid_object.rs",
        "src/policy/settlement.rs",
        "src/accounting/exporter.rs",
    ]));
    let lower = source.to_ascii_lowercase();

    for required in [
        "starts_with(\"b3:\")",
        "3 + 64",
        "b'a'..=b'f'",
        "MemoryStorage",
        "get_full",
        "get_range",
        "WalletSettlementHttpClient",
        "settle_paid_storage",
        "AccountingExportRequest",
        "export_usage_events",
    ] {
        assert_contains(
            &source,
            required,
            "svc-storage byte/artifact/paid-accounting source",
        );
    }

    for forbidden in [
        "ron_ledger::",
        "svc_wallet::",
        "ron_proto::quickchain",
        "committee",
        "quorum",
        "fork_choice",
        "validator_signature",
        "validator_set",
        "checkpoint_signature",
        "finality_vote",
        "root_producer",
        "anchor_receipt",
        "anchor_root",
        "bridge_settled",
        "solana",
        "rox",
        "staking",
        "liquidity",
        "settlement_finality",
    ] {
        assert_not_contains(
            &lower,
            forbidden,
            "svc-storage read-only replay artifact boundary source",
        );
    }
}
