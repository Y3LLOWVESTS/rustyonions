#![allow(clippy::missing_panics_doc)]

//! QuickChain Phase-0 gateway test: no fake economic or chain authority may
//! cross the public gateway boundary as request or response headers.
//!
//! RO:WHAT — Proves gateway strips fake balances, fake unlocks, roots, finality,
//!           validator, bridge, staking, liquidity, and external-settlement headers.
//! RO:WHY — Gateway is a public proxy/admission boundary, not wallet, ledger,
//!          receipt, entitlement, finality, bridge, or validator authority.
//! RO:INVARIANTS — backend receipt metadata may be relayed as display/backend
//!                 context, but gateway must not treat client/root/finality
//!                 headers as authority.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_preflight_no_fake_receipts

use http::HeaderName;
use svc_gateway::headers::proxy;

const AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-account-sequence",
    "x-ron-operation-id",
    "x-ron-state-root",
    "x-ron-receipt-root",
    "x-ron-accounting-root",
    "x-ron-reward-root",
    "x-ron-checkpoint-root",
    "x-ron-checkpoint-hash",
    "x-ron-data-availability-root",
    "x-ron-validator-set",
    "x-ron-validator-signature",
    "x-ron-finality",
    "x-ron-finalized",
    "x-ron-anchored",
    "x-ron-anchor",
    "x-ron-bridge",
    "x-ron-external-settlement",
    "x-ron-staking",
    "x-ron-liquidity",
    "x-ron-balance",
    "x-ron-entitlement",
    "x-ron-unlock-authorized",
    "x-ron-quickchain-root",
    "x-ron-quickchain-finality",
    "x-ron-validator-approval",
    "x-ron-bridge-settlement",
    "x-ron-anchor-proof",
    "x-ron-checkpoint-proof",
    "x-ron-root-proof",
    "x-ron-ledger-proof",
];

#[test]
fn response_headers_cannot_smuggle_economic_or_chain_authority() {
    for &raw in AUTHORITY_HEADERS {
        let name = HeaderName::from_static(raw);

        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway must not copy authority-looking response header: {raw}"
        );
    }
}

#[test]
fn request_headers_cannot_smuggle_economic_or_chain_authority() {
    for &raw in AUTHORITY_HEADERS {
        let name = HeaderName::from_static(raw);

        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must not forward authority-looking request header: {raw}"
        );

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must not forward authority-looking request header: {raw}"
        );
    }
}

#[test]
fn backend_receipt_metadata_remains_display_context_not_gateway_authority() {
    let receipt_id = HeaderName::from_static("x-ron-receipt-id");
    let receipt_hash = HeaderName::from_static("x-ron-receipt-hash");
    let quote_id = HeaderName::from_static("x-ron-quote-id");

    assert!(
        proxy::should_copy_response_header(&receipt_id),
        "backend receipt id metadata may be relayed as display context"
    );
    assert!(
        proxy::should_copy_response_header(&receipt_hash),
        "backend receipt hash metadata may be relayed as display context"
    );
    assert!(
        proxy::should_copy_response_header(&quote_id),
        "backend quote id metadata may be relayed as display context"
    );

    assert!(
        proxy::should_forward_product_header(&receipt_id),
        "receipt metadata may reach omnigate for backend validation"
    );
    assert!(
        proxy::should_forward_product_header(&receipt_hash),
        "receipt hash metadata may reach omnigate for backend validation"
    );
    assert!(
        proxy::should_forward_product_header(&quote_id),
        "quote metadata may reach omnigate for backend validation"
    );
}
