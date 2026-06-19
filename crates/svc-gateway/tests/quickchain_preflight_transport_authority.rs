#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — Transport/header authority tests for svc-gateway QuickChain Phase-0.
//! RO:WHY — Public transport metadata must not become wallet, ledger, receipt, root, finality, validator, or bridge authority.
//! RO:INTERACTS — `svc_gateway::headers::proxy`.
//! RO:INVARIANTS — hop-by-hop headers are stripped; identity/retry context is allowed; operation/root/finality claims are stripped.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — blocks caller-supplied authority smuggling through HTTP headers.
//! RO:TEST — `cargo test -p svc-gateway --test quickchain_preflight_transport_authority`.

use http::{
    header::{self, HeaderName},
    HeaderMap,
};
use svc_gateway::headers::proxy;

#[test]
fn hop_by_hop_and_host_headers_are_never_forwarded_or_copied() {
    let denied = [
        header::HOST,
        header::CONNECTION,
        header::PROXY_AUTHENTICATE,
        header::PROXY_AUTHORIZATION,
        header::TE,
        header::TRAILER,
        header::TRANSFER_ENCODING,
        header::UPGRADE,
        header::CONTENT_LENGTH,
    ];

    for name in denied {
        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough must strip hop-by-hop/content-length header: {name}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must strip hop-by-hop/content-length header: {name}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response copy must strip hop-by-hop/content-length header: {name}"
        );
    }
}

#[test]
fn identity_and_retry_headers_are_context_not_authority() {
    for raw in [
        "authorization",
        "x-ron-passport",
        "x-ron-wallet-account",
        "idempotency-key",
        "x-request-id",
        "x-correlation-id",
    ] {
        let name = HeaderName::from_static(raw);
        assert!(
            proxy::should_forward_product_header(&name),
            "gateway may forward identity/retry/context header as non-authoritative context: {raw}"
        );
    }

    for raw in [
        "x-ron-operation-id",
        "x-ron-account-sequence",
        "x-ron-state-root",
        "x-ron-receipt-root",
        "x-ron-checkpoint-hash",
        "x-ron-validator-signature",
        "x-ron-bridge-authorized",
        "x-ron-finality",
        "x-ron-ledger-mutation",
        "x-ron-quickchain-claim",
    ] {
        let name = HeaderName::from_static(raw);
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway must strip authority-looking x-ron header: {raw}"
        );
    }
}

#[test]
fn ordinary_cache_headers_remain_transport_metadata_only() {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "max-age=60".parse().unwrap());
    headers.insert(header::ETAG, "\"b3:aaaaaaaa\"".parse().unwrap());
    headers.insert(header::IF_NONE_MATCH, "\"b3:aaaaaaaa\"".parse().unwrap());

    for name in [
        header::CACHE_CONTROL,
        header::ETAG,
        header::IF_NONE_MATCH,
        header::IF_MODIFIED_SINCE,
        header::IF_UNMODIFIED_SINCE,
    ] {
        assert!(
            proxy::should_copy_response_header(&name),
            "ordinary cache header should remain copyable as transport metadata: {name}"
        );
        assert!(
            !name.as_str().contains("paid") && !name.as_str().contains("receipt"),
            "cache header name must not claim paid/receipt authority: {name}"
        );
    }
}
