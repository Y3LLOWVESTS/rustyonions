#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — Tooling boundary tests for svc-gateway QuickChain Phase-0.
//! RO:WHY — Preflight must remain Bash/cargo-only, dynamic, and parkable without hidden helper drift.
//! RO:INTERACTS — `scripts/dev-quickchain-preflight.sh`, `scripts/dev-quickchain-park.sh`, `docs/quickchain-preflight.md`.
//! RO:INVARIANTS — dynamic `quickchain*.rs` discovery; no Python helper drift; clippy/all-target gates.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents bypassing focused boundary tests.
//! RO:TEST — `cargo test -p svc-gateway --test quickchain_tooling_boundary`.

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

#[test]
fn docs_and_runner_files_exist() {
    for rel in [
        "docs/quickchain-preflight.md",
        "scripts/dev-quickchain-preflight.sh",
        "scripts/dev-quickchain-park.sh",
    ] {
        let path = crate_root().join(rel);
        assert!(path.is_file(), "expected file to exist: {}", path.display());
    }
}

#[test]
fn preflight_script_discovers_quickchain_tests_dynamically_and_runs_hard_gates() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "set -euo pipefail",
        "PKG=\"svc-gateway\"",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "basename \"$test_path\" .rs",
        "test -p \"$PKG\" --test \"$test_name\"",
        "test -p \"$PKG\" --all-targets",
        "clippy -p \"$PKG\" --all-targets --no-deps -- -D warnings",
        "== svc-gateway quickchain exhaustive preflight gate passed: tests=",
    ] {
        assert!(
            script.contains(required),
            "preflight script must contain dynamic/hard-gate phrase `{required}`"
        );
    }
}

#[test]
fn preflight_script_lists_known_boundary_suites_for_reviewability() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for suite in [
        "quickchain_preflight_boundary",
        "quickchain_preflight_docs",
        "quickchain_preflight_no_fake_receipts",
        "quickchain_preflight_cache_boundary",
        "quickchain_preflight_paid_access",
        "quickchain_preflight_transport_authority",
        "quickchain_tooling_boundary",
    ] {
        assert!(
            script.contains(suite),
            "preflight script should keep known focused suite visible: {suite}"
        );
    }
}

#[test]
fn park_script_delegates_to_preflight_and_prints_parking_marker() {
    let script = read_rel("scripts/dev-quickchain-park.sh");

    for required in [
        "dev-quickchain-preflight.sh",
        "quickchain_tooling_boundary.rs",
        "== svc-gateway QuickChain parking gate passed ==",
    ] {
        assert!(
            script.contains(required),
            "park script must contain parking contract phrase `{required}`"
        );
    }
}

#[test]
fn no_python_helper_drift_exists_under_crate() {
    let mut offenders = Vec::new();
    collect_files(crate_root().as_path(), &mut offenders);

    offenders.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("py"));

    assert!(
        offenders.is_empty(),
        "svc-gateway QuickChain tooling must remain Bash/cargo-only; unexpected Python files:\n{}",
        offenders
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}
