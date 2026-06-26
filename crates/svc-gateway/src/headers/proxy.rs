//! Proxy header filtering for public gateway → upstream service hops.
//!
//! RO:WHAT — Central allow/deny helpers for headers forwarded by gateway proxy routes.
//! RO:WHY — P6/P12; concerns: SEC/ECON/GOV. Gateway may relay product traffic but must not relay caller-supplied chain authority.
//! RO:INTERACTS — `routes::app`, `routes::objects`, `routes::paid_storage`, `routes::product`.
//! RO:INVARIANTS — no hop-by-hop headers; no client-supplied roots/finality/validator/committee/quorum/bridge/bond/slash/stake/liquidity/dispute authority; `idempotency-key` is retry identity only.
//! RO:METRICS — none; proxy routes own request metrics.
//! RO:CONFIG — none.
//! RO:SECURITY — filters authority-looking `QuickChain` / `x-ron-*` headers before upstream service hops.
//! RO:TEST — `quickchain_preflight_boundary`, phase2/3/4 boundary tests, `quickchain_phase4_bond_dispute_boundary`, proxy route tests.

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

    is_ledger_or_replay_identity_header(raw)
        || is_receipt_payment_or_balance_authority_header(raw)
        || is_root_checkpoint_or_proof_authority_header(raw)
        || is_committee_or_verifier_authority_header(raw)
        || is_validator_or_registry_authority_header(raw)
        || is_finality_settlement_or_governance_authority_header(raw)
        || is_bond_slash_stake_or_liquidity_authority_header(raw)
        || is_phase4_round2_dispute_authority_header(raw)
        || has_quickchain_authority_prefix(raw)
}

#[must_use]
fn is_ledger_or_replay_identity_header(raw: &str) -> bool {
    matches!(raw, "x-ron-account-sequence" | "x-ron-operation-id")
}

#[must_use]
fn is_receipt_payment_or_balance_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-receipt"
            | "x-ron-balance"
            | "x-ron-entitlement"
            | "x-ron-unlock-authorized"
            | "x-ron-paid"
            | "x-ron-unlock"
            | "x-ron-unlocked"
            | "x-ron-cache-unlock"
            | "x-ron-local-unlock"
            | "x-ron-client-receipt"
            | "x-ron-spend-authority"
            | "x-ron-capture-authority"
    )
}

#[must_use]
fn is_root_checkpoint_or_proof_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-root"
            | "x-ron-state-root"
            | "x-ron-receipt-root"
            | "x-ron-accounting-root"
            | "x-ron-reward-root"
            | "x-ron-hold-root"
            | "x-ron-epoch-root"
            | "x-ron-checkpoint"
            | "x-ron-checkpoint-root"
            | "x-ron-checkpoint-hash"
            | "x-ron-checkpoint-signature"
            | "x-ron-data-availability-root"
    )
}

#[must_use]
fn is_committee_or_verifier_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-replay-result"
            | "x-ron-replay-root"
            | "x-ron-verifier-result"
            | "x-ron-verifier-attestation"
            | "x-ron-committee-attestation"
            | "x-ron-committee-signature"
            | "x-ron-committee-member"
            | "x-ron-quorum"
            | "x-ron-quorum-certificate"
            | "x-ron-quorum-reached"
    )
}

#[must_use]
fn is_validator_or_registry_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-validator"
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
    )
}

#[must_use]
fn is_finality_settlement_or_governance_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-finalized"
            | "x-ron-finality"
            | "x-ron-epoch-included"
            | "x-ron-anchored"
            | "x-ron-anchor"
            | "x-ron-settlement"
            | "x-ron-bridge"
            | "x-ron-bridge-settled"
            | "x-ron-external-settlement"
            | "x-ron-governance-parameter-update"
            | "x-ron-governance-approval"
            | "x-ron-validator-lifecycle-decision"
            | "x-ron-lifecycle-decision"
    )
}

#[must_use]
fn is_bond_slash_stake_or_liquidity_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-bond"
            | "x-ron-bond-account"
            | "x-ron-bond-intent"
            | "x-ron-bond-lifecycle"
            | "x-ron-bond-lifecycle-decision"
            | "x-ron-bond-authority"
            | "x-ron-validator-bond"
            | "x-ron-bonded-stake"
            | "x-ron-slash"
            | "x-ron-slashing"
            | "x-ron-slash-evidence"
            | "x-ron-slash-decision"
            | "x-ron-stake"
            | "x-ron-staking"
            | "x-ron-liquidity"
    )
}

#[must_use]
fn is_phase4_round2_dispute_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        "x-ron-bond-dispute"
            | "x-ron-bond-dispute-state"
            | "x-ron-dispute"
            | "x-ron-dispute-window"
            | "x-ron-challenge"
            | "x-ron-challenge-window"
            | "x-ron-appeal"
            | "x-ron-appeal-window"
            | "x-ron-freeze"
            | "x-ron-frozen-bond"
            | "x-ron-disputed-bond"
            | "x-ron-irreversible-slash"
            | "x-ron-slash-appeal"
            | "x-ron-slash-challenge"
            | "x-ron-slash-simulation"
    )
}

#[must_use]
fn has_quickchain_authority_prefix(raw: &str) -> bool {
    raw.starts_with("x-quickchain-")
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
        || raw.starts_with("x-ron-bond-")
        || raw.starts_with("x-ron-slash-")
        || raw.starts_with("x-ron-stake-")
        || raw.starts_with("x-ron-dispute-")
        || raw.starts_with("x-ron-disputed-")
        || raw.starts_with("x-ron-challenge-")
        || raw.starts_with("x-ron-appeal-")
        || raw.starts_with("x-ron-freeze-")
        || raw.starts_with("x-ron-frozen-")
        || raw.starts_with("x-ron-irreversible-slash")
        || raw.starts_with("x-ron-slash-simulation")
}
