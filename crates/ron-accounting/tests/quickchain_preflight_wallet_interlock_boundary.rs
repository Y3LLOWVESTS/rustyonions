//! RO:WHAT — QuickChain preflight tests for the ron-accounting wallet/ledger interlock boundary.
//! RO:WHY — ron-accounting consumes derivative observations but must not depend on or mutate wallet/ledger truth.
//! RO:INTERACTS — Cargo manifest, src tree, UsageEvent, UsageEventsIngestRequest.
//! RO:INVARIANTS — no svc-wallet/ron-ledger dependency, no wallet mutation API, no ledger mutation API, strict poison rejection.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects wallet/ledger authority-looking ingest fields before accounting can treat them as facts.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_wallet_interlock_boundary.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    http_ingest::{UsageEventsIngestRequest, STORAGE_USAGE_EVENTS_SCHEMA},
    UsageEvent,
};
use serde_json::json;

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

#[test]
fn cargo_manifest_does_not_depend_on_wallet_or_ledger_authority_crates() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in ["svc-wallet", "svc_wallet", "ron-ledger", "ron_ledger"] {
        assert!(
            !manifest.contains(forbidden),
            "ron-accounting must not directly depend on wallet/ledger authority crate: {forbidden}"
        );
    }
}

#[test]
fn source_tree_has_no_direct_wallet_or_ledger_mutation_imports() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    for path in files {
        let text = read(&path);
        let code = strip_line_comments(&text);

        for forbidden in [
            "svc_wallet::",
            "ron_ledger::",
            "WalletClient",
            "LedgerClient",
            "WalletState",
            "LedgerState",
        ] {
            assert!(
                !code.contains(forbidden),
                "{} must not import wallet/ledger authority fragment: {forbidden}",
                path.display()
            );
        }

        for forbidden_fn in [
            "pub fn issue(",
            "pub fn transfer(",
            "pub fn burn(",
            "pub fn hold(",
            "pub fn capture(",
            "pub fn release(",
            "pub fn expire_hold(",
            "pub fn mint(",
            "pub fn mutate_ledger(",
            "pub fn mutate_wallet(",
        ] {
            assert!(
                !code.contains(forbidden_fn),
                "{} must not expose wallet/ledger mutation API: {forbidden_fn}",
                path.display()
            );
        }
    }
}

#[test]
fn top_level_ingest_rejects_wallet_and_ledger_authority_poison() {
    for field in [
        "wallet_balance",
        "wallet_receipt",
        "wallet_mutation",
        "ledger_commit",
        "ledger_sequence",
        "operation_id",
        "account_sequence",
        "settlement_status",
        "state_root",
        "receipt_root",
        "checkpoint_root",
    ] {
        let mut value = json!({
            "schema": STORAGE_USAGE_EVENTS_SCHEMA,
            "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "wallet_txid": "tx_wallet_observation_only",
            "source_service": "svc-storage",
            "events": []
        });

        value
            .as_object_mut()
            .expect("ingest request should be an object")
            .insert(field.to_string(), json!("client-supplied-authority"));

        let result = serde_json::from_value::<UsageEventsIngestRequest>(value);

        assert!(
            result.is_err(),
            "ron-accounting ingest must reject wallet/ledger authority poison field: {field}"
        );
    }
}

#[test]
fn nested_usage_events_reject_wallet_and_ledger_authority_poison() {
    for field in [
        "wallet_balance",
        "wallet_receipt",
        "wallet_mutation",
        "ledger_commit",
        "ledger_sequence",
        "operation_id",
        "account_sequence",
        "settlement_status",
        "state_root",
        "receipt_root",
        "checkpoint_root",
    ] {
        let mut value = json!({
            "timestamp_ms": 1,
            "tenant": 7,
            "subject": "provider_a",
            "metric_kind": "bytes_stored",
            "value": 128
        });

        value
            .as_object_mut()
            .expect("usage event should be an object")
            .insert(field.to_string(), json!("client-supplied-authority"));

        let result = serde_json::from_value::<UsageEvent>(value);

        assert!(
            result.is_err(),
            "UsageEvent must reject wallet/ledger authority poison field: {field}"
        );
    }
}

#[test]
fn quickchain_test_discovery_now_includes_wallet_interlock_boundary() {
    let tests_dir = crate_dir().join("tests");
    let mut quickchain_tests = BTreeSet::new();

    for entry in fs::read_dir(&tests_dir).expect("tests directory should be readable") {
        let entry = entry.expect("test directory entry should be readable");
        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("test file name should be UTF-8")
                .to_string();

            if name.starts_with("quickchain") {
                quickchain_tests.insert(name);
            }
        }
    }

    assert!(
        quickchain_tests.contains("quickchain_preflight_wallet_interlock_boundary.rs"),
        "dynamic QuickChain test discovery must see the new wallet interlock boundary test"
    );
    assert!(
        quickchain_tests.len() >= 9,
        "ron-accounting should now have at least 9 QuickChain preflight test targets, got {quickchain_tests:?}"
    );
}
