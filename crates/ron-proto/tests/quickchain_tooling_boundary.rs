//! RO:WHAT — Tooling-boundary regression tests for ron-proto QuickChain test helpers.
//! RO:WHY — ECON/GOV: QuickChain DTO/vector tooling must stay small, auditable, bash-only, and exhaustive.
//! RO:INTERACTS — crates/ron-proto/scripts, crates/ron-proto/tests/tools, crates/ron-proto/tests/vectors/quickchain.
//! RO:INVARIANTS — no Python helper scripts; required bash verifiers exist; runner discovers all quickchain_*.rs tests; vector inventory stays reviewed.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — tooling tests do not create roots, checkpoints, settlement, receipts, validators, anchors, bridges, or authority.
//! RO:TEST — this file, scripts/dev-quickchain-preflight.sh, and tests/tools/verify_quickchain_*.sh.

use std::{
    fs,
    path::{Path, PathBuf},
};

const EXPECTED_VECTOR_FILES: usize = 36;
const EXPECTED_LOCKED_BYTES: usize = 31;
const EXPECTED_LOCKED_HASH: usize = 5;
const EXPECTED_SKETCH: usize = 0;

const REQUIRED_SCRIPT: &str = "dev-quickchain-preflight.sh";

const REQUIRED_BASH_TOOLS: &[&str] = &[
    "verify_quickchain_vector_inventory.sh",
    "verify_quickchain_hash_payloads.sh",
];

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
        text.contains("find crates/ron-proto/tests -maxdepth 1 -type f -name 'quickchain_*.rs'"),
        "{REQUIRED_SCRIPT}: must discover every quickchain_*.rs integration test"
    );
    assert!(
        text.contains(r#"--test "${test_name}""#),
        "{REQUIRED_SCRIPT}: must run discovered test names, not a hand-curated subset"
    );
    assert!(
        text.contains("bash crates/ron-proto/tests/tools/verify_quickchain_vector_inventory.sh"),
        "{REQUIRED_SCRIPT}: must run the bash vector inventory verifier"
    );
    assert!(
        text.contains("bash crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.sh"),
        "{REQUIRED_SCRIPT}: must run the bash hash payload verifier"
    );
    assert!(
        text.contains("cargo fmt -p ron-proto -- --check"),
        "{REQUIRED_SCRIPT}: must run format check"
    );
    assert!(
        text.contains("cargo clippy -p ron-proto --all-targets -- -D warnings"),
        "{REQUIRED_SCRIPT}: must run clippy"
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
        !text.contains("--test quickchain_hash_payloads\n"),
        "{REQUIRED_SCRIPT}: runner must not regress to a hand-curated quickchain test list"
    );
}

#[test]
fn quickchain_tools_are_bash_only() {
    let tools_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/tools");
    assert!(
        tools_root.is_dir(),
        "QuickChain tests/tools directory must exist"
    );

    let mut files = Vec::new();
    collect_files(&tools_root, &mut files);
    files.sort();

    for required in REQUIRED_BASH_TOOLS {
        assert!(
            files
                .iter()
                .any(|path| path.file_name().and_then(|name| name.to_str()) == Some(*required)),
            "{required} must exist"
        );
    }

    for path in files {
        let rel = relative_key(&tools_root, &path);
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        assert_ne!(
            extension, "py",
            "{rel}: QuickChain helper scripts must stay bash-only"
        );

        if extension == "sh" {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));

            assert!(
                text.starts_with("#!/usr/bin/env bash\n"),
                "{rel}: shell tools must use the bash shebang"
            );
            assert!(
                text.contains("set -euo pipefail"),
                "{rel}: shell tools must fail closed"
            );
            assert!(
                text.contains("RO:WHAT"),
                "{rel}: shell tools must keep the RustyOnions RO header"
            );
        }
    }
}

