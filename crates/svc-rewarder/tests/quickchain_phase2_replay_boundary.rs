//! RO:WHAT — Phase 2 Round 1 verifier-artifact boundary tests for svc-rewarder.
//! RO:WHY — Confirms reward plans can feed read-only replay artifacts without becoming committee/finality authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, outputs::{manifest,intents,artifacts,wallet}, core::compute.
//! RO:INVARIANTS — rewarder plans only; verifier artifacts are read-only inputs; wallet/ledger remain mutation/truth.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks quorum, fork-choice, validator-signature, bridge, staking, and settlement creep.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase2_replay_boundary.

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
fn docs_name_phase2_round1_read_only_verifier_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 1 verifier artifact / read-only replication",
        "reward manifests may become read-only verifier artifact inputs",
        "svc-rewarder does not sign committee votes",
        "svc-rewarder does not decide quorum",
        "svc-rewarder does not claim fork choice",
        "svc-rewarder does not claim finality",
        "svc-rewarder still cannot mutate ledger truth",
        "svc-wallet commits approved payout intents",
        "ron-ledger remains durable economic truth",
        "quickchain_phase2_replay_boundary",
    ] {
        assert_contains(&doc, required, "svc-rewarder quickchain-preflight.md");
    }
}

#[test]
fn reward_outputs_are_replay_artifact_inputs_not_committee_authority() {
    let manifest = read("src/outputs/manifest.rs");
    let artifacts = read("src/outputs/artifacts.rs");
    let intents = read("src/outputs/intents.rs");
    let wallet = read("src/outputs/wallet.rs");
    let compute = read("src/core/compute.rs");

    let source = strip_line_comments(&format!(
        "{manifest}\n{artifacts}\n{intents}\n{wallet}\n{compute}"
    ));
    let lower = source.to_ascii_lowercase();

    for required in [
        "RewardManifest",
        "commitment_for_manifest",
        "manifest_commitment",
        "SettlementBatch",
        "WalletIssueRequest",
        "to_wallet_issue_request",
        "IntentResult",
        "serde_json::to_vec_pretty",
        "#[serde(deny_unknown_fields)]",
    ] {
        assert!(
            manifest.contains(required)
                || artifacts.contains(required)
                || intents.contains(required)
                || wallet.contains(required)
                || compute.contains(required),
            "rewarder planning/artifact surface must preserve marker: {required}"
        );
    }

    for forbidden in [
        "committee",
        "quorum",
        "fork_choice",
        "validator_signature",
        "validator_set",
        "checkpoint_signature",
        "finality_vote",
        "replicated_replay_result",
        "verifier_result",
        "root_producer",
        "external_anchor",
        "bridge_txid",
        "solana",
        "rox",
        "staking",
        "liquidity",
    ] {
        assert_not_contains(
            &lower,
            forbidden,
            "svc-rewarder Phase 2 read-only replay boundary source",
        );
    }
}

#[test]
fn rewarder_does_not_turn_replay_artifacts_into_wallet_or_ledger_mutation_truth() {
    let source = strip_line_comments(&read_sources(&[
        "src/inputs/accounting.rs",
        "src/inputs/cid.rs",
        "src/inputs/policy.rs",
        "src/core/compute.rs",
        "src/outputs/intents.rs",
        "src/outputs/wallet.rs",
    ]));
    let lower = source.to_ascii_lowercase();

    for required in [
        "AccountingSnapshot",
        "ContentCid",
        "RewardFundingSource",
        "policy_hash_is_canonical",
        "WalletIssueClient",
        "WALLET_ISSUE_PATH",
        "Idempotency-Key",
    ] {
        assert_contains(
            &source,
            required,
            "svc-rewarder planning/wallet handoff source",
        );
    }

    for forbidden in [
        "ron_ledger::",
        "ledgerclient",
        "ledger_commit",
        "mutate_ledger",
        "mutate_balance",
        "issue_without_wallet",
        "mint_directly",
        "operation_id",
        "account_sequence",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "checkpoint_hash",
        "settlement_finality",
        "committee_signature",
        "quorum_signature",
    ] {
        assert_not_contains(
            &lower,
            forbidden,
            "svc-rewarder replay artifact must not become mutation truth",
        );
    }
}
