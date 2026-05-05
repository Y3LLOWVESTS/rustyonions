//! RO:WHAT — I-10 semver snapshot smoke check for `ron-app-sdk`.
//! RO:WHY — Guards that the crate exposes a non-empty package identity for API snapshot tooling.
//! RO:INTERACTS — Cargo package metadata and future public API snapshot checks.
//! RO:INVARIANTS — Package name/version are stable enough to drive semver gates.
//! RO:SECURITY — No runtime I/O or secrets.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

#[test]
fn package_metadata_is_available_for_semver_snapshot_gate() {
    let package_name = env!("CARGO_PKG_NAME");
    let package_version = env!("CARGO_PKG_VERSION");

    assert_eq!(package_name, "ron-app-sdk");
    assert!(
        !package_version.trim().is_empty(),
        "CARGO_PKG_VERSION must be present for semver snapshot tooling"
    );

    let parts: Vec<&str> = package_version.split('.').collect();
    assert!(
        parts.len() >= 3,
        "expected semver-like version with at least major.minor.patch, got {package_version}"
    );
}