#[test]
fn ron_proto_contains_no_helper_python_scripts() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_files(manifest, &mut files);
    files.sort();

    for path in files {
        let rel = relative_key(manifest, &path);
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        assert_ne!(
            extension, "py",
            "{rel}: ron-proto helper/tooling files must stay bash-or-Rust only"
        );
    }
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
            .any(|name| name == "quickchain_hash_payloads.rs"),
        "hash-payload truth test must remain in the discovered quickchain suite"
    );
    assert!(
        quickchain_tests
            .iter()
            .any(|name| name == "quickchain_vector_inventory.rs"),
        "vector inventory test must remain in the discovered quickchain suite"
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
            .any(|name| name == "quickchain_locked_bytes.rs"),
        "locked-bytes vector test must remain in the discovered quickchain suite"
    );

    assert!(
        quickchain_tests.len() >= 20,
        "expected at least 20 quickchain integration tests, discovered {}: {:?}",
        quickchain_tests.len(),
        quickchain_tests
    );
}

#[test]
fn quickchain_vector_inventory_counts_match_bash_verifier_contract() {
    let vector_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/quickchain");
    let mut files = Vec::new();
    collect_json_files(&vector_root, &mut files);
    files.sort();

    let mut locked_bytes = 0_usize;
    let mut locked_hash = 0_usize;
    let mut sketch = 0_usize;

    for path in &files {
        let rel = relative_key(&vector_root, path);
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));

        if text.contains(r#""status": "locked_bytes""#) {
            locked_bytes += 1;
            assert!(
                rel.ends_with("_locked_bytes_v1.json"),
                "{rel}: locked_bytes status must match filename"
            );
        } else if text.contains(r#""status": "locked_hash""#) {
            locked_hash += 1;
            assert!(
                rel.ends_with("_locked_hash_v1.json"),
                "{rel}: locked_hash status must match filename"
            );
        } else if text.contains(r#""status": "sketch""#) {
            sketch += 1;
            assert!(
                rel.ends_with("_sketch_v1.json"),
                "{rel}: sketch status must match filename"
            );
        } else {
            panic!("{rel}: vector status must be sketch, locked_bytes, or locked_hash");
        }
    }

    assert_eq!(
        files.len(),
        EXPECTED_VECTOR_FILES,
        "vector file count changed; update the Rust and bash inventory gates after review"
    );
    assert_eq!(locked_bytes, EXPECTED_LOCKED_BYTES);
    assert_eq!(locked_hash, EXPECTED_LOCKED_HASH);
    assert_eq!(sketch, EXPECTED_SKETCH);
}

#[test]
fn quickchain_locked_hash_vectors_have_reviewed_preimage_fields() {
    let vector_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/quickchain");
    let mut files = Vec::new();
    collect_json_files(&vector_root, &mut files);
    files.sort();

    let mut checked = 0_usize;

    for path in files {
        let rel = relative_key(&vector_root, &path);
        if !rel.ends_with("_locked_hash_v1.json") {
            continue;
        }

        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));

        assert!(
            text.contains(r#""status": "locked_hash""#),
            "{rel}: locked hash filename must carry locked_hash status"
        );
        assert!(
            text.contains(r#""canonical_payload_hex": ""#),
            "{rel}: locked_hash vector must carry canonical_payload_hex"
        );
        assert!(
            text.contains(r#""preimage_hex": ""#),
            "{rel}: locked_hash vector must carry preimage_hex"
        );
        assert!(
            text.contains(r#""expected_b3": "b3:"#),
            "{rel}: locked_hash vector must carry expected_b3"
        );
        assert!(
            text.contains(r#""hash_algorithm": "blake3-256""#),
            "{rel}: locked_hash vector must declare blake3-256"
        );
        assert!(
            text.contains(
                r#""preimage_framing": "domain_separator_bytes || 0x00 || canonical_payload_bytes""#
            ),
            "{rel}: locked_hash vector must declare the reviewed preimage framing"
        );

        checked += 1;
    }

    assert_eq!(
        checked, EXPECTED_LOCKED_HASH,
        "locked_hash vector count changed; update bash verifier and inventory gate after review"
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

fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) {
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
            collect_json_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
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
