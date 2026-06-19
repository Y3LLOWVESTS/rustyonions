//! RO:WHAT — QuickChain Phase-0 tooling boundary tests for ron-accounting.
//! RO:WHY — Accounting must remain deterministic metering/snapshot tooling, not chain runtime tooling.
//! RO:INTERACTS — crates/ron-accounting/scripts/dev-quickchain-preflight.sh and crate-local tests.
//! RO:INVARIANTS — bash/cargo-only preflight; no Python helpers; exhaustive test discovery; no root/settlement drift.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents hidden helper drift toward roots, validators, settlement, bridges, external anchors, or balance authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_tooling_boundary.

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
        text.contains("ron-accounting quickchain exhaustive preflight gate passed: tests="),
        "preflight script should print an auditable final success line"
    );
}

#[test]
fn quickchain_preflight_script_preserves_accounting_non_authority_posture() {
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
            "ron-accounting QuickChain preflight script must not shell out to {forbidden:?}"
        );
    }

    for required_boundary in [
        "not balance truth",
        "no balance mutation",
        "no wallet/ledger mutation",
        "no roots",
        "checkpoints",
        "validators",
        "settlement",
        "anchors",
        "bridges",
        "staking",
        "liquidity",
        "pruning",
    ] {
        assert!(
            text.contains(required_boundary),
            "preflight script should explicitly preserve accounting boundary marker: {required_boundary}"
        );
    }
}
