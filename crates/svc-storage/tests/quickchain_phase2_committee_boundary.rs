//! RO:WHAT — Phase 2 Round 2 committee-readiness boundary tests for svc-storage.
//! RO:WHY — Storage may retain/retrieve replay and committee-readiness artifact bytes, but it must not become committee, quorum, fork-choice, finality, settlement, paid-unlock, or validator-economy authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, storage::MemoryStorage, storage/http/policy/accounting source boundary.
//! RO:INVARIANTS — b3 proves bytes only; storage has no mutation authority over replay outcome, committee agreement, or paid unlock.
//! RO:METRICS — none; functional storage and source/docs boundary tests.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — blocks quorum, fork-choice, validator-signature, bridge, staking, and settlement creep.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase2_committee_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use svc_storage::storage::{MemoryStorage, Storage};

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

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
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

#[test]
fn docs_name_phase2_round2_committee_readiness_boundary() {
    let doc = normalized(&read(crate_dir().join("docs/quickchain-preflight.md")));

    for required in [
        "phase 2 round 2 committee readiness boundary",
        "svc-storage stores committee/replay artifacts as bytes only",
        "storage is not a committee member",
        "storage does not produce signed verification attestations",
        "storage does not decide quorum",
        "storage does not claim fork choice",
        "storage does not claim finality",
        "b3 proves byte integrity, not committee agreement",
        "artifact cids are byte references, not verifier authority",
        "cache cannot unlock paid content alone",
        "wallet/ledger receipts remain backend truth",
        "quickchain_phase2_committee_boundary",
    ] {
        assert_contains(&doc, required, "svc-storage quickchain-preflight.md");
    }
}

#[tokio::test]
async fn storage_can_hold_committee_readiness_artifact_bytes_without_interpreting_authority() {
    let store = MemoryStorage::new();

    let body = Bytes::from_static(
        br#"{"schema":"quickchain.committee-readiness-artifact.v1","committee_member_id":"validator-alpha","signed_verification_attestation":"opaque-byte-payload-only","quorum_certificate":"opaque-byte-payload-only","purpose":"storage-byte-truth-only"}"#,
    );

    let cid = cid_for(body.as_ref());
    assert_canonical_b3(&cid);

    store
        .put(&cid, body.clone())
        .await
        .expect("committee-readiness artifact bytes should store by b3 cid");

    assert!(
        store
            .exists(&cid)
            .await
            .expect("exists check should succeed"),
        "stored committee-readiness artifact should be discoverable by exact b3 cid"
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
        .expect("full artifact read should succeed");
    assert_eq!(full, body);

    let (prefix, total) = store
        .get_range(&cid, 0, 7)
        .await
        .expect("range artifact read should succeed");
    assert_eq!(total, body.len() as u64);
    assert_eq!(&prefix[..], &body[..8]);
}

#[test]
fn storage_source_preserves_byte_storage_and_paid_boundary_seams() {
    let source = strip_line_comments(
        &[
            "src/storage/cas.rs",
            "src/storage/mod.rs",
            "src/http/routes/get_object.rs",
            "src/http/routes/head_object.rs",
            "src/http/routes/paid_object.rs",
            "src/policy/settlement.rs",
            "src/accounting/exporter.rs",
        ]
        .iter()
        .map(|relative| read(crate_dir().join(relative)))
        .collect::<Vec<_>>()
        .join("\n"),
    );

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
}

#[test]
fn storage_source_does_not_implement_committee_or_validator_economy_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
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
            "committee_signature",
            "committee_signatures",
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
            "slashing",
            "bridge_finality",
            "settlement_finality",
            "external_settlement",
            "cache_unlock_authority",
            "cache_only_unlock",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not implement Phase 2 Round 2 committee/validator-economy authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn storage_boundary_has_no_committee_authority_keys_in_paid_or_policy_runtime_source() {
    let paid_policy_source = strip_line_comments(
        &[
            "src/http/routes/paid_object.rs",
            "src/policy/settlement.rs",
            "src/accounting/exporter.rs",
        ]
        .iter()
        .map(|relative| read(crate_dir().join(relative)))
        .collect::<Vec<_>>()
        .join("\n"),
    );

    for forbidden in COMMITTEE_AUTHORITY_KEYS {
        assert!(
            !paid_policy_source.contains(forbidden),
            "svc-storage paid/policy/accounting runtime source must not accept Phase 2 Round 2 committee authority key `{forbidden}`"
        );
    }
}
