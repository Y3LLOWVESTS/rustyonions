//! RO:WHAT — Crate version/commit surface for svc-storage.
//! RO:WHY  — Used by /version and logs for precise diagnostics.
//! RO:INVARIANTS — Always returns a stable ASCII string; safe to expose publicly.

pub fn version_string() -> String {
    // Prefer build-time env if present; fall back to Cargo package version.
    let pkg = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    let git = option_env!("GIT_COMMIT_HASH").unwrap_or("unknown");
    let build = option_env!("BUILD_TS").unwrap_or("unknown");
    format!("{name} {pkg} (git:{git}, built:{build})")
}
