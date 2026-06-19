//! RO:WHAT — Tooling boundary tests for svc-rewarder QuickChain Phase-0 gates.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/DX. Preflight must discover all focused QuickChain tests without Python helper drift.
//! RO:INTERACTS — scripts/dev-quickchain-preflight.sh, scripts/dev-quickchain-park.sh, tests/quickchain*.rs.
//! RO:INVARIANTS — bash/cargo only; dynamic test discovery; no roots/checkpoints/validators/settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents the rewarder gate from silently skipping new QuickChain tests.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_tooling_boundary.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn quickchain_test_targets() -> Vec<String> {
    let tests_dir = crate_dir().join("tests");
    let mut targets = fs::read_dir(&tests_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_dir.display()))
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if file_name.starts_with("quickchain") && file_name.ends_with(".rs") {
                Some(file_name.trim_end_matches(".rs").to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    targets.sort();
    targets
}

fn assert_no_python_files(dir: &Path) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
    {
        let entry = entry.unwrap_or_else(|err| panic!("failed to read dir entry: {err}"));
        let path = entry.path();

        if path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "target")
        {
            continue;
        }

        if path.is_dir() {
            assert_no_python_files(&path);
            continue;
        }

        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        assert!(
            !file_name.ends_with(".py") && !file_name.ends_with(".pyc"),
            "QuickChain tooling must stay bash/cargo-only; found Python helper: {}",
            path.display()
        );
    }
}

#[test]
fn preflight_script_discovers_all_quickchain_tests_dynamically() {
    let script = read("scripts/dev-quickchain-preflight.sh");
    let targets = quickchain_test_targets();

    assert!(
        targets.len() >= 6,
        "svc-rewarder should have the existing focused QuickChain suites plus the tooling boundary; found {targets:?}"
    );

    assert!(
        script.contains("find crates/svc-rewarder/tests"),
        "preflight script must discover QuickChain tests from the tests directory"
    );
    assert!(
        script.contains("-name 'quickchain*.rs'"),
        "preflight script must discover quickchain*.rs tests dynamically"
    );
    assert!(
        script.contains("cargo test -p svc-rewarder --test \"$test_name\""),
        "preflight script must run discovered tests through cargo, not a stale hardcoded list"
    );

    for target in targets {
        assert!(
            !script.contains(&format!("cargo test -p svc-rewarder --test {target}\n")),
            "preflight script should not hardcode focused test target {target}; dynamic discovery should cover it"
        );
    }
}

#[test]
fn preflight_script_is_bash_cargo_only_and_keeps_full_gate() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for required in [
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        "cargo fmt -p svc-rewarder -- --check",
        "cargo test -p svc-rewarder --all-targets",
        "cargo clippy -p svc-rewarder --all-targets -- -D warnings",
        "svc-rewarder quickchain exhaustive preflight gate passed: tests=",
        "no roots; no checkpoints; no validators; no settlement; no anchors; no bridges",
    ] {
        assert!(
            script.contains(required),
            "dev-quickchain-preflight.sh missing required marker: {required}"
        );
    }

    for forbidden in ["python ", "python3", "python -", "python3 -"] {
        assert!(
            !script.contains(forbidden),
            "dev-quickchain-preflight.sh must not invoke Python helper tooling: {forbidden}"
        );
    }
}

#[test]
fn no_python_helpers_are_checked_into_rewarder_crate() {
    assert_no_python_files(&crate_dir());
}

#[test]
fn park_script_delegates_to_exhaustive_preflight_gate() {
    let script = read("scripts/dev-quickchain-park.sh");

    for required in [
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        "crates/svc-rewarder/scripts/dev-quickchain-preflight.sh",
        "svc-rewarder QuickChain parking gate passed",
    ] {
        assert!(
            script.contains(required),
            "dev-quickchain-park.sh missing required marker: {required}"
        );
    }
}
