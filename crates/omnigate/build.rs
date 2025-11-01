// Build script to embed a short Git commit into the binary as GIT_COMMIT_SHORT.
// Falls back cleanly if `git` is unavailable (e.g., shallow CI clones).

use std::process::Command;

fn main() {
    // Re-run if HEAD moves.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");

    let short = Command::new("git")
        .args(["rev-parse", "--short=9", "HEAD"])
        .output()
        .ok()
        .and_then(|o| o.status.success().then_some(o))
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    if let Some(s) = short {
        println!("cargo:rustc-env=GIT_COMMIT_SHORT={}", s);
    }
}
