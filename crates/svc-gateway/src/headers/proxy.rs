//! Proxy header filtering for public gateway → upstream service hops.
//!
//! RO:WHAT — Central allow/deny helpers for headers forwarded by gateway proxy routes.
//! RO:WHY — P6/P12; concerns: SEC/ECON/GOV. Gateway may relay product traffic but must not relay caller-supplied chain authority.
//! RO:INTERACTS — `routes::app`, `routes::objects`, `routes::paid_storage`, `routes::product`.
//! RO:INVARIANTS — no hop-by-hop headers; no client-supplied roots/finality/validator/bridge authority; `idempotency-key` is retry identity only.
//! RO:METRICS — none; proxy routes own request metrics.
//! RO:CONFIG — none.
//! RO:SECURITY — filters authority-looking `x-ron-*` headers before upstream service hops.
//! RO:TEST — `quickchain_preflight_boundary`, `app_proxy`, `paid_storage_*_proxy`, `product_routes_proxy`.

use http::{
    header::{self},
    HeaderName,
};

#[must_use]
pub fn should_forward_passthrough_header(name: &HeaderName) -> bool {
    !is_hop_by_hop_or_host(name)
        && name != header::CONTENT_LENGTH
        && !is_quickchain_authority_header(name)
}

#[must_use]
pub fn should_forward_product_header(name: &HeaderName) -> bool {
    if !should_forward_passthrough_header(name) {
        return false;
    }

    let raw = name.as_str();

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || raw == "x-correlation-id"
        || raw == "x-request-id"
        || raw == "idempotency-key"
        || raw.starts_with("x-ron-")
}

#[must_use]
pub fn should_copy_response_header(name: &HeaderName) -> bool {
    !is_hop_by_hop_or_host(name)
        && name != header::CONTENT_LENGTH
        && !is_quickchain_authority_header(name)
}

#[must_use]
fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    matches!(
        name,
        &header::HOST
            | &header::CONNECTION
            | &header::PROXY_AUTHENTICATE
            | &header::PROXY_AUTHORIZATION
            | &header::TE
            | &header::TRAILER
            | &header::TRANSFER_ENCODING
            | &header::UPGRADE
    )
}

#[must_use]
fn is_quickchain_authority_header(name: &HeaderName) -> bool {
    let raw = name.as_str();

    matches!(
        raw,
        "x-ron-account-sequence"
            | "x-ron-operation-id"
            | "x-ron-state-root"
            | "x-ron-receipt-root"
            | "x-ron-accounting-root"
            | "x-ron-reward-root"
            | "x-ron-checkpoint-root"
            | "x-ron-checkpoint-hash"
            | "x-ron-data-availability-root"
            | "x-ron-validator-set"
            | "x-ron-validator-signature"
            | "x-ron-finality"
            | "x-ron-finalized"
            | "x-ron-anchored"
            | "x-ron-anchor"
            | "x-ron-bridge"
            | "x-ron-external-settlement"
            | "x-ron-staking"
            | "x-ron-liquidity"
            | "x-ron-balance"
            | "x-ron-entitlement"
            | "x-ron-unlock-authorized"
    ) || raw.starts_with("x-ron-quickchain-")
        || raw.starts_with("x-ron-validator-")
        || raw.starts_with("x-ron-bridge-")
        || raw.starts_with("x-ron-anchor-")
        || raw.starts_with("x-ron-checkpoint-")
        || raw.starts_with("x-ron-root-")
        || raw.starts_with("x-ron-ledger-")
}
