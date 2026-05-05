//! RO:WHAT — I-12 canonical dependency guard for `ron-app-sdk`.
//! RO:WHY — Keeps the SDK tied to canonical RustyOnions crates instead of drifting into duplicate owners.
//! RO:INTERACTS — Cargo.toml, ron-proto, reqwest/rustls transport stack.
//! RO:INVARIANTS — SDK depends on DTOs via ron-proto; no service crates become SDK dependencies.
//! RO:SECURITY — TLS stack should remain rustls-oriented; no ambient native-tls dependency is added here.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::fs;

#[test]
fn sdk_cargo_manifest_keeps_canonical_dependency_shape() {
    let manifest_path = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {manifest_path}: {err}"));

    assert!(
        manifest.contains("ron-proto"),
        "ron-app-sdk must depend on ron-proto for canonical DTOs"
    );
    assert!(
        manifest.contains("reqwest"),
        "ron-app-sdk should keep using a thin HTTP client transport during this phase"
    );
    assert!(
        manifest.contains("rustls-tls") || manifest.contains("rustls"),
        "ron-app-sdk transport should remain rustls-oriented"
    );

    let forbidden_service_deps = [
        "svc-gateway",
        "omnigate",
        "svc-storage",
        "svc-index",
        "svc-wallet",
        "ron-ledger",
    ];

    for dep in forbidden_service_deps {
        assert!(
            !manifest_contains_dependency(&manifest, dep),
            "ron-app-sdk should not directly depend on service crate `{dep}`"
        );
    }
}

fn manifest_contains_dependency(manifest: &str, dep: &str) -> bool {
    manifest
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with(&format!("{dep} =")))
}
