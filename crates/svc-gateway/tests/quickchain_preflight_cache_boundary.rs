#![allow(clippy::missing_panics_doc)]

//! QuickChain Phase-0 gateway test: cache metadata is never paid unlock truth.
//!
//! RO:WHAT — Proves ordinary cache headers remain transport metadata while
//!           authority-looking paid/unlock/finality headers are stripped.
//! RO:WHY — Offline/browser/cache convenience must not unlock paid content alone.
//! RO:INVARIANTS — ETag/cache headers can optimize transport, but only backend
//!                 wallet/receipt truth can authorize paid access.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_preflight_cache_boundary

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::{header, HeaderName};
use svc_gateway::headers::{etag, proxy};

const FORBIDDEN_ROUTE_SNIPPETS: &[&str] = &[
    "paid=true",
    "unlock=true",
    "cache_unlocked",
    "cache_unlock",
    "local_unlock",
    "x-ron-paid",
    "x-cache-unlocked",
    "x-local-entitlement",
    "receipt_final=true",
];

#[test]
fn cache_headers_are_transport_metadata_not_unlock_authority() {
    let transport_headers = [
        &header::CACHE_CONTROL,
        &header::ETAG,
        &header::IF_NONE_MATCH,
        &header::IF_MODIFIED_SINCE,
        &header::IF_UNMODIFIED_SINCE,
    ];

    for name in transport_headers {
        assert!(
            proxy::should_copy_response_header(name),
            "ordinary cache response metadata should remain transport-only: {name}"
        );
    }

    let rendered =
        etag::etag_from_b3("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    assert!(
        rendered.starts_with("\"b3:"),
        "ETag should quote the b3 address without creating entitlement truth"
    );
    assert!(
        rendered.ends_with('"'),
        "ETag should be an HTTP quoted string, not a paid-access token"
    );
}

#[test]
fn cache_or_client_unlock_signals_are_stripped_when_they_claim_authority() {
    let authority_headers = [
        "x-ron-entitlement",
        "x-ron-unlock-authorized",
        "x-ron-finality",
        "x-ron-finalized",
        "x-ron-balance",
        "x-ron-state-root",
        "x-ron-receipt-root",
        "x-ron-quickchain-unlock",
    ];

    for raw in authority_headers {
        let name = HeaderName::from_static(raw);

        assert!(
            !proxy::should_forward_product_header(&name),
            "client-provided paid/cache authority header must be stripped: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "authority-looking response header must not become gateway truth: {raw}"
        );
    }
}

#[test]
fn route_source_does_not_unlock_paid_content_from_cache_or_query_claims() {
    let route_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes");
    let mut files = Vec::new();
    collect_rust_files(route_root.as_path(), &mut files);

    let mut offenders = Vec::new();

    for path in files {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read route source {}: {err}", path.display()));

        for needle in FORBIDDEN_ROUTE_SNIPPETS {
            if text.contains(needle) {
                offenders.push(format!(
                    "{} contains forbidden snippet {needle:?}",
                    path.display()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "gateway route source must not define cache/query/client paid unlock shortcuts:\n{}",
        offenders.join("\n")
    );
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
