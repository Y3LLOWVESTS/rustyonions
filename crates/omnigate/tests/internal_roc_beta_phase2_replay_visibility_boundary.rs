//! RO:WHAT — Internal ROC Beta Phase 2 Round 2 replay/audit visibility boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate backend-derived replay/conservation/audit status as read-only display context, but must not become replay, wallet, ledger, receipt, balance, paid-unlock, finality, settlement, bridge, staking, or liquidity authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, scripts/dev-internal-roc-beta-phase2-preflight.sh, routes/v1, hydration, downstream, header policy.
//! RO:INVARIANTS — hydrated replay/audit status is source-labeled, backend-derived, read-only, display-only, and never paid unlock authority.
//! RO:METRICS — none; source/docs/status DTO boundary only.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — no new ledger mutation path; no omnigate direct ledger mutation; no fake receipts, fake balances, fake finality, silent spend, bridge, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p omnigate --test internal_roc_beta_phase2_replay_visibility_boundary.

#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

const HYDRATION_SCHEMA: &str = "omnigate.internal-roc-beta.replay-audit-hydration.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OmnigateReplayAuditHydration {
    schema: String,
    produced_at_ms: u64,
    hydration_source: String,
    chain_id: String,
    account_history_cid: String,
    replay_report_cid: String,
    conservation_report_cid: String,
    display_label: String,
    audit_status: String,
    backend_derived: bool,
    read_only: bool,
    display_only: bool,
    hydration_context_only: bool,
    replay_observation_only: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    paid_unlock_authority: bool,
    balance_truth: bool,
    receipt_truth: bool,
    finality_truth: bool,
    settlement_truth: bool,
    bridge_runtime: bool,
    staking_runtime: bool,
    liquidity_enabled: bool,
    cache_unlock_authority: bool,
}

