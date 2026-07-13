//! RO:WHAT — Shared exact-b3 moderation gate for legacy HTTP reads.
//! RO:WHY — GET and HEAD must use ron-policy precedence before storage access.
//! RO:INTERACTS — get_object, head_object, ron-policy moderation decisions.
//! RO:INVARIANTS — no duplicate precedence; no storage lookup before permission.
//! RO:METRICS — callers may count only returned refusal reasons.
//! RO:CONFIG — immutable policy snapshot supplied through the router.
//! RO:SECURITY — no deletion, provider mutation, reward, wallet, or ledger authority.
//! RO:TEST — legacy_http_moderation, moderation_observability.

use ron_policy::{B3Id, ModerationPolicy, ModerationReasonCode};

/// Legacy read refusal without erasing the core-owned policy reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LegacyModerationError {
    InvalidIdentity,
    Refused(ModerationReasonCode),
}

/// Require the immutable policy snapshot to permit serving this exact b3 ID.
///
/// Callers must run canonical-CID validation first. A parse failure here
/// therefore represents an internal identity inconsistency.
pub(super) fn require_serve(
    policy: &ModerationPolicy,
    cid: &str,
) -> Result<(), LegacyModerationError> {
    let object = cid
        .parse::<B3Id>()
        .map_err(|_| LegacyModerationError::InvalidIdentity)?;

    let decision = policy.evaluate(&object);

    if decision.permits_serve() {
        Ok(())
    } else {
        Err(LegacyModerationError::Refused(decision.reason))
    }
}
