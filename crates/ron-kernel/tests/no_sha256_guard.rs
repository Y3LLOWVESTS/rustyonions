// crates/ron-kernel/tests/no_sha256_guard.rs
#![forbid(unsafe_code)]

use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

/// Banned tokens anywhere in the repo (code, docs, scripts),
/// excluding a tiny allowlist (TLS helpers / OpenSSL config) and this test file.
const NEEDLES: &[&str] = &["sha-256", "sha 256", "sha256:", "sha256"];

/// Files where the token is allowed (OpenSSL cert generation only),
/// plus this test file (which necessarily mentions the tokens).
const ALLOWLIST_FILES: &[&str] = &[
    "scripts/run_tile_demo.sh",
    "scripts/run_mailbox_demo.sh",
    "scripts/dev_tls_setup_mac.sh",
    "testing/tls/openssl-server.cnf",
    "crates/ron-kernel/tests/no_sha256_guard.rs",
];

/// Directories to skip entirely.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".cargo",
    ".idea",
    ".vscode",
    ".onions", // test artifacts
];

#[test]
fn no_sha256_anywhere_in_repo_except_tls_helpers() -> Result<(), Box<dyn Error>> {
    let root = find_workspace_root()?;
    let allowlist: Vec<PathBuf> = ALLOWLIST_FILES.iter().map(|p| root.join(p)).collect();

    let mut violations: Vec<String> = Vec::new();
    walk(&root, &mut |path| {
        // Skip allowlisted files entirely.
        if allowlist.iter().any(|p| p == path) {
            return Ok(());
        }
        // Only scan regular UTF-8 text files.
        let meta = fs::metadata(path)?;
        if !meta.is_file() {
            return Ok(());
        }
        let bytes = fs::read(path)?;
        let content = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => return Ok(()), // skip non-UTF8
        };
        for (lineno, line) in content.lines().enumerate() {
            for n in NEEDLES {
                if line.contains(n) {
                    violations.push(format!(
                        "{}:{}: matched token {:?}",
                        rel(&root, path).display(),
                        lineno + 1,
                        n
                    ));
                }
            }
        }
        Ok(())
    })?;

    if !violations.is_empty() {
        let msg = format!(
            "Forbidden SHA-256 references found (use BLAKE3 / b3:<hex> instead), \
             excluding only TLS helper files and this test file:\n{}",
            violations.join("\n")
        );
        return Err(msg.into());
    }

    Ok(())
}

fn walk<F>(dir: &Path, f: &mut F) -> io::Result<()>
where
    F: FnMut(&Path) -> io::Result<()>,
{
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            walk(&path, f)?;
        } else {
            f(&path)?;
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| SKIP_DIRS.contains(&n))
        .unwrap_or(false)
}

fn rel(root: &Path, p: &Path) -> PathBuf {
    p.strip_prefix(root).unwrap_or(p).to_path_buf()
}

fn find_workspace_root() -> io::Result<PathBuf> {
    let mut cur = env::current_dir()?;
    loop {
        if cur.join("Cargo.lock").exists() {
            return Ok(cur);
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Cargo.lock not found in any parent directories",
    ))
}
