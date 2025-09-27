// Enforce repository-wide policy: no "sha256" mention in code/docs,
// except for explicit allowlisted files/dirs. We prefer BLAKE3/b3: everywhere.
//
// Allowed:
//  - This test file
//  - TLS helper stubs (e.g., */tls.rs) that may reference sha256 for interop docs
//  - DailyTodo.md (engineering notes)
//  - .git/, target/
//
// Note: keeps scanning the whole workspace (not just this crate).

#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

fn is_texty_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "toml" | "md" | "yml" | "yaml" | "json" | "txt" | "sh" | "lock"
    )
}

fn path_has_component(path: &Path, needle: &str) -> bool {
    path.components().any(|c| match c {
        Component::Normal(s) => s.to_string_lossy().eq_ignore_ascii_case(needle),
        _ => false,
    })
}

fn under_subpath(path: &Path, segment_seq: &[&str]) -> bool {
    // true if all given components occur in order within the path
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();
    let mut i = 0usize;
    for seg in parts {
        if i < segment_seq.len() && seg.eq_ignore_ascii_case(segment_seq[i]) {
            i += 1;
            if i == segment_seq.len() {
                return true;
            }
        }
    }
    false
}

fn is_allowlisted(path: &Path) -> bool {
    // Directories we ignore entirely
    if path_has_component(path, ".git")
        || path_has_component(path, "target")
        || path_has_component(path, ".onions")   // workspace artifacts
        || path_has_component(path, "scripts")   // dev/demo scripts may mention sha256 for tooling
        || path_has_component(path, "testing")   // CI helper scripts
    {
        return true;
    }

    // Specific files we allow
    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if fname.eq_ignore_ascii_case("no_sha256_guard.rs")
        || fname.eq_ignore_ascii_case("dailytodo.md")
        || fname.eq_ignore_ascii_case("todo.md") // root TODO notes
    {
        return true;
    }

    // TLS helpers (interop docs/snippets)
    if fname.eq_ignore_ascii_case("tls.rs") || path_has_component(path, "tls") {
        return true;
    }

    // Kernel test README (notes)
    if fname.eq_ignore_ascii_case("README.md")
        && under_subpath(path, &["crates", "ron-kernel", "tests"])
    {
        return true;
    }

    false
}


fn find_workspace_root(start: &Path) -> PathBuf {
    // Walk up until we find a Cargo.toml that declares a [workspace] table.
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(s) = fs::read_to_string(&candidate) {
                if s.contains("[workspace]") {
                    return dir.to_path_buf();
                }
            }
        }
        cur = dir.parent();
    }
    // Fallback: current crate root.
    start.to_path_buf()
}

fn gather_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if is_allowlisted(&path) {
                // Skip allowlisted dirs/files entirely
                if path.is_dir() {
                    continue;
                }
                if path.is_file() {
                    continue;
                }
            }

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();
                if is_texty_extension(&ext) {
                    out.push(path);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn forbid_sha256_mentions_workspace_wide() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws_root = find_workspace_root(crate_root);

    let mut files = Vec::new();
    gather_files(&ws_root, &mut files).expect("walk workspace");

    let mut hits: Vec<String> = Vec::new();

    for file in files {
        if is_allowlisted(&file) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&file) else {
            // Binary or unreadable; skip.
            continue;
        };
        let lower = contents.to_ascii_lowercase();

        // Look for plain "sha256" or "sha-256"
        if lower.contains("sha256") || lower.contains("sha-256") {
            // Produce line-oriented matches for better diagnostics
            for (idx, line) in lower.lines().enumerate() {
                if line.contains("sha256") || line.contains("sha-256") {
                    hits.push(format!(
                        "{}:{}: matched token \"{}\"",
                        file.display(),
                        idx + 1,
                        if line.contains("sha-256") { "sha-256" } else { "sha256" }
                    ));
                }
            }
        }
    }

    if !hits.is_empty() {
        let mut msg = String::new();
        msg.push_str(
            "Forbidden SHA-256 references found (use BLAKE3 / b3:<hex> instead).\n\nAllowlist:\n  \
             - this test file\n  \
             - TLS helpers (*/tls.rs and tls/ modules)\n  \
             - DailyTodo.md (engineering notes)\n  \
             - .git/ and target/\n\nMatches:\n",
        );
        for h in hits {
            msg.push_str(&h);
            msg.push('\n');
        }
        panic!("{msg}");
    }
}
