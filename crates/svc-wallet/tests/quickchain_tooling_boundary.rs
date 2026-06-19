//! RO:WHAT — QuickChain Phase-0 tooling boundary tests for svc-wallet.
//! RO:WHY — svc-wallet must stay a wallet service, not a script-driven chain runtime.
//! RO:INTERACTS — crates/svc-wallet/scripts/dev-quickchain-preflight.sh and crate-local test files.
//! RO:INVARIANTS — bash/cargo-only preflight; no Python helpers; exhaustive test discovery; feature-gated QuickChain.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents hidden helper drift toward roots, validators, settlement, bridges, or external authority.
//! RO:TEST — cargo test -p svc-wallet --test quickchain_tooling_boundary.

use std::{
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

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
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
    let script = crate_dir()
        .join("scripts")
        .join("dev-quickchain-preflight.sh");
    let text = read_to_string(&script);

    assert!(
        text.contains("find \"$CRATE_DIR/tests\""),
        "preflight script should discover crate-local QuickChain tests"
    );
    assert!(
        text.contains("-name 'quickchain*.rs'"),
        "preflight script should discover every quickchain*.rs integration target"
    );
    assert!(
        text.contains("quickchain_count"),
        "preflight script should count discovered QuickChain tests"
    );
    assert!(
        text.contains("svc-wallet quickchain exhaustive preflight gate passed: tests="),
        "preflight script should print an auditable final success line"
    );
}

#[test]
fn quickchain_preflight_script_stays_bash_and_cargo_only() {
    let script = crate_dir()
        .join("scripts")
        .join("dev-quickchain-preflight.sh");
    let text = read_to_string(&script);

    assert!(
        text.starts_with("#!/usr/bin/env bash"),
        "preflight script must use bash"
    );

    for forbidden in [
        "python ", "python3 ", "node ", "npm ", "npx ", "bun ", "ts-node", "deno ",
    ] {
        assert!(
            !text.contains(forbidden),
            "svc-wallet QuickChain preflight script must not shell out to {forbidden:?}"
        );
    }

    for forbidden_scope in [
        "validator",
        "settlement",
        "external anchors",
        "bridges",
        "staking",
        "liquidity",
        "live chain authority",
    ] {
        assert!(
            text.contains(forbidden_scope),
            "preflight script should explicitly preserve forbidden scope marker: {forbidden_scope}"
        );
    }
}
