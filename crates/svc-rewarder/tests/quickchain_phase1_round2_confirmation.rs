//! RO:WHAT — Phase 1 Round 2 downstream-confirmation tests for svc-rewarder.
//! RO:WHY — Confirms rewarder can produce referenceable payout artifacts without becoming ledger/root/finality authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, outputs::{manifest,intents,artifacts,wallet}, inputs::{accounting,cid,policy}.
//! RO:INVARIANTS — reward plans are artifacts/plans; svc-wallet commits approved payout intents; ron-ledger remains truth.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents Phase 1 root/proof vocabulary from becoming rewarder mutation authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase1_round2_confirmation.

use std::{fs, path::PathBuf};

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
fn docs_name_phase1_round2_downstream_confirmation_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 1 round 2 downstream confirmation",
        "reward plans can be referenced as artifacts",
        "reward manifest commitments are artifact references, not quickchain roots",
        "storage artifact cids are artifact references, not quickchain roots",
        "svc-rewarder cannot mutate ledger truth",
        "svc-wallet commits approved payout intents",
        "ron-ledger remains durable economic truth",
        "quickchain_phase1_round2_confirmation",
    ] {
        assert_contains(&doc, required, "svc-rewarder quickchain-preflight.md");
    }
}

#[test]
fn reward_manifest_commitment_is_referenceable_artifact_not_root_authority() {
    let manifest = read("src/outputs/manifest.rs");
    let artifacts = read("src/outputs/artifacts.rs");
    let intents = read("src/outputs/intents.rs");
    let source = strip_line_comments(&format!("{manifest}\n{artifacts}\n{intents}"));

    for required in [
        "RewardManifest",
        "commitment_for_manifest",
        "blake3::hash",
        "inputs_cid",
        "maybe_write_manifest",
        "serde_json::to_vec_pretty",
        "manifest_commitment",
        "SettlementBatch",
        "WalletIssueRequest",
        "to_wallet_issue_request",
        "#[serde(deny_unknown_fields)]",
    ] {
        assert!(
            manifest.contains(required)
                || artifacts.contains(required)
                || intents.contains(required),
            "rewarder artifact/planning surface must preserve marker: {required}"
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "direct_ledger",
        "mutate_ledger",
        "wallet_balance",
        "account_sequence",
        "operation_id",
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
        "solana",
        "rox",
        "staking",
        "liquidity",
    ] {
        assert_not_contains(
            &source,
            forbidden,
            "svc-rewarder manifest/artifact/intent source",
        );
    }
}

#[test]
fn accounting_and_policy_inputs_are_planning_inputs_not_balance_or_root_truth() {
    let source = strip_line_comments(&read_sources(&[
        "src/inputs/accounting.rs",
        "src/inputs/cid.rs",
        "src/inputs/policy.rs",
        "src/core/compute.rs",
    ]));

    for required in [
        "AccountingSnapshot",
        "AccountContribution",
        "canonical_snapshot_cid",
        "resolve_accounting_snapshot",
        "ContentCid",
        "RewardFundingSource",
        "policy_hash_is_canonical",
        "#[serde(deny_unknown_fields)]",
    ] {
        assert_contains(&source, required, "svc-rewarder input/compute source");
    }

    for forbidden in [
        "wallet_issue_request",
        "wallet_transfer_request",
        "wallet_capture_request",
        "execute_payout",
        "ledger_commit",
        "ron_ledger::",
        "LedgerClient",
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
    ] {
        assert_not_contains(&source, forbidden, "svc-rewarder input/compute source");
    }
}

#[test]
fn wallet_egress_remains_explicit_wallet_issue_handoff_not_rewarder_mutation_truth() {
    let wallet = read("src/outputs/wallet.rs");
    let intents = read("src/outputs/intents.rs");
    let source = strip_line_comments(&format!("{wallet}\n{intents}"));

    for required in [
        "WalletIssueClient",
        "WalletHttpIssueOutcome",
        "HttpWalletIssueClient",
        "post_issue",
        "WALLET_ISSUE_PATH",
        "Idempotency-Key",
        "WalletIssueRequest",
        "SettlementBatch",
    ] {
        assert!(
            wallet.contains(required) || intents.contains(required),
            "rewarder wallet handoff must preserve marker: {required}"
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "issue_without_wallet",
        "mint_directly",
        "mutate_balance",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "external_anchor",
        "bridge_txid",
        "staking",
        "liquidity",
    ] {
        assert_not_contains(&source, forbidden, "svc-rewarder wallet egress source");
    }
}
