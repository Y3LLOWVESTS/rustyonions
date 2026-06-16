//! RO:WHAT — Tooling and feature-boundary regression tests for ron-ledger QuickChain preflight.
//! RO:WHY — ECON/RES/GOV: QuickChain work must stay gated, bash-only, exhaustive, and pre-root until vectors authorize more.
//! RO:INTERACTS — crates/ron-ledger/scripts, crates/ron-ledger/tests/quickchain_*.rs.
//! RO:INVARIANTS — no Python helper scripts; all QuickChain integration tests are feature-gated; runner discovers all quickchain_*.rs tests.
//! RO:METRICS — none.
//! RO:CONFIG — requires quickchain-preflight.
//! RO:SECURITY — does not create roots, checkpoints, validators, settlement, anchors, signatures, or authority.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_tooling_boundary.

#![cfg(feature = "quickchain-preflight")]

use std::{
    fs,
    path::{Path, PathBuf},
};

const REQUIRED_SCRIPT: &str = "dev-quickchain-preflight.sh";
const REQUIRED_FEATURE_GATE: &str = r#"#![cfg(feature = "quickchain-preflight")]"#;

#[test]
fn quickchain_preflight_runner_is_bash_only_and_exhaustive() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest.join("scripts").join(REQUIRED_SCRIPT);

    assert!(
        script.is_file(),
        "{} must exist",
        script.strip_prefix(manifest).unwrap_or(&script).display()
    );

    let text = fs::read_to_string(&script)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", script.display()));

    assert!(
        text.starts_with("#!/usr/bin/env bash\n"),
        "{REQUIRED_SCRIPT}: must use bash shebang"
    );
    assert!(
        text.contains("set -euo pipefail"),
        "{REQUIRED_SCRIPT}: must fail closed"
    );
    assert!(
        text.contains("RO:WHAT"),
        "{REQUIRED_SCRIPT}: must keep RO header"
    );
    assert!(
        text.contains("find crates/ron-ledger/tests -maxdepth 1 -type f -name 'quickchain_*.rs'"),
        "{REQUIRED_SCRIPT}: must discover every quickchain_*.rs integration test"
    );
    assert!(
        text.contains(r#"--test "${test_name}""#),
        "{REQUIRED_SCRIPT}: must run discovered test names, not a hand-curated subset"
    );
    assert!(
        text.contains("--features quickchain-preflight"),
        "{REQUIRED_SCRIPT}: must run the gated QuickChain feature"
    );
    assert!(
        text.contains("cargo fmt -p ron-ledger -- --check"),
        "{REQUIRED_SCRIPT}: must run format check"
    );
    assert!(
        text.contains("cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings"),
        "{REQUIRED_SCRIPT}: must run feature-gated clippy"
    );

    let lowered = text.to_ascii_lowercase();
    for forbidden in [
        "cargo test --workspace",
        "cargo clippy --workspace",
        "python",
        "python3",
        "curl ",
        "wget ",
        "npm ",
        "node ",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "{REQUIRED_SCRIPT}: focused preflight runner must not contain forbidden token {forbidden:?}"
        );
    }

    assert!(
        !text.contains("--test quickchain_projection_context_boundaries"),
        "{REQUIRED_SCRIPT}: runner must not regress to a hand-curated quickchain test list"
    );
}

#[test]
fn ron_ledger_contains_no_helper_python_scripts() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_files(manifest, &mut files);
    files.sort();

    for path in files {
        let rel = relative_key(manifest, &path);
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        assert_ne!(
            extension, "py",
            "{rel}: ron-ledger helper/tooling files must stay bash-or-Rust only"
        );
    }
}

#[test]
fn every_quickchain_integration_test_is_feature_gated() {
    let tests_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    collect_files(&tests_root, &mut files);
    files.sort();

    let mut checked = 0_usize;

    for path in files {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !name.starts_with("quickchain_") || !name.ends_with(".rs") {
            continue;
        }

        let rel = relative_key(&tests_root, &path);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));

        assert!(
            text.contains(REQUIRED_FEATURE_GATE),
            "{rel}: QuickChain integration tests must remain behind quickchain-preflight"
        );

        checked += 1;
    }

    assert!(
        checked >= 20,
        "expected the existing ron-ledger QuickChain preflight suite to be present; checked only {checked}"
    );
}

#[test]
fn quickchain_test_discovery_count_matches_runner_contract() {
    let tests_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    collect_files(&tests_root, &mut files);
    files.sort();

    let quickchain_tests: Vec<String> = files
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .filter(|name| name.starts_with("quickchain_") && name.ends_with(".rs"))
        .map(str::to_owned)
        .collect();

    assert!(
        quickchain_tests
            .iter()
            .any(|name| name == "quickchain_pre_root_boundary.rs"),
        "pre-root authority boundary test must remain in the discovered quickchain suite"
    );
    assert!(
        quickchain_tests
            .iter()
            .any(|name| name == "quickchain_tooling_boundary.rs"),
        "tooling boundary test must remain in the discovered quickchain suite"
    );
    assert!(
        quickchain_tests
            .iter()
            .any(|name| name == "quickchain_projection_replay_equality.rs"),
        "projection replay equality test must remain in the discovered quickchain suite"
    );

    assert!(
        quickchain_tests.len() >= 20,
        "expected at least 20 quickchain integration tests, discovered {}: {:?}",
        quickchain_tests.len(),
        quickchain_tests
    );
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read directory entry: {error}"))
                .path()
        })
        .collect();

    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn relative_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
