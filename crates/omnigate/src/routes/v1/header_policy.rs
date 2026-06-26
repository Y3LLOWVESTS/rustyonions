//! RO:WHAT — Shared v1 product-route header forwarding policy.
//! RO:WHY — P6/P12; Concerns: SEC/ECON/GOV. Product routes may forward context, not caller-supplied QuickChain authority.
//! RO:INTERACTS — assets/chat/content_view/paid/profile/site_visit/sites/text_assets route proxy helpers.
//! RO:INVARIANTS — no hop-by-hop authority; no caller roots/finality/checkpoints/validators/committees/quorums/bridges/bonds/slashes/stakes/liquidity/passport-registry authority; wallet receipt headers are product context only.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — strips QuickChain-like authority headers before downstream forwarding.
//! RO:TEST — quickchain_preflight_transport_authority, quickchain_phase2_replay_boundary, quickchain_phase2_committee_boundary, quickchain_phase3_validator_boundary, quickchain_phase3_validator_lifecycle_boundary, quickchain_phase4_bond_boundary, quickchain_phase4_bond_dispute_boundary.

use axum::http::HeaderName;

/// Returns true for `x-ron-*` headers that are allowed to travel as product
/// context through omnigate route proxies.
///
/// This deliberately allows current WEB3/ROC product context such as:
///
/// - `x-ron-wallet-account`
/// - `x-ron-passport`
/// - `x-ron-wallet-txid`
/// - `x-ron-wallet-receipt-hash`
/// - `x-ron-paid-op`
/// - `x-ron-paid-asset`
/// - asset metadata headers
///
/// It deliberately rejects caller-supplied QuickChain/root/finality/validator,
/// passport-registry, capability, committee, quorum, replay, proof, bridge,
/// bond, slash, stake/liquidity, and settlement authority shapes. Omnigate may
/// coordinate paid product flows, but it must not accept or forward client
/// claims that look like ledger/root/checkpoint/finality/validator/bond truth.
pub(crate) fn is_allowed_ron_context_header(name: &HeaderName) -> bool {
    let raw = name.as_str();

    raw.starts_with("x-ron-") && !is_quickchain_authority_header(raw)
}

fn is_quickchain_authority_header(raw: &str) -> bool {
    matches!(
        raw,
        // Durable ledger/replay identity must be backend/ledger assigned.
        "x-ron-operation-id"
            | "x-ron-account-sequence"
            // Direct fake receipt/payment/unlock claims.
            | "x-ron-receipt"
            | "x-ron-receipt-id"
            | "x-ron-receipt-hash"
            | "x-ron-paid"
            | "x-ron-unlocked"
            | "x-ron-unlock"
            | "x-ron-entitlement"
            // Fake balance/ledger truth.
            | "x-ron-balance"
            | "x-ron-ledger"
            // Root/checkpoint/proof authority.
            | "x-ron-root"
            | "x-ron-state-root"
            | "x-ron-receipt-root"
            | "x-ron-hold-root"
            | "x-ron-epoch-root"
            | "x-ron-checkpoint"
            | "x-ron-checkpoint-root"
            | "x-ron-checkpoint-hash"
            | "x-ron-checkpoint-signature"
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
            // Validator / passport-gated registry authority.
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
            // Finality / settlement / external anchor authority.
            | "x-ron-finalized"
            | "x-ron-finality"
            | "x-ron-epoch-included"
            | "x-ron-anchored"
            | "x-ron-anchor"
            | "x-ron-settlement"
            | "x-ron-external-settlement"
            | "x-ron-governance-parameter-update"
            | "x-ron-governance-approval"
            | "x-ron-validator-lifecycle-decision"
            | "x-ron-lifecycle-decision"
            // Phase 4 Round 1 bond/slash/stake/liquidity authority.
            | "x-ron-bond"
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
            // Phase 4 Round 2 dispute/challenge/appeal/freeze authority.
            | "x-ron-bond-dispute"
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
            // Validator/bridge/spend authority.
            | "x-ron-bridge"
            | "x-ron-bridge-settled"
            | "x-ron-spend-authority"
            | "x-ron-capture-authority"
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
        || raw.starts_with("x-ron-proof-")
        || raw.starts_with("x-ron-replay-")
        || raw.starts_with("x-ron-verifier-")
        || raw.starts_with("x-ron-committee-")
        || raw.starts_with("x-ron-quorum-")
        || raw.starts_with("x-ron-ledger-")
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
