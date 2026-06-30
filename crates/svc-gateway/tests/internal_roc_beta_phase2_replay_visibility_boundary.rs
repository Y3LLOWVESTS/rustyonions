//! RO:WHAT — Internal ROC Beta Phase 2 Round 2 replay/audit visibility boundary tests for svc-gateway.
//! RO:WHY — Gateway may expose backend-derived replay/conservation/audit status as read-only display metadata, but must not become replay, wallet, ledger, receipt, balance, paid-unlock, finality, settlement, bridge, staking, or liquidity authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, scripts/dev-internal-roc-beta-phase2-preflight.sh, routes, headers, forwarding/admission source.
//! RO:INVARIANTS — replay/audit status is source-labeled, backend-derived, read-only, display-only, and never paid unlock authority.
//! RO:METRICS — none; source/docs/status DTO boundary only.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — no new ledger mutation path; no gateway direct ledger mutation; no fake receipts, fake balances, fake finality, silent spend, bridge, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p svc-gateway --test internal_roc_beta_phase2_replay_visibility_boundary.

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

const STATUS_SCHEMA: &str = "svc-gateway.internal-roc-beta.replay-audit-status.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatewayReplayAuditStatus {
    schema: String,
    produced_at_ms: u64,
    source: String,
    chain_id: String,
    account_history_cid: String,
    replay_report_cid: String,
    conservation_report_cid: String,
    status_label: String,
    display_label: String,
    backend_derived: bool,
    read_only: bool,
    display_only: bool,
    audit_only: bool,
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

impl GatewayReplayAuditStatus {
    fn sample() -> Self {
        Self {
            schema: STATUS_SCHEMA.to_string(),
            produced_at_ms: 1_780_000_002_000,
            source: "backend-derived:svc-gateway".to_string(),
            chain_id: "internal-roc-beta".to_string(),
            account_history_cid: format!("b3:{}", "1".repeat(64)),
            replay_report_cid: format!("b3:{}", "2".repeat(64)),
            conservation_report_cid: format!("b3:{}", "3".repeat(64)),
            status_label: "replay_observed".to_string(),
            display_label: "Replay/conservation status observed from backend audit data"
                .to_string(),
            backend_derived: true,
            read_only: true,
            display_only: true,
            audit_only: true,
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
        if self.schema != STATUS_SCHEMA {
            return Err("invalid gateway replay/audit status schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("gateway replay/audit status produced_at_ms must be nonzero".to_string());
        }

        if !self.source.starts_with("backend-derived:") {
            return Err(
                "gateway replay/audit status must be source-labeled as backend-derived".to_string(),
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

        match self.status_label.as_str() {
            "replay_observed" | "conservation_observed" | "audit_unavailable" => {}
            other => {
                return Err(format!(
                    "gateway replay/audit status label must not overstate finality: {other}"
                ));
            }
        }

        if !self.backend_derived {
            return Err("gateway replay/audit status must be backend-derived".to_string());
        }
        if !self.read_only {
            return Err("gateway replay/audit status must be read-only".to_string());
        }
        if !self.display_only {
            return Err("gateway replay/audit status must be display-only".to_string());
        }
        if !self.audit_only {
            return Err("gateway replay/audit status must be audit-only".to_string());
        }
        if !self.replay_observation_only {
            return Err("gateway replay/audit status must be replay-observation-only".to_string());
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
                    "gateway replay/audit status must not claim {field}"
                ));
            }
        }

        Ok(())
    }
}

#[test]
fn replay_visibility_status_is_strict_source_labeled_display_metadata() {
    let status = GatewayReplayAuditStatus::sample();
    status
        .validate()
        .expect("sample gateway replay/audit status should validate");

    let encoded = serde_json::to_value(&status).expect("status should encode");
    assert_eq!(encoded["schema"], STATUS_SCHEMA);
    assert_eq!(encoded["source"], "backend-derived:svc-gateway");
    assert_eq!(encoded["backend_derived"], true);
    assert_eq!(encoded["read_only"], true);
    assert_eq!(encoded["display_only"], true);
    assert_eq!(encoded["audit_only"], true);
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
        let mut poisoned = GatewayReplayAuditStatus::sample();
        poisoned.status_label = forbidden_value.to_string();
        assert!(
            poisoned.validate().is_err(),
            "gateway replay/audit status must reject overclaiming label `{forbidden_value}`"
        );
    }

