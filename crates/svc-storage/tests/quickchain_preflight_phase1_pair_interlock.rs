//! RO:WHAT — QC-1A pair interlock tests for svc-storage in the svc-rewarder ↔ svc-storage pass.
//! RO:WHY — Locks storage as byte/artifact + metering infrastructure, not root, payout, cache-unlock, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, http::routes::paid_object, policy::{paid_write,settlement}, accounting exporter.
//! RO:INVARIANTS — b3 is byte truth only; wallet receipts are evidence only; accounting export is metering; rewarder/wallet/ledger own later steps.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents storage responses, cache paths, or artifact CIDs from becoming paid-access/finality/root authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_phase1_pair_interlock.

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

/// Remove ordinary `#[cfg(test)] mod tests { ... }` blocks before runtime source scans.
///
/// The broad src-tree authority scan is meant to catch production/runtime authority creep.
/// It should not fail on deliberately hostile strings inside unit tests such as parser
/// rejection fixtures.
fn strip_cfg_test_modules(text: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(relative_attr_start) = text[cursor..].find("#[cfg(test)]") {
        let attr_start = cursor + relative_attr_start;
        output.push_str(&text[cursor..attr_start]);

        let after_attr = attr_start + "#[cfg(test)]".len();
        let Some(relative_mod_start) = text[after_attr..].find("mod tests") else {
            output.push_str(&text[attr_start..after_attr]);
            cursor = after_attr;
            continue;
        };

        let mod_start = after_attr + relative_mod_start;
        if text[after_attr..mod_start]
            .chars()
            .any(|ch| !ch.is_whitespace())
        {
            output.push_str(&text[attr_start..after_attr]);
            cursor = after_attr;
            continue;
        }

        let Some(relative_open_brace) = text[mod_start..].find('{') else {
            output.push_str(&text[attr_start..]);
            cursor = text.len();
            break;
        };

        let open_brace = mod_start + relative_open_brace;
        let mut depth = 0usize;
        let mut block_end = None;

        for (relative_index, ch) in text[open_brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        block_end = Some(open_brace + relative_index + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }

        match block_end {
            Some(end) => {
                output.push_str("\n/* stripped cfg(test) module for runtime authority scan */\n");
                cursor = end;
            }
            None => {
                output.push_str(&text[attr_start..]);
                cursor = text.len();
                break;
            }
        }
    }

    output.push_str(&text[cursor..]);
    output
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
fn phase1_round1_docs_lock_storage_as_paid_admission_and_metering_only() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "svc-storage paid admission and b3 byte integrity",
        "storage/access metering",
        "ron-accounting derivative snapshots",
        "svc-rewarder deterministic payout planning",
        "explicit approved payout intent",
        "svc-wallet",
        "ron-ledger",
        "b3 hashes identify bytes only",
        "cache must not decide paid access by itself",
        "storage metering is derivative accounting input only",
    ] {
        assert!(
            doc.contains(required),
            "quickchain-preflight.md must preserve QC-1A storage value-loop phrase: {required}"
        );
    }
}

#[test]
fn paid_object_response_is_storage_admission_plus_metering_not_reward_or_root_authority() {
    let paid_object = read("src/http/routes/paid_object.rs");
    let source = strip_line_comments(&paid_object);

    for required in [
        "PaidPutResp",
        "cid: String",
        "paid: bool",
        "wallet_txid: String",
        "wallet_receipt_hash: String",
        "paid_context_idem: Option<String>",
        "settlement: Option<PaidStorageSettlement>",
        "accounting_export: AccountingExportReport",
        "usage_events: Vec<UsageEventDto>",
    ] {
        assert!(
            paid_object.contains(required),
            "paid object response must preserve storage/admission/metering marker: {required}"
        );
    }

    for forbidden in [
        "reward_root",
        "rewarder_decision",
        "payout_plan",
        "payout_receipt",
        "accounting_root",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "validator_signature",
        "settlement_finality",
        "external_anchor",
        "bridge_txid",
    ] {
        assert_not_contains(&source, forbidden, "paid object route source");
    }
}

#[test]
fn settlement_adapter_targets_wallet_capture_release_without_ledger_or_chain_authority() {
    let settlement = read("src/policy/settlement.rs");
    let source = strip_line_comments(&settlement);

    for required in [
        "WalletSettlementHttpClient",
        "/v1/capture",
        "/v1/release",
        "PaidStorageSettlementPlan",
        "PaidStorageSettlement",
        "capture_idem",
        "release_idem",
        "failed_write_release_idem",
    ] {
        assert!(
            settlement.contains(required),
            "storage settlement adapter must preserve wallet capture/release marker: {required}"
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "direct_ledger",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "validator_signature",
        "validator_set",
        "external_anchor",
        "bridge_txid",
        "staking",
        "liquidity",
    ] {
        assert_not_contains(&source, forbidden, "storage settlement source");
    }
}

#[test]
fn accounting_export_is_metering_only_not_payout_authorization() {
    let full_text = read_sources(&[
        "src/accounting/mod.rs",
        "src/accounting/exporter.rs",
        "src/http/routes/paid_object.rs",
    ]);
    let source = strip_line_comments(&full_text);

    for required in [
        "UsageEventDto",
        "ACCOUNTING_EXPORT_SCHEMA",
        "ACCOUNTING_USAGE_EVENTS_PATH",
        "AccountingExportReport",
        "export_usage_events_from_env",
        "export failure never mutates ledger or wallet state",
        "no wallet receipt/body bytes exported",
    ] {
        assert!(
            full_text.contains(required),
            "storage accounting/export seam must preserve metering-only marker: {required}"
        );
    }

    for forbidden in [
        "wallet_issue_request",
        "wallet_transfer_request",
        "approved_payout",
        "execute_payout",
        "rewarder_decision",
        "payout_plan",
        "payout_receipt",
        "reward_root",
        "protocol_reward",
        "mint_from_storage",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "validator_signature",
        "external_anchor",
        "bridge_txid",
    ] {
        assert_not_contains(&source, forbidden, "storage accounting/export source");
    }
}

#[test]
fn storage_src_has_no_root_producer_validator_bridge_or_external_settlement_runtime() {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);

    assert!(
        !files.is_empty(),
        "svc-storage src tree should contain Rust files"
    );

    let mut source = String::new();
    for path in files {
        let raw = fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", path.display());
        });
        let runtime_only = strip_cfg_test_modules(&raw);
        source.push_str(&strip_line_comments(&runtime_only.to_ascii_lowercase()));
        source.push('\n');
    }

    for forbidden in [
        "produce_root",
        "seal_checkpoint",
        "checkpoint_producer",
        "validator_runtime",
        "validator_signature",
        "committee_quorum",
        "reward_root",
        "accounting_root",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "external_anchor",
        "bridge_txid",
        "solana",
        "rox",
        "staking",
        "liquidity",
        "public_chain",
    ] {
        assert_not_contains(&source, forbidden, "svc-storage runtime src tree");
    }
}
