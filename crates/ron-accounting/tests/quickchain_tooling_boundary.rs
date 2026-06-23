//! RO:WHAT — QuickChain Phase-0/Phase-1 tooling boundary tests for ron-accounting.
//! RO:WHY — Accounting must remain deterministic metering/snapshot tooling, not chain runtime tooling.
//! RO:INTERACTS — crates/ron-accounting/scripts/dev-quickchain-preflight.sh and crate-local tests.
//! RO:INVARIANTS — bash/cargo-only preflight; no Python helpers; exhaustive test discovery; no root/settlement drift.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents hidden helper drift toward roots, validators, settlement, bridges, external anchors, or balance authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_tooling_boundary.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
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
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn quickchain_test_targets() -> BTreeSet<String> {
    let mut files = Vec::new();
    collect_files(&crate_dir().join("tests"), &mut files);

    files
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_string();
            if name.starts_with("quickchain") && name.ends_with(".rs") {
                Some(name.trim_end_matches(".rs").to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn no_python_helpers_are_checked_into_ron_accounting() {
    let mut files = Vec::new();
    collect_files(&crate_dir(), &mut files);

    let python_files = files
        .iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| matches!(ext, "py" | "pyi"))
        })
        .collect::<Vec<_>>();

    assert!(
        python_files.is_empty(),
        "ron-accounting QuickChain preflight must stay bash/cargo-only; found Python helper files: {python_files:?}"
    );
}

#[test]
fn quickchain_preflight_script_discovers_tests_instead_of_hardcoding_the_matrix() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$CRATE_DIR/tests\"",
        "-name 'quickchain*.rs'",
        "quickchain_count",
        "expected at least 11 ron-accounting QuickChain test targets",
        "required_quickchain_tests=(",
        "quickchain_phase1_root_material_non_authority",
        "quickchain_preflight_wallet_interlock_boundary",
        "quickchain_preflight_phase1_pair_interlock",
        "\"$CARGO\" test -p ron-accounting --test \"$test_name\"",
        "ron-accounting quickchain exhaustive preflight gate passed: tests=",
    ] {
        assert!(
            script.contains(required),
            "ron-accounting preflight script must contain required dynamic gate marker: {required}"
        );
    }
}

#[test]
fn quickchain_preflight_script_preserves_accounting_non_authority_posture() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for required in [
        "balance truth",
        "wallet mutation",
        "ledger mutation",
        "roots",
        "validators",
        "settlement",
        "external anchors",
        "bridges",
        "staking",
        "liquidity",
        "pruning",
        "live chain authority",
        "no fake balances",
        "no fake receipts",
    ] {
        assert!(
            script.contains(required),
            "ron-accounting preflight script must preserve non-authority marker: {required}"
        );
    }

    for forbidden in [
        "python ",
        "python3",
        "node ",
        "npm ",
        "npx ",
        "ts-node",
        "cargo run --bin quickchain",
    ] {
        assert!(
            !script.contains(forbidden),
            "ron-accounting QuickChain preflight must remain bash/cargo-only and avoid helper runtime: {forbidden}"
        );
    }
}

#[test]
fn dynamic_discovery_includes_phase1_round2_accounting_tests() {
    let tests = quickchain_test_targets();

    for required in [
        "quickchain_phase1_root_material_non_authority",
        "quickchain_preflight_boundary",
        "quickchain_preflight_docs",
        "quickchain_preflight_event_class_boundary",
        "quickchain_preflight_ingest_poisoning",
        "quickchain_preflight_phase1_pair_interlock",
        "quickchain_preflight_reward_dto_strictness",
        "quickchain_preflight_reward_projection_boundary",
        "quickchain_preflight_snapshot_non_authority",
        "quickchain_preflight_wallet_interlock_boundary",
        "quickchain_tooling_boundary",
    ] {
        assert!(
            tests.contains(required),
            "dynamic QuickChain discovery must include ron-accounting test target: {required}"
        );
    }

    assert!(
        tests.len() >= 11,
        "ron-accounting should now have at least 11 QuickChain test targets, got {tests:?}"
    );
}
