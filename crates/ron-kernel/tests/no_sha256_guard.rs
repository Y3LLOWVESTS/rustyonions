#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Walk upward from this crate to locate the workspace root (the directory
/// whose Cargo.toml contains a `[workspace]` section).
fn find_workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let toml = dir.join("Cargo.toml");
        if toml.exists() {
            if let Ok(s) = fs::read_to_string(&toml) {
                if s.contains("[workspace]") {
                    return dir;
                }
            }
        }
        if !dir.pop() {
            panic!("could not find workspace root with [workspace] Cargo.toml");
        }
    }
}

fn should_scan(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    // avoid false positives in lockfiles or this test itself
    if name == "Cargo.lock" || name == "no_sha256_guard.rs" {
        return false;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    matches!(
        ext,
        "rs" | "toml" | "md" | "yml" | "yaml" | "json" | "sh" | "bash" | "fish" | "zsh" | "txt"
    )
}

fn skip_dir(dir: &Path) -> bool {
    let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    matches!(
        name,
        "target" | ".git" | ".github" | ".vscode" | ".idea" | "node_modules" | "dist" | "build"
            | "coverage" | "venv" | ".onions" | "scripts" | "testing"
    )
}

/// TEMPORARY allowlist while we migrate everything to BLAKE3.
/// Remove entries as you fix the files.
fn is_allowlisted(path: &Path, ws_root: &Path) -> bool {
    let rel = path.strip_prefix(ws_root).unwrap_or(path);
    let rel = rel.to_string_lossy();

    // Keep this list as short as possible, and shrink it aggressively.
    const ALLOW: &[&str] = &[
        // TODO: remove once migrated in the naming crate
        "crates/naming/src/address.rs",
        "crates/naming/src/hash.rs",
        // TODO: remove once docs updated
        "docs/blueprints/Scaling_Blueprint.md",
        "docs/blueprints/Microkernel_Blueprint.md",
        // TODO: optional housekeeping and backlog files
        "TODO.md",
    ];

    ALLOW.iter().any(|p| rel == *p)
}

#[test]
fn no_sha256_anywhere_in_repo() {
    let root = find_workspace_root();

    // Lowercase substrings to search for (space, dash, or colon variants).
    const NEEDLES: &[&str] = &["sha-256", "sha 256", "sha256:", "sha256"];

    let mut offenders = Vec::<String>::new();
    let mut stack = vec![root.clone()];

    while let Some(dir) = stack.pop() {
        if skip_dir(&dir) {
            continue;
        }
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !should_scan(&path) {
                continue;
            }
            if is_allowlisted(&path, &root) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                // Non-UTF8 or unreadable — skip
                continue;
            };
            let lc = text.to_lowercase();
            if NEEDLES.iter().any(|n| lc.contains(n)) {
                let rel = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
                offenders.push(rel);
            }
        }
    }

    if !offenders.is_empty() {
        panic!(
            "Forbidden SHA-256 references found (use BLAKE3 / b3:<hex> instead):\n{}",
            offenders.join("\n")
        );
    }
}
