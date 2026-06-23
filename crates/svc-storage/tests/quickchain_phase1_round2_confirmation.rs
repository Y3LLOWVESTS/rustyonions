//! RO:WHAT — Phase 1 Round 2 downstream-confirmation tests for svc-storage.
//! RO:WHY — Confirms storage can store/retrieve QuickChain artifacts by b3 while remaining byte infrastructure only.
//! RO:INTERACTS — docs/quickchain-preflight.md, storage::MemoryStorage, policy::{paid_write,settlement}, accounting exporter.
//! RO:INVARIANTS — b3 proves bytes only; storage cannot mutate balances, unlock paid content alone, or claim finality.
//! RO:METRICS — none; functional storage and source/docs boundary tests.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents vector/root/proof artifact storage from becoming settlement authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase1_round2_confirmation.

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
fn docs_name_phase1_round2_storage_artifact_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 1 round 2 downstream confirmation",
        "storage can store and retrieve vector/root/proof artifacts by b3",
        "artifact cids are byte references, not quickchain roots",
        "storage cannot mutate balances",
        "storage cannot unlock paid content from cache alone",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase1_round2_confirmation",
    ] {
        assert_contains(&doc, required, "svc-storage quickchain-preflight.md");
    }
}

#[tokio::test]
async fn storage_can_store_and_retrieve_phase1_artifacts_by_canonical_b3() {
    let store = MemoryStorage::new();

    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.vector.locked_bytes.v1","kind":"reward_manifest_artifact"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.proof_artifact.v1","kind":"receipt_inclusion_fixture"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.root_artifact.v1","kind":"state_root_fixture_bytes"}"#,
        ),
    ];

    for body in artifacts {
        let cid = cid_for(body.as_ref());
        assert_canonical_b3(&cid);

        store
            .put(&cid, body.clone())
            .await
            .expect("artifact bytes should store by b3 cid");

        assert!(
            store
                .exists(&cid)
                .await
                .expect("exists check should succeed"),
            "stored artifact should be discoverable by exact b3 cid"
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

        if body.len() > 4 {
            let (range, total) = store
                .get_range(&cid, 0, 3)
                .await
                .expect("range artifact read should succeed");
            assert_eq!(total, body.len() as u64);
            assert_eq!(&range[..], &body[..4]);
        }
    }
}

#[test]
fn paid_and_accounting_storage_sources_do_not_turn_artifacts_into_unlock_or_finality_authority() {
    let source = strip_line_comments(&read_sources(&[
        "src/policy/paid_write.rs",
        "src/policy/settlement.rs",
        "src/accounting/mod.rs",
        "src/accounting/exporter.rs",
        "src/http/routes/paid_object.rs",
        "src/storage/mod.rs",
    ]));

    for required in [
        "WalletReceipt",
        "PaidWriteProof",
        "validate_as_paid_write_hold",
        "wallet receipt hash must be b3:<64 lowercase hex>",
        "paid proof must reference a wallet hold receipt",
        "PaidStorageSettlementPlan",
        "WalletSettlementHttpClient",
        "AccountingExportReport",
        "UsageEventDto",
        "MemoryStorage",
        "get_full",
        "get_range",
    ] {
        assert_contains(
            &source,
            required,
            "svc-storage paid/accounting/storage source",
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "wallet_issue_request",
        "wallet_transfer_request",
        "approved_payout",
        "execute_payout",
        "rewarder_decision",
        "payout_plan",
        "payout_receipt",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "reward_root",
        "checkpoint_root",
        "checkpoint_hash",
        "validator_signature",
        "validator_set",
        "settlement_finality",
        "external_anchor",
        "bridge_txid",
        "solana",
        "rox",
        "staking",
        "liquidity",
    ] {
        assert_not_contains(
            &source,
            forbidden,
            "svc-storage paid/accounting/storage source",
        );
    }
}

#[test]
fn object_routes_and_store_remain_content_addressed_byte_paths_not_chain_authority() {
    let source = strip_line_comments(&read_sources(&[
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/put_object.rs",
        "src/storage/mod.rs",
        "src/storage/fs.rs",
    ]));

    for required in [
        r#"starts_with("b3:")"#,
        "3 + 64",
        "b'a'..=b'f'",
        "is_valid_cid",
        "blake3::hash",
        "ETAG",
        "CONTENT_RANGE",
        "put",
        "head",
        "get_full",
        "get_range",
    ] {
        assert_contains(&source, required, "svc-storage object/store source");
    }

    for forbidden in [
        "paid_unlock_from_cache",
        "unlock_without_wallet",
        "mint_from_storage",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "reward_root",
        "checkpoint_root",
        "checkpoint_hash",
        "validator_signature",
        "settlement_finality",
        "external_anchor",
        "bridge_txid",
    ] {
        assert_not_contains(&source, forbidden, "svc-storage object/store source");
    }
}
