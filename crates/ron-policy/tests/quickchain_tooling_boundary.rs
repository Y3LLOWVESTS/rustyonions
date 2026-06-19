//! RO:WHAT — Tooling boundary tests for ron-policy QuickChain Phase-0 parking.
//! RO:WHY — Keeps the crate-local preflight/parking gate repeatable and Bash-only.
//! RO:INTERACTS — docs/quickchain-preflight.md and scripts/dev-quickchain-*.sh.
//! RO:INVARIANTS — tooling may verify boundaries but must not add roots/checkpoints/validators/settlement behavior.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[test]
fn quickchain_tooling_files_exist() {
    for rel in [
        "docs/quickchain-preflight.md",
        "scripts/dev-quickchain-preflight.sh",
        "scripts/dev-quickchain-park.sh",
    ] {
        let path = crate_path(rel);
        assert!(
            path.exists(),
            "missing QuickChain tooling file: {}",
            path.display()
        );
    }
}

#[test]
fn preflight_script_is_bash_strict_and_dynamic() {
    let script = read_crate_file("scripts/dev-quickchain-preflight.sh");

    for phrase in [
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        r#"CARGO="${CARGO:-cargo}""#,
        "quickchain*.rs",
        r#"find "$TEST_DIR""#,
        r#"--test "$test_name""#,
        r#""$CARGO" fmt"#,
        "--check",
        r#""$CARGO" test -p "$PKG" --test "$test_name""#,
        r#""$CARGO" test -p "$PKG" --all-targets"#,
        r#""$CARGO" clippy -p "$PKG" --all-targets --no-deps -- -D warnings"#,
        "economics_policy",
        "unit_model_serde_strict",
        "unit_eval_determinism",
        "unit_first_match_wins",
        "golden_reasons",
        "forbidden Python helper drift",
        "ron-policy QuickChain Phase-0 preflight passed.",
    ] {
        assert!(
            script.contains(phrase),
            "ron-policy preflight script missing expected phrase: {phrase}"
        );
    }
}

#[test]
fn park_script_delegates_to_preflight() {
    let script = read_crate_file("scripts/dev-quickchain-park.sh");

    for phrase in [
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        "dev-quickchain-preflight.sh",
        "docs/quickchain-preflight.md",
        "quickchain_tooling_boundary.rs",
        "== ron-policy QuickChain parking gate passed ==",
    ] {
        assert!(
            script.contains(phrase),
            "ron-policy park script missing expected phrase: {phrase}"
        );
    }
}

#[test]
fn crate_has_no_python_helper_drift() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut python_files = Vec::new();
    collect_python_files(root, &mut python_files);
    python_files.sort();

    assert!(
        python_files.is_empty(),
        "ron-policy QuickChain tooling must stay Bash/Rust-only; found Python files: {python_files:?}"
    );
}

fn crate_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn read_crate_file(rel: &str) -> String {
    let path = crate_path(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn collect_python_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir
        .components()
        .any(|component| component.as_os_str() == OsStr::new("target"))
    {
        return;
    }

    for entry in
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
    {
        let entry = entry.unwrap_or_else(|err| panic!("read_dir entry {}: {err}", dir.display()));
        let path = entry.path();

        if path.is_dir() {
            collect_python_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == OsStr::new("py"))
        {
            out.push(path);
        }
    }
}
