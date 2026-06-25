//! RO:WHAT — QuickChain Phase-0/Phase-1/Phase-2 tooling boundary tests for svc-wallet.
//! RO:WHY — svc-wallet must stay a wallet service, not a script-driven chain runtime.
//! RO:INTERACTS — crates/svc-wallet/scripts/dev-quickchain-preflight.sh and crate-local test files.
//! RO:INVARIANTS — bash/cargo-only preflight; no Python helpers; exhaustive test discovery; feature-gated QuickChain.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents hidden helper drift toward roots, validators, committees, settlement, bridges, or external authority.
//! RO:TEST — cargo test -p svc-wallet --test quickchain_tooling_boundary.

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
fn no_python_helpers_are_checked_into_svc_wallet() {
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
        "svc-wallet QuickChain preflight must stay bash/cargo-only; found Python helper files: {python_files:?}"
    );
}

#[test]
fn quickchain_preflight_script_discovers_tests_instead_of_hardcoding_the_matrix() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$CRATE_DIR/tests\"",
        "-name 'quickchain*.rs'",
        "quickchain_count",
        "expected at least 14 svc-wallet QuickChain test targets",
        "required_quickchain_tests=(",
        "quickchain_phase1_receipt_root_material_interlock",
        "quickchain_phase2_replay_boundary",
        "quickchain_phase2_committee_boundary",
        "quickchain_phase3_validator_boundary",
        "quickchain_preflight_accounting_observer_boundary",
        "quickchain_preflight_phase1_pair_interlock",
        "cargo test -p svc-wallet --features quickchain-preflight --test \"$test_name\"",
        "svc-wallet quickchain exhaustive preflight gate passed: tests=",
    ] {
        assert!(
            script.contains(required),
            "svc-wallet preflight script must contain required dynamic gate marker: {required}"
        );
    }
}

#[test]
fn quickchain_preflight_script_stays_bash_and_cargo_only() {
    let script = read("scripts/dev-quickchain-preflight.sh");

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
            "svc-wallet QuickChain preflight must remain bash/cargo-only and avoid helper runtime: {forbidden}"
        );
    }
}

#[test]
fn dynamic_discovery_includes_phase2_round2_wallet_tests() {
    let tests = quickchain_test_targets();

    for required in [
        "quickchain_phase1_receipt_root_material_interlock",
        "quickchain_phase2_replay_boundary",
        "quickchain_phase2_committee_boundary",
        "quickchain_phase3_validator_boundary",
        "quickchain_preflight_accounting_observer_boundary",
        "quickchain_preflight_boundary",
        "quickchain_preflight_docs",
        "quickchain_preflight_idempotency_identity_boundary",
        "quickchain_preflight_live_route_matrix",
        "quickchain_preflight_no_runtime_authority",
        "quickchain_preflight_phase1_pair_interlock",
        "quickchain_preflight_projection_validation_matrix",
        "quickchain_preflight_request_poisoning_matrix",
        "quickchain_tooling_boundary",
    ] {
        assert!(
            tests.contains(required),
            "dynamic QuickChain discovery must include svc-wallet test target: {required}"
        );
    }

    assert!(
        tests.len() >= 14,
        "svc-wallet should now have at least 14 QuickChain test targets, got {tests:?}"
    );
}