    for forbidden_field in [
        "unlock_from_replay_status",
        "paid_access_from_replay_status",
        "balance_from_replay_status",
        "receipt_from_replay_status",
        "finality_from_replay_status",
        "settlement_from_replay_status",
        "bridge_from_replay_status",
        "staking_from_replay_status",
        "liquidity_from_replay_status",
        "cache_unlock_from_replay_status",
    ] {
        let mut value = encoded.clone();
        value
            .as_object_mut()
            .expect("status JSON object")
            .insert(forbidden_field.to_string(), json!(true));

        assert!(
            serde_json::from_value::<GatewayReplayAuditStatus>(value).is_err(),
            "gateway replay/audit DTO must reject unknown authority field `{forbidden_field}`"
        );
    }
}

#[test]
fn replay_visibility_authority_flags_fail_closed() {
    for mutator in [
        |status: &mut GatewayReplayAuditStatus| status.wallet_side_effect = true,
        |status: &mut GatewayReplayAuditStatus| status.ledger_side_effect = true,
        |status: &mut GatewayReplayAuditStatus| status.paid_unlock_authority = true,
        |status: &mut GatewayReplayAuditStatus| status.balance_truth = true,
        |status: &mut GatewayReplayAuditStatus| status.receipt_truth = true,
        |status: &mut GatewayReplayAuditStatus| status.finality_truth = true,
        |status: &mut GatewayReplayAuditStatus| status.settlement_truth = true,
        |status: &mut GatewayReplayAuditStatus| status.bridge_runtime = true,
        |status: &mut GatewayReplayAuditStatus| status.staking_runtime = true,
        |status: &mut GatewayReplayAuditStatus| status.liquidity_enabled = true,
        |status: &mut GatewayReplayAuditStatus| status.cache_unlock_authority = true,
    ] {
        let mut status = GatewayReplayAuditStatus::sample();
        mutator(&mut status);
        assert!(
            status.validate().is_err(),
            "gateway replay/audit visibility must fail closed on authority flags: {status:?}"
        );
    }
}

#[test]
fn gateway_runtime_source_does_not_turn_replay_status_into_authority() {
    let mut files = Vec::new();
    for rel in [
        "src/routes",
        "src/forward",
        "src/admission",
        "src/layers",
        "src/headers",
        "src/policy",
    ] {
        collect_rust_files(&crate_root().join(rel), &mut files);
    }

    assert!(
        !files.is_empty(),
        "expected svc-gateway runtime source files to scan"
    );

    let forbidden_compact_markers = [
        "unlockfromreplaystatus",
        "paidaccessfromreplaystatus",
        "receiptfromreplaystatus",
        "balancefromreplaystatus",
        "finalityfromreplaystatus",
        "settlementfromreplaystatus",
        "gatewayreplaytruth",
        "gatewayconservationtruth",
        "gatewayaudittruth",
        "gatewayreplaywalletmutation",
        "gatewayreplayledgermutation",
        "replaystatuspaidunlockauthority:true",
        "\"replaystatuspaidunlockauthority\":true",
        "replaystatusfinalitytruth:true",
        "\"replaystatusfinalitytruth\":true",
        "replaystatussettlementtruth:true",
        "\"replaystatussettlementtruth\":true",
        "replaystatuscacheunlockauthority:true",
        "\"replaystatuscacheunlockauthority\":true",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("svc-gateway runtime source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_does_not_forward_replay_authority_headers() {
    let header_source = read_rel("src/headers/proxy.rs").to_ascii_lowercase();

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
            "svc-gateway proxy header allowlist",
        );
    }
}

#[test]
fn docs_and_preflight_name_internal_roc_phase2_round2_boundary() {
    let doc = read_rel("docs/quickchain-preflight.md");
    let script = read_rel("scripts/dev-internal-roc-beta-phase2-preflight.sh");

    for required in [
        "Internal ROC Beta Phase 2 Round 2 downstream replay visibility boundary",
        "svc-gateway may expose read-only replay/conservation/audit status",
        "Replay status is display/audit metadata only",
        "Replay status cannot unlock paid content",
        "Replay status cannot claim finality",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }

    for required in [
        "internal_roc_beta_phase2_replay_visibility_boundary",
        "internal_roc_beta_paid_content_route_boundary",
        "quickchain_phase2_replay_boundary",
        "quickchain_phase5_anchor_status_boundary",
        "no fake receipts, fake balances, fake finality, silent spend",
    ] {
        assert_contains(
            &script,
            required,
            "svc-gateway Internal ROC Beta Phase 2 preflight script",
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
