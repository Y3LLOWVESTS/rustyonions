// RO:WHAT — build script that stamps a UNIX build time for /version.
// RO:WHY  — Observability/versioning; helps triage running binaries.
// RO:INVARIANTS — Emits MICRONODE_BUILD_UNIX env var always.
fn main() {
    println!("cargo:rustc-env=MICRONODE_BUILD_UNIX={}", chrono_unix());
}

fn chrono_unix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