impl OmnigateReplayAuditHydration {
    fn sample() -> Self {
        Self {
            schema: HYDRATION_SCHEMA.to_string(),
            produced_at_ms: 1_780_000_002_500,
            hydration_source: "backend-derived:omnigate".to_string(),
            chain_id: "internal-roc-beta".to_string(),
            account_history_cid: format!("b3:{}", "4".repeat(64)),
            replay_report_cid: format!("b3:{}", "5".repeat(64)),
            conservation_report_cid: format!("b3:{}", "6".repeat(64)),
            display_label: "Replay/conservation status hydrated from backend audit data"
                .to_string(),
            audit_status: "replay_observed".to_string(),
            backend_derived: true,
            read_only: true,
            display_only: true,
            hydration_context_only: true,
            replay_observation_only: true,
            wallet_side_effect: false,
            ledger_side_effect: false,
            paid_unlock_authority: false,
            balance_truth: false,
            receipt_truth: false,
            finality_truth: false,
            settlement_truth: false,
            bridge_runtime: false,
            staking_runtime: false,
            liquidity_enabled: false,
            cache_unlock_authority: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != HYDRATION_SCHEMA {
            return Err("invalid omnigate replay/audit hydration schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err(
                "omnigate replay/audit hydration produced_at_ms must be nonzero".to_string(),
            );
        }

        if !self.hydration_source.starts_with("backend-derived:") {
            return Err(
                "omnigate replay/audit hydration must be source-labeled as backend-derived"
                    .to_string(),
            );
        }

        for (field, value) in [
            ("account_history_cid", self.account_history_cid.as_str()),
            ("replay_report_cid", self.replay_report_cid.as_str()),
            (
                "conservation_report_cid",
                self.conservation_report_cid.as_str(),
            ),
        ] {
            validate_b3(field, value)?;
        }

        match self.audit_status.as_str() {
            "replay_observed" | "conservation_observed" | "audit_unavailable" => {}
            other => {
                return Err(format!(
                    "omnigate replay/audit hydration must not overstate finality: {other}"
                ));
            }
        }

        if !self.backend_derived {
            return Err("omnigate replay/audit hydration must be backend-derived".to_string());
        }
        if !self.read_only {
            return Err("omnigate replay/audit hydration must be read-only".to_string());
        }
        if !self.display_only {
            return Err("omnigate replay/audit hydration must be display-only".to_string());
        }
        if !self.hydration_context_only {
            return Err(
                "omnigate replay/audit hydration must be hydration-context-only".to_string(),
            );
        }
        if !self.replay_observation_only {
            return Err(
                "omnigate replay/audit hydration must be replay-observation-only".to_string(),
            );
        }

        for (field, value) in [
            ("wallet_side_effect", self.wallet_side_effect),
            ("ledger_side_effect", self.ledger_side_effect),
            ("paid_unlock_authority", self.paid_unlock_authority),
            ("balance_truth", self.balance_truth),
            ("receipt_truth", self.receipt_truth),
            ("finality_truth", self.finality_truth),
            ("settlement_truth", self.settlement_truth),
            ("bridge_runtime", self.bridge_runtime),
            ("staking_runtime", self.staking_runtime),
            ("liquidity_enabled", self.liquidity_enabled),
            ("cache_unlock_authority", self.cache_unlock_authority),
        ] {
            if value {
                return Err(format!(
                    "omnigate replay/audit hydration must not claim {field}"
                ));
            }
        }

        Ok(())
    }
}

#[test]
fn replay_audit_hydration_is_strict_source_labeled_display_context() {
    let hydrated = OmnigateReplayAuditHydration::sample();
    hydrated
        .validate()
        .expect("sample omnigate replay/audit hydration should validate");

    let encoded = serde_json::to_value(&hydrated).expect("hydration should encode");
    assert_eq!(encoded["schema"], HYDRATION_SCHEMA);
    assert_eq!(encoded["hydration_source"], "backend-derived:omnigate");
    assert_eq!(encoded["backend_derived"], true);
    assert_eq!(encoded["read_only"], true);
    assert_eq!(encoded["display_only"], true);
    assert_eq!(encoded["hydration_context_only"], true);
    assert_eq!(encoded["replay_observation_only"], true);
    assert_eq!(encoded["wallet_side_effect"], false);
    assert_eq!(encoded["ledger_side_effect"], false);
    assert_eq!(encoded["paid_unlock_authority"], false);
    assert_eq!(encoded["balance_truth"], false);
    assert_eq!(encoded["receipt_truth"], false);
    assert_eq!(encoded["finality_truth"], false);
    assert_eq!(encoded["settlement_truth"], false);
    assert_eq!(encoded["bridge_runtime"], false);
    assert_eq!(encoded["staking_runtime"], false);
    assert_eq!(encoded["liquidity_enabled"], false);
    assert_eq!(encoded["cache_unlock_authority"], false);

    for forbidden_value in [
        "epoch_finalized",
        "finalized",
        "anchored_finality",
        "settled",
        "bridge_settled",
        "staking_confirmed",
        "liquidity_settled",
        "paid_unlock_granted",
        "cache_unlock_granted",
    ] {
        let mut poisoned = OmnigateReplayAuditHydration::sample();
        poisoned.audit_status = forbidden_value.to_string();
        assert!(
            poisoned.validate().is_err(),
            "omnigate replay/audit hydration must reject overclaiming label `{forbidden_value}`"
        );
    }

    for forbidden_field in [
        "unlock_from_replay_hydration",
        "paid_access_from_replay_hydration",
        "balance_from_replay_hydration",
        "receipt_from_replay_hydration",
        "finality_from_replay_hydration",
        "settlement_from_replay_hydration",
        "bridge_from_replay_hydration",
        "staking_from_replay_hydration",
        "liquidity_from_replay_hydration",
        "cache_unlock_from_replay_hydration",
    ] {
        let mut value = encoded.clone();
        value
            .as_object_mut()
            .expect("hydration JSON object")
            .insert(forbidden_field.to_string(), json!(true));

        assert!(
            serde_json::from_value::<OmnigateReplayAuditHydration>(value).is_err(),
            "omnigate replay/audit hydration DTO must reject unknown authority field `{forbidden_field}`"
        );
    }
}

#[test]
fn replay_hydration_authority_flags_fail_closed() {
    for mutator in [
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.wallet_side_effect = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.ledger_side_effect = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.paid_unlock_authority = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.balance_truth = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.receipt_truth = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.finality_truth = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.settlement_truth = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.bridge_runtime = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.staking_runtime = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.liquidity_enabled = true,
        |hydrated: &mut OmnigateReplayAuditHydration| hydrated.cache_unlock_authority = true,
    ] {
        let mut hydrated = OmnigateReplayAuditHydration::sample();
        mutator(&mut hydrated);
        assert!(
            hydrated.validate().is_err(),
            "omnigate replay/audit hydration must fail closed on authority flags: {hydrated:?}"
        );
    }
}

#[test]
fn omnigate_runtime_source_does_not_turn_replay_hydration_into_authority() {
    let mut files = Vec::new();
    for rel in [
        "src/routes",
        "src/hydration",
        "src/downstream",
        "src/middleware",
        "src/auth",
        "src/state.rs",
    ] {
        collect_rust_files(&crate_root().join(rel), &mut files);
    }

    assert!(
        !files.is_empty(),
        "expected omnigate runtime source files to scan"
    );

    let forbidden_compact_markers = [
        "unlockfromreplayhydration",
        "paidaccessfromreplayhydration",
        "receiptfromreplayhydration",
        "balancefromreplayhydration",
        "finalityfromreplayhydration",
        "settlementfromreplayhydration",
        "omnigatereplaytruth",
        "omnigateconservationtruth",
        "omnigateaudittruth",
        "omnigatereplaywalletmutation",
        "omnigatereplayledgermutation",
        "replayhydrationpaidunlockauthority:true",
        "\"replayhydrationpaidunlockauthority\":true",
        "replayhydrationfinalitytruth:true",
        "\"replayhydrationfinalitytruth\":true",
        "replayhydrationsettlementtruth:true",
        "\"replayhydrationsettlementtruth\":true",
        "replayhydrationcacheunlockauthority:true",
        "\"replayhydrationcacheunlockauthority\":true",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("omnigate runtime source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_does_not_forward_replay_authority_headers() {
    let header_source = read_rel("src/routes/v1/header_policy.rs").to_ascii_lowercase();

    for forbidden in [
        "x-ron-replay-paid-unlock",
        "x-ron-replay-finality",
        "x-ron-replay-settlement",
        "x-ron-replay-wallet-mutation",
        "x-ron-replay-ledger-mutation",
        "x-ron-replay-balance-truth",
        "x-ron-replay-receipt-truth",
        "x-ron-replay-cache-unlock",
        "x-ron-replay-bridge",
        "x-ron-replay-staking",
        "x-ron-replay-liquidity",
    ] {
        assert_not_contains(
            &header_source,
            forbidden,
            "omnigate v1 header policy allowlist",
        );
    }
}

#[test]
fn docs_and_preflight_name_internal_roc_phase2_round2_boundary() {
    let doc = read_rel("docs/quickchain-preflight.md");
    let script = read_rel("scripts/dev-internal-roc-beta-phase2-preflight.sh");

    for required in [
        "Internal ROC Beta Phase 2 Round 2 downstream replay visibility boundary",
        "omnigate may hydrate read-only replay/conservation/audit status",
        "Replay hydration is display/audit context only",
        "Replay hydration cannot unlock paid content",
        "Replay hydration cannot claim finality",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }

    for required in [
        "internal_roc_beta_phase2_replay_visibility_boundary",
        "internal_roc_beta_paid_content_access_boundary",
        "quickchain_phase2_replay_boundary",
        "quickchain_phase5_anchor_hydration_boundary",
        "no fake receipts, fake balances, fake finality, silent spend",
    ] {
        assert_contains(
            &script,
            required,
            "omnigate Internal ROC Beta Phase 2 preflight script",
        );
    }
}

fn validate_b3(field: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(format!("{field} must be b3:<64 lowercase hex>"));
    };

    if hex.len() != 64 || !hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(format!("{field} must be b3:<64 lowercase hex>"));
    }

    Ok(())
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| panic!("read {}: {err}", full.display()))
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }

    if root.is_file() {
        if root.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(root.to_path_buf());
        }
        return;
    }

    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()));

    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("read dir entry under {}: {err}", root.display()));
        let path = entry.path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn compact_without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} missing required marker `{needle}`"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden marker `{needle}`"
    );
}
