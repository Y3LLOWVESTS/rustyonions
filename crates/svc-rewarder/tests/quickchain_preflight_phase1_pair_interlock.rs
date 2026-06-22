//! RO:WHAT — QC-1A pair interlock tests for svc-rewarder in the svc-rewarder ↔ svc-storage pass.
//! RO:WHY — Locks rewarder as deterministic payout planner while storage/accounting/wallet/ledger keep their own authority boundaries.
//! RO:INTERACTS — docs/quickchain-preflight.md, outputs::{manifest,intents,wallet}, inputs::accounting, core::compute.
//! RO:INVARIANTS — manifests/intents are planning artifacts; wallet receipts are backend-returned; no roots/finality/validators/bridges.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents rewarder from treating storage CIDs, accounting snapshots, or wallet responses as settlement authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_phase1_pair_interlock.

use std::{
    fs,
    path::{Path, PathBuf},
};

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

fn read_sources(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_not_contains(haystack: &str, forbidden: &str, context: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{context} must not contain forbidden QuickChain authority marker: {forbidden}"
    );
}

#[test]
fn phase1_round1_docs_lock_rewarder_as_planner_not_root_authority() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "svc-storage/svc-gateway/omnigate paid enforcement",
        "ron-accounting snapshots",
        "svc-rewarder payout planning",
        "explicit approved payout intent",
        "svc-wallet",
        "ron-ledger",
        "deterministic roc payout planner",
        "svc-wallet is the mutation front-door",
        "ron-ledger is durable economic truth",
    ] {
        assert!(
            doc.contains(required),
            "quickchain-preflight.md must preserve QC-1A rewarder planner boundary phrase: {required}"
        );
    }
}

#[test]
fn manifests_and_settlement_batches_are_planning_artifacts_not_roots() {
    let manifest = read("src/outputs/manifest.rs");
    let intents = read("src/outputs/intents.rs");
    let source = strip_line_comments(&format!("{manifest}\n{intents}"));

    for required in [
        "RewardManifest",
        "commitment",
        "SettlementIntent",
        "SettlementBatch",
        "WalletIssueBatch",
        "WalletIssueRequest",
        "funding_source",
        "#[serde(deny_unknown_fields)]",
        "to_wallet_issue_request",
    ] {
        assert!(
            manifest.contains(required) || intents.contains(required),
            "rewarder planning surface must preserve marker: {required}"
        );
    }

    for forbidden in [
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "checkpoint_hash",
        "validator_signature",
        "validator_set",
        "settlement_finality",
        "external_anchor",
        "bridge_txid",
        "staking",
        "liquidity",
    ] {
        assert_not_contains(&source, forbidden, "rewarder manifest/intent source");
    }
}

#[test]
fn wallet_http_outcome_may_return_backend_wallet_receipts_but_not_rewarder_receipt_authority() {
    let wallet = read("src/outputs/wallet.rs");
    let source = strip_line_comments(&wallet);

    for required in [
        "WalletHttpIssueOutcome",
        "pub receipts: Vec<Value>",
        "WalletIssueClient",
        "HttpWalletIssueClient",
        "post_issue",
        "WALLET_ISSUE_PATH",
    ] {
        assert!(
            wallet.contains(required),
            "rewarder wallet seam must preserve backend-wallet handoff marker: {required}"
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "mint_directly",
        "issue_without_wallet",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "validator_signature",
        "external_anchor",
        "bridge_txid",
    ] {
        assert_not_contains(&source, forbidden, "rewarder wallet egress source");
    }
}

#[test]
fn accounting_and_storage_inputs_cannot_smuggle_payout_execution_or_root_material() {
    let source = strip_line_comments(&read_sources(&[
        "src/inputs/accounting.rs",
        "src/inputs/cache.rs",
        "src/inputs/ledger_snapshot.rs",
        "src/core/compute.rs",
    ]));

    for forbidden in [
        "wallet_issue_request",
        "wallet_transfer_request",
        "wallet_capture_request",
        "approved_payout",
        "execute_payout",
        "ledger_commit",
        "operation_id",
        "account_sequence",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "validator_signature",
        "external_anchor",
        "bridge_txid",
    ] {
        assert_not_contains(&source, forbidden, "rewarder input/compute source");
    }
}

#[test]
fn rewarder_src_has_no_root_producer_validator_bridge_or_external_settlement_runtime() {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);

    assert!(
        !files.is_empty(),
        "svc-rewarder src tree should contain Rust files"
    );

    let mut source = String::new();
    for path in files {
        source.push_str(&strip_line_comments(
            &fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!("failed to read {}: {err}", path.display());
            }),
        ));
        source.push('\n');
    }

    for forbidden in [
        "produce_root",
        "seal_checkpoint",
        "checkpoint_producer",
        "validator_runtime",
        "validator_signature",
        "committee_quorum",
        "external_anchor",
        "bridge_txid",
        "solana",
        "rox",
        "staking",
        "liquidity",
        "public_chain",
    ] {
        assert_not_contains(&source, forbidden, "svc-rewarder src tree");
    }
}
