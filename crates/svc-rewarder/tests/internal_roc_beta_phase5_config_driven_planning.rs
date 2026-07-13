//! RO:WHAT — Internal ROC Beta Phase 5 config-driven planning tests for svc-rewarder.
//!
//! RO:WHY — Proves rewarder consumes canonical tokenomics config for planning only and avoids hard-coded payout caps.
//! RO:INTERACTS — `inputs::economics`, `RewardPolicy`, `AccountingSnapshot`, and `compute_manifest`.
//! RO:INVARIANTS — config values are planning caps only; reward plans are not receipts; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — reads canonical `configs/roc-economics.toml` fixture.
//! RO:SECURITY — no fake balance, fake receipt, finality, bridge, staking, liquidity, or external settlement.
//! RO:TEST — `cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning`.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use std::fs;
use std::path::{Path, PathBuf};

use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    inputs::{
        canonical_snapshot_cid, load_internal_roc_planning_economics_toml, AccountContribution,
        AccountingSnapshot, ContentCid,
    },
    outputs::IntentResult,
};

const ROC_ECONOMICS_TOML: &str = include_str!("../../../configs/roc-economics.toml");
const POLICY_ID: &str = "policy:internal-roc-beta-phase5";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn snapshot_above_configured_epoch_cap() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_930_000_000_000,
        pool_minor_units: AmountMinor(2_000_000),
        contributions: vec![
            AccountContribution {
                account: "acct_creator_a".to_owned(),
                bytes_stored: 900,
                bytes_served: 300,
                uptime_seconds: 100,
            },
            AccountContribution {
                account: "acct_creator_b".to_owned(),
                bytes_stored: 500,
                bytes_served: 200,
                uptime_seconds: 50,
            },
        ],
    }
}

fn content_id_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("snapshot should canonicalize");
    ContentCid::parse(cid).expect("canonical snapshot CID should parse")
}

#[test]
fn rewarder_consumes_roc_economics_config_for_planning_only() {
    let economics = load_internal_roc_planning_economics_toml(ROC_ECONOMICS_TOML.as_bytes())
        .expect("canonical economics config projects to reward planning economics");

    assert_eq!(economics.schema, "internal_roc.economics-config.v1");
    assert_eq!(economics.version, 1);
    assert_eq!(economics.profile, "canonical");

    let economics_hash_hex = economics
        .economics_config_hash
        .strip_prefix("b3:")
        .expect("economics config hash must use b3 prefix");

    assert_eq!(economics_hash_hex.len(), 64);
    assert!(economics_hash_hex
        .bytes()
        .all(|byte| { byte.is_ascii_digit() || matches!(byte, b'a'..=b'f') }));

    assert_eq!(economics.epoch_pool_cap_minor, AmountMinor(1_000_000));
    assert_eq!(
        economics.max_reward_minor_per_account_per_epoch,
        AmountMinor(10_000)
    );
    assert_eq!(
        economics.max_reward_minor_per_content_per_epoch,
        AmountMinor(50_000)
    );
    assert_eq!(economics.rounding_mode, "floor");
    assert_eq!(economics.remainder_sink, "treasury");
    assert!(economics.bridge_inert);
    assert!(economics.staking_inert);

    let snapshot = snapshot_above_configured_epoch_cap();
    let policy = economics
        .to_reward_policy(POLICY_ID, POLICY_HASH)
        .expect("config-derived reward policy validates");

    assert_eq!(
        policy.max_payout_minor_units,
        economics.epoch_pool_cap_minor
    );

    let manifest = compute_manifest(
        ComputeInput {
            epoch_id: "epoch-internal-roc-beta-phase5-config-driven".to_owned(),
            inputs_cid: content_id_for(snapshot.clone()),
            policy,
            snapshot,
            dry_run: true,
            idempotency_salt: "internal-roc-beta|phase5|config-driven-planning".to_owned(),
        },
        IntentResult::DryRun,
    )
    .expect("config-driven dry-run manifest computes");

    assert_eq!(
        manifest.totals.pool_minor_units,
        economics.epoch_pool_cap_minor
    );
    assert!(!manifest.ledger.emitted);
    assert_eq!(manifest.ledger.result, "dry_run");

    let encoded = serde_json::to_string(&manifest).expect("manifest serializes");
    for forbidden in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "wallet_receipt",
        "ledger_receipt",
        "balance_truth",
        "finality",
        "finalized",
        "bridge_txid",
        "solana_signature",
        "staking_position_id",
        "liquidity_pool_id",
        "exchange_order_id",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "config-driven reward plan must not claim authority field {forbidden}"
        );
    }
}

#[test]
fn rewarder_rejects_enabled_future_bridge_or_staking_config_for_planning() {
    let bridge_enabled = ROC_ECONOMICS_TOML.replacen("enabled = false", "enabled = true", 1);
    let err = load_internal_roc_planning_economics_toml(bridge_enabled.as_bytes())
        .expect_err("enabled bridge config must reject");
    assert!(
        err.to_string().contains("future_bridge.enabled"),
        "unexpected error: {err}"
    );

    let staking_enabled = ROC_ECONOMICS_TOML.replacen("enabled = false", "enabled = true", 2);
    let err = load_internal_roc_planning_economics_toml(staking_enabled.as_bytes())
        .expect_err("enabled staking config must reject");
    assert!(
        err.to_string().contains("must remain false/inert"),
        "unexpected error: {err}"
    );
}

#[test]
fn rewarder_business_logic_has_no_hardcoded_phase5_payout_caps() {
    let mut files = Vec::new();
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    collect_rs_files(&src_dir, &mut files);

    let forbidden_fragments = [
        "AmountMinor(1_000_000)",
        "AmountMinor(1000000)",
        "pool_minor_units: AmountMinor(1_000_000)",
        "max_payout_minor_units: AmountMinor(1_000_000)",
        "epoch_pool_cap_minor: \"1000000\"",
        "max_reward_minor_per_account_per_epoch: \"10000\"",
        "max_reward_minor_per_content_per_epoch: \"50000\"",
    ];

    for file in files {
        let text = fs::read_to_string(&file).expect("source file readable");
        for forbidden in forbidden_fragments {
            assert!(
                !text.contains(forbidden),
                "{} must not hard-code Phase 5 payout/tokenomics cap `{forbidden}` in business logic",
                file.display()
            );
        }
    }
}

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("source directory readable") {
        let entry = entry.expect("directory entry readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
