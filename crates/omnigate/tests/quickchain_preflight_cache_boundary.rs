//! RO:WHAT — Cache/hydration boundary tests for omnigate QuickChain Phase-0.
//! RO:WHY — Cache and b3 verification prove bytes, not paid entitlement.
//! RO:INTERACTS — route sources and docs/quickchain-preflight.md.
//! RO:INVARIANTS — no cache-only unlock; no b3/manifest payment proof; ETag/cache headers are transport metadata.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents cache/client claims from bypassing backend receipt truth.
//! RO:TEST — cargo test -p omnigate --test quickchain_preflight_cache_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", full.display());
    })
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn production_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (path, text)
        })
        .collect()
}

#[test]
fn docs_state_cache_and_b3_are_not_paid_entitlement_authority() {
    let docs = read_rel("docs/quickchain-preflight.md");

    for required in [
        "Cache-Control",
        "ETag",
        "If-None-Match",
        "Content-addressing is integrity, not payment proof",
        "b3 proves bytes",
        "b3 does not prove payment",
        "manifest hydration does not prove payment",
        "local receipt cache does not prove payment without backend validation",
        "Paid render/access must be based on backend wallet/ledger truth",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must preserve cache/b3 boundary `{required}`"
        );
    }
}

#[test]
fn production_source_does_not_claim_cache_b3_or_manifest_proves_payment() {
    let forbidden_markers = [
        "cache proves payment",
        "cache proves paid",
        "b3 proves payment",
        "b3 proves paid",
        "manifest proves payment",
        "manifest proves paid",
        "etag proves payment",
        "etag proves paid",
        "local receipt proves payment",
        "local receipt proves paid",
        "offline cache proves payment",
        "offline cache proves paid",
    ];

    for (path, source) in production_sources() {
        let lower = source.to_ascii_lowercase();
        for marker in forbidden_markers {
            assert!(
                !lower.contains(marker),
                "{} must not contain cache/b3 entitlement marker `{marker}`",
                path.display()
            );
        }
    }
}

#[test]
fn paid_access_routes_do_not_parse_transport_cache_headers_as_unlock_authority() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/streams.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/paid.rs",
    ] {
        let source = read_rel(rel);
        let lower = source.to_ascii_lowercase();

        for header in [
            "if-none-match",
            "if-modified-since",
            "if-unmodified-since",
            "cache-control",
            "etag",
        ] {
            let unlock_context = lower.contains(&format!("{header} unlock"))
                || lower.contains(&format!("{header} paid"))
                || lower.contains(&format!("{header} entitlement"));

            assert!(
                !unlock_context,
                "{rel} must not use transport cache header `{header}` as paid unlock authority"
            );
        }
    }
}

#[test]
fn stream_and_chat_sources_explicitly_fail_closed_for_cached_paid_media() {
    let streams = read_rel("src/routes/v1/streams.rs");
    assert!(
        streams.contains("fail-closed receipt validation"),
        "streams route must say receipt validation fails closed"
    );
    assert!(
        streams.contains("viewer media requires wallet receipt lookup"),
        "streams route must require wallet receipt lookup for viewer media"
    );

    let chat = read_rel("src/routes/v1/chat.rs");
    assert!(
        chat.contains("cache never unlocks paid chat"),
        "chat route must preserve cache-never-unlocks-paid-chat invariant"
    );
}
