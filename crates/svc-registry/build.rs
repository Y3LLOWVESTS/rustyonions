// crates/svc-registry/build.rs
use std::process::Command;

fn main() {
    // Re-run if we change this file or git state changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/index");

    // RFC3339 build time (UTC)
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_TIME_RFC3339={}", now);

    // Full + short commit (best-effort)
    let full = run_git(&["rev-parse", "--verify", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let short = run_git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());

    // Dirty flag (any uncommitted changes?)
    let dirty = run_git(&["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if dirty {
        println!("cargo:rustc-env=VERGEN_GIT_DIRTY=1");
    }

    // New envs expected by build_info.rs
    println!("cargo:rustc-env=VERGEN_GIT_SHA={}", full);
    println!("cargo:rustc-env=VERGEN_GIT_SHA_SHORT={}", short);

    // Legacy compatibility (you had these before)
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", full);
    println!("cargo:rustc-env=GIT_COMMIT_SHORT={}", short);
}

fn run_git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    Some(s.trim().to_string())
}
