//! Proxy header filtering for public gateway → upstream service hops.
//!
//! RO:WHAT — Central allow/deny helpers for headers forwarded by gateway proxy routes.
//! RO:WHY — P6/P12; concerns: SEC/ECON/GOV. Gateway may relay product traffic but must not relay caller-supplied chain authority.
//! RO:INTERACTS — `routes::app`, `routes::objects`, `routes::paid_storage`, `routes::product`.
//! RO:INVARIANTS — no hop-by-hop headers; no client-supplied roots/finality/validator/committee/quorum/bridge/passport-registry authority; `idempotency-key` is retry identity only.
//! RO:METRICS — none; proxy routes own request metrics.
//! RO:CONFIG — none.
//! RO:SECURITY — filters authority-looking `QuickChain` / `x-ron-*` headers before upstream service hops.
//! RO:TEST — `quickchain_preflight_boundary`, `quickchain_phase2_replay_boundary`, `quickchain_phase2_committee_boundary`, `quickchain_phase3_validator_boundary`, proxy route tests.

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
        // Ledger / sequencing / operation authority.
        "x-ron-account-sequence"
            | "x-ron-operation-id"
            // Root / checkpoint / DA / proof authority.
            | "x-ron-state-root"
            | "x-ron-receipt-root"
            | "x-ron-accounting-root"
            | "x-ron-reward-root"
            | "x-ron-checkpoint-root"
            | "x-ron-checkpoint-hash"
            | "x-ron-data-availability-root"
            // Replay / verifier / committee / quorum authority.
            | "x-ron-replay-result"
            | "x-ron-replay-root"
            | "x-ron-verifier-result"
            | "x-ron-verifier-attestation"
            | "x-ron-committee-attestation"
            | "x-ron-committee-signature"
            | "x-ron-committee-member"
            | "x-ron-quorum"
            | "x-ron-quorum-certificate"
            | "x-ron-quorum-reached"
            // Validator / passport-gated registry / finality / settlement authority.
            | "x-ron-validator"
            | "x-ron-validator-set"
            | "x-ron-validator-signature"
            | "x-ron-validator-passport"
            | "x-ron-validator-capability"
            | "x-ron-validator-registry-entry"
            | "x-ron-validator-membership-proof"
            | "x-ron-validator-authorization"
            | "x-ron-validator-authz-result"
            | "x-ron-passport-validator"
            | "x-ron-passport-validator-admission"
            | "x-ron-passport-validator-capability"
            | "x-ron-registry-validator"
            | "x-ron-registry-validator-set"
            | "x-ron-capability-validator"
            | "x-ron-capability-validator-scope"
            | "x-ron-attestation-identity"
            | "x-ron-finality"
            | "x-ron-finalized"
            | "x-ron-anchored"
            | "x-ron-anchor"
            | "x-ron-bridge"
            | "x-ron-bridge-settled"
            | "x-ron-external-settlement"
            | "x-ron-governance-parameter-update"
            | "x-ron-governance-approval"
            | "x-ron-validator-lifecycle-decision"
            | "x-ron-lifecycle-decision"
            | "x-ron-staking"
            | "x-ron-liquidity"
            // Balance / entitlement / paid-unlock authority.
            | "x-ron-balance"
            | "x-ron-entitlement"
            | "x-ron-unlock-authorized"
            | "x-ron-paid"
            | "x-ron-unlock"
            | "x-ron-unlocked"
            | "x-ron-cache-unlock"
            | "x-ron-local-unlock"
            | "x-ron-client-receipt"
    ) || raw.starts_with("x-quickchain-")
        || raw.starts_with("x-qc-")
        || raw.starts_with("x-ron-quickchain-")
        || raw.starts_with("x-ron-validator-")
        || raw.starts_with("x-ron-passport-validator")
        || raw.starts_with("x-ron-registry-validator")
        || raw.starts_with("x-ron-capability-validator")
        || raw.starts_with("x-ron-attestation-identity")
        || raw.starts_with("x-ron-validator-authz")
        || raw.starts_with("x-ron-bridge-")
        || raw.starts_with("x-ron-anchor-")
        || raw.starts_with("x-ron-checkpoint-")
        || raw.starts_with("x-ron-root-")
        || raw.starts_with("x-ron-ledger-")
        || raw.starts_with("x-ron-proof-")
        || raw.starts_with("x-ron-replay-")
        || raw.starts_with("x-ron-verifier-")
        || raw.starts_with("x-ron-committee-")
        || raw.starts_with("x-ron-quorum-")
        || raw.starts_with("x-ron-settlement-")
        || raw.starts_with("x-ron-governance-")
        || raw.starts_with("x-ron-lifecycle-")
}
