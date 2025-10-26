// RO:WHAT
//   Inject GIT_SHA and BUILD_TS envs for /version.
// RO:WHY
//   Make /version useful in dev and CI without runtime shelling.

use std::process::Command;

fn main() {
    // Best-effort short SHA
    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
    {
        if out.status.success() {
            if let Ok(sha) = String::from_utf8(out.stdout) {
                println!("cargo:rustc-env=GIT_SHA={}", sha.trim());
            }
        }
    }
    // UTC-ish timestamp using std
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    println!("cargo:rustc-env=BUILD_TS=unix:{now}");

    // Re-run on HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}
