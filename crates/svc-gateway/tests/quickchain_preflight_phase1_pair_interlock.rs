#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — QC-1A pair-interlock tests for svc-gateway.
//! RO:WHY — Keeps the public gateway boundary from becoming wallet, ledger, root, finality, validator, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, scripts/dev-quickchain-preflight.sh, svc-gateway runtime source.
//! RO:INVARIANTS — backend-derived paid enforcement only; no direct ledger mutation; no route-level finality claims.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — blocks authority creep through routes, source shortcuts, or client/cache/header trust.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_preflight_phase1_pair_interlock.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| panic!("read {}: {err}", full.display()))
}

fn assert_contains_all(haystack: &str, label: &str, phrases: &[&str]) {
    for phrase in phrases {
        assert!(
            haystack.contains(phrase),
            "{label} must contain QC-1A interlock phrase `{phrase}`"
        );
    }
}

fn assert_contains_none(haystack: &str, label: &str, snippets: &[&str]) {
    let normalized = haystack.to_ascii_lowercase();

    for snippet in snippets {
        let needle = snippet.to_ascii_lowercase();
        assert!(
            !normalized.contains(&needle),
            "{label} must not contain forbidden QC-1A authority snippet `{snippet}`"
        );
    }
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn strip_cfg_test_modules(input: &str) -> String {
    let mut out = String::new();
    let mut pending_cfg_test = false;
    let mut skipping_test_mod = false;
    let mut brace_depth = 0_i32;

    for line in input.lines() {
        let trimmed = line.trim();

        if skipping_test_mod {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;

            if brace_depth <= 0 {
                skipping_test_mod = false;
                brace_depth = 0;
            }

            continue;
        }

        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            continue;
        }

        if pending_cfg_test
            && (trimmed.starts_with("mod tests") || trimmed.starts_with("pub mod tests"))
        {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;

            if brace_depth > 0 {
                skipping_test_mod = true;
            }

            pending_cfg_test = false;
            continue;
        }

        if pending_cfg_test {
            out.push_str("#[cfg(test)]\n");
            pending_cfg_test = false;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

fn runtime_source_text() -> String {
    let src_root = crate_root().join("src");
    let mut files = Vec::new();
    collect_rust_files(&src_root, &mut files);
    files.sort();

    let mut combined = String::new();

    for path in files {
        let rel = path
            .strip_prefix(crate_root())
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        let stripped = strip_cfg_test_modules(&raw);

        combined.push_str("\n// FILE: ");
        combined.push_str(&rel);
        combined.push('\n');
        combined.push_str(&stripped);
    }

    combined
}

fn route_source_text() -> String {
    let route_root = crate_root().join("src/routes");
    let scan_root = if route_root.is_dir() {
        route_root
    } else {
        crate_root().join("src")
    };

    let mut files = Vec::new();
    collect_rust_files(&scan_root, &mut files);
    files.sort();

    let mut combined = String::new();

    for path in files {
        let rel = path
            .strip_prefix(crate_root())
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

        combined.push_str("\n// FILE: ");
        combined.push_str(&rel);
        combined.push('\n');
        combined.push_str(&strip_cfg_test_modules(&raw));
    }

    combined
}

#[test]
fn docs_lock_gateway_into_qc1a_pair_interlock_role() {
    let docs = read_rel("docs/quickchain-preflight.md");

    assert_contains_all(
        &docs,
        "svc-gateway quickchain-preflight.md",
        &[
            "svc-gateway public route boundary -> omnigate product hydration/access coordination -> svc-wallet mutation front-door -> ron-ledger durable economic truth",
            "client intent -> svc-gateway public boundary -> omnigate quote/access/hydration coordinator -> svc-wallet hold/transfer/capture/release/receipt path -> ron-ledger accepted receipt -> paid unlock/render using backend-derived truth",
            "gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority",
            "gateway is not receipt truth",
            "gateway is not balance truth",
            "gateway is not settlement finality",
            "current paid unlock is backend-derived local access, not future QuickChain epoch inclusion",
        ],
    );
}

#[test]
fn manifest_does_not_add_chain_or_ledger_runtime_dependencies() {
    let cargo = read_rel("Cargo.toml");

    assert_contains_none(
        &cargo,
        "svc-gateway Cargo.toml",
        &[
            "ron-ledger",
            "ron_proto::quickchain",
            "quickchain",
            "solana",
            "anchor-lang",
            "spl-token",
            "ethers",
            "web3",
        ],
    );
}

#[test]
fn runtime_source_does_not_define_direct_economic_or_quickchain_authority() {
    let source = runtime_source_text();

    assert_contains_none(
        &source,
        "svc-gateway runtime source",
        &[
            "ron_ledger::",
            "ron_proto::quickchain",
            "quickchain::",
            "ledger_commit(",
            "append_operation(",
            "apply_operation(",
            "direct_ledger",
            "mint_roc_direct",
            "issue_roc_direct",
            "transfer_roc_direct",
            "burn_roc_direct",
            "hold_roc_direct",
            "capture_roc_direct",
            "release_roc_direct",
            "generate_state_root",
            "generate_receipt_root",
            "produce_checkpoint",
            "sign_checkpoint",
            "validator_signature_for",
            "bridge_settlement",
            "external_settlement",
            "settlement_finality = true",
            "route_level_finality = true",
            "anchor_lang",
            "solana_sdk",
            "solana_client",
            "spl_token",
            "ethers::",
            "web3::",
        ],
    );
}

#[test]
fn public_routes_do_not_expose_live_quickchain_or_settlement_families() {
    let routes = route_source_text();

    assert_contains_none(
        &routes,
        "svc-gateway route source",
        &[
            "\"/quickchain",
            "\"/checkpoint",
            "\"/state-root",
            "\"/receipt-root",
            "\"/validator",
            "\"/validators",
            "\"/bridge",
            "\"/external-settlement",
            "\"/settlement",
            "\"/staking",
            "\"/liquidity",
            "\"/rox",
            "\"/solana",
        ],
    );
}

#[test]
fn dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    assert_contains_all(
        &script,
        "svc-gateway dev-quickchain-preflight.sh",
        &[
            "find \"$TEST_DIR\"",
            "-name 'quickchain*.rs'",
            "basename \"$test_path\" .rs",
            "test -p \"$PKG\" --test \"$test_name\"",
        ],
    );
}
