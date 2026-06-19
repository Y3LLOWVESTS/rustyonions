#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — QuickChain Phase-0 paid-access boundary tests for svc-gateway.
//! RO:WHY — Gateway must fail closed and cannot turn cache/client claims into paid unlocks.
//! RO:INTERACTS — `svc_gateway::headers::proxy`, route source files.
//! RO:INVARIANTS — backend-derived receipt context may be relayed; caller/cache unlock claims are not authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — blocks fake paid/unlock/header/cache entitlement drift.
//! RO:TEST — `cargo test -p svc-gateway --test quickchain_preflight_paid_access`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

const PAID_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-paid",
    "x-ron-unlocked",
    "x-ron-unlock",
    "x-ron-entitlement",
    "x-ron-unlock-authorized",
    "x-ron-cache-unlock",
    "x-ron-local-unlock",
    "x-ron-client-receipt",
    "x-ron-balance",
    "x-ron-finality",
    "x-ron-finalized",
    "x-ron-state-root",
    "x-ron-receipt-root",
    "x-ron-checkpoint-hash",
    "x-ron-validator-signature",
    "x-ron-bridge-settled",
];

const FORBIDDEN_ROUTE_SNIPPETS: &[&str] = &[
    "paid=true",
    "unlock=true",
    "unlocked=true",
    "cache_unlocked",
    "local_unlock",
    "dev_unlock",
    "free_unlock",
    "skip_payment",
    "bypass_payment",
    "trust_client_receipt",
    "trust_cache_receipt",
    "cache_receipt_valid = true",
    "header_receipt_valid = true",
    "entitlement_from_cache",
    "unlock_from_cache",
    "unlock_from_query",
    "unlock_from_header",
];

#[test]
fn client_paid_unlock_headers_are_never_gateway_authority() {
    for &raw in PAID_AUTHORITY_HEADERS {
        let name = HeaderName::from_static(raw);

        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway must not forward caller paid/unlock authority header: {raw}"
        );
        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough must not forward caller paid/unlock authority header: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway must not copy authority-looking paid/unlock response header: {raw}"
        );
    }
}

#[test]
fn backend_receipt_metadata_is_context_not_gateway_paid_authority() {
    for raw in ["x-ron-receipt-id", "x-ron-receipt-hash", "x-ron-quote-id"] {
        let name = HeaderName::from_static(raw);

        assert!(
            proxy::should_forward_product_header(&name),
            "backend receipt/quote metadata may be forwarded for downstream validation: {raw}"
        );
        assert!(
            proxy::should_copy_response_header(&name),
            "backend receipt/quote metadata may be relayed as display context: {raw}"
        );
    }

    for raw in [
        "x-ron-balance",
        "x-ron-finality",
        "x-ron-state-root",
        "x-ron-receipt-root",
        "x-ron-checkpoint-hash",
        "x-ron-validator-signature",
        "x-ron-bridge-settled",
    ] {
        let name = HeaderName::from_static(raw);
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway must not forward balance/finality/root/validator authority as paid truth: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway must not copy balance/finality/root/validator authority as paid truth: {raw}"
        );
    }
}

#[test]
fn route_source_does_not_define_cache_or_client_paid_unlock_shortcuts() {
    let route_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes");
    let mut files = Vec::new();
    collect_rust_files(route_root.as_path(), &mut files);

    let mut offenders = Vec::new();

    for path in files {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read route source {}: {err}", path.display()));
        let lower = text.to_ascii_lowercase();

        for needle in FORBIDDEN_ROUTE_SNIPPETS {
            if lower.contains(needle) {
                offenders.push(format!(
                    "{} contains forbidden paid-unlock shortcut {needle:?}",
                    path.display()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "gateway route source must not define cache/client paid unlock shortcuts:\n{}",
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
