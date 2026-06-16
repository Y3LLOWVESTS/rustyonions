//! RO:WHAT — Public route boundary tests for svc-index QuickChain Phase-0.
//! RO:WHY — Ensure svc-index exposes lookup/pointer routes only, not roots/checkpoints/validators/settlement/bridges.
//! RO:INTERACTS — src/router.rs and src/http/routes/*.rs.
//! RO:INVARIANTS — no public QuickChain, bridge, staking, liquidity, ROX, Solana, or settlement route surface.
//! RO:TEST — run with `cargo test -p svc-index --test quickchain_preflight_routes`.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }

    let entries = fs::read_dir(root)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", root.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn strip_rust_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut idx = 0;
    let mut in_block = false;

    while idx < bytes.len() {
        if in_block {
            if idx + 1 < bytes.len() && bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                in_block = false;
                idx += 2;
            } else {
                if bytes[idx] == b'\n' {
                    out.push('\n');
                }
                idx += 1;
            }
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            in_block = true;
            idx += 2;
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            while idx < bytes.len() && bytes[idx] != b'\n' {
                idx += 1;
            }
            if idx < bytes.len() {
                out.push('\n');
                idx += 1;
            }
            continue;
        }

        out.push(bytes[idx] as char);
        idx += 1;
    }

    out
}

fn route_sources_without_comments() -> Vec<(PathBuf, String)> {
    let mut files = vec![manifest_dir().join("src/router.rs")];
    collect_rust_files(&manifest_dir().join("src/http/routes"), &mut files);

    files
        .into_iter()
        .filter(|path| path.exists())
        .map(|path| {
            let body = strip_rust_comments(&read(&path)).to_lowercase();
            (path, body)
        })
        .collect()
}

#[test]
fn public_routes_do_not_expose_quickchain_settlement_surfaces() {
    let forbidden_route_fragments = [
        "\"/quickchain",
        "\"/v1/quickchain",
        "\"/roots",
        "\"/root",
        "\"/state-root",
        "\"/receipt-root",
        "\"/accounting-root",
        "\"/reward-root",
        "\"/checkpoint",
        "\"/checkpoints",
        "\"/validator",
        "\"/validators",
        "\"/settlement",
        "\"/settle",
        "\"/bridge",
        "\"/anchors",
        "\"/anchor",
        "\"/staking",
        "\"/stake",
        "\"/liquidity",
        "\"/rox",
        "\"/solana",
        "\"/finality",
        "\"/entitlement",
        "\"/unlock",
    ];

    for (path, source) in route_sources_without_comments() {
        for forbidden in forbidden_route_fragments {
            assert!(
                !source.contains(forbidden),
                "{} must not expose forbidden QuickChain/value-plane public route fragment {forbidden:?}",
                path.display()
            );
        }
    }
}

#[test]
fn allowed_routes_remain_lookup_and_pointer_or_admin_health_surfaces() {
    let router = strip_rust_comments(&read(manifest_dir().join("src/router.rs"))).to_lowercase();

    for expected in [
        "\"/healthz\"",
        "\"/readyz\"",
        "\"/version\"",
        "\"/metrics\"",
        "\"/resolve/:key\"",
        "\"/providers/:cid\"",
        "\"/v1/index/assets/:asset_cid/manifest\"",
        "\"/v1/index/sites/:name/manifest\"",
    ] {
        assert!(
            router.contains(expected),
            "router should keep expected lookup/pointer/admin route {expected}"
        );
    }
}
