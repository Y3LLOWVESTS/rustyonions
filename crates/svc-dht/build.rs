use std::process::Command;

fn main() {
    // Best-effort short git SHA
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Build timestamp (UTC, RFC3339)
    let ts = chrono::Utc::now().to_rfc3339();

    println!("cargo:rustc-env=BUILD_GIT_SHA={}", sha);
    println!("cargo:rustc-env=BUILD_TS={}", ts);
}
