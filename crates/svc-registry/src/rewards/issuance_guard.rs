//! RO:WHAT — Non-authoritative payout authorization preview checks.
//! RO:WHY — Reward evidence must not smuggle payout addresses or allow a node to self-issue ROC.
//! RO:INTERACTS — RewardBindingRegistry resolution, future ron-policy/svc-rewarder/svc-wallet gates.
//! RO:INVARIANTS — preview only; no wallet mutation, no ledger mutation, no payout execution.
//! RO:SECURITY — rejects payout overrides and self-issued payouts by default.

use ron_proto::{RewardRecipientResolutionStateV1, RewardRecipientResolutionV1};
use thiserror::Error;

/// Self-issuance handling posture for local/dev tests.
///
/// Production/default logic rejects self-issued payouts. The allow mode is only
/// active when the crate is built with `test-self-issuance-fixtures`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SelfIssuanceMode {
    /// Reject issuer == beneficiary service node.
    #[default]
    RejectByDefault,
    /// Allow only when compiled with the test fixture feature.
    AllowTestFixtureOnly,
}

/// Input to the non-authoritative reward payout preview guard.
#[derive(Debug, Clone, Copy)]
pub struct RewardPayoutAuthorizationPreview<'a> {
    /// Node proposing/signing/issuing the payout preview.
    pub issuer_service_node_id: &'a str,
    /// Node whose service evidence would be rewarded.
    pub beneficiary_service_node_id: &'a str,
    /// Registry-derived recipient resolution for the beneficiary node.
    pub recipient_resolution: &'a RewardRecipientResolutionV1,
    /// Any payout address carried by evidence. Must be absent.
    pub evidence_payout_override: Option<&'a str>,
    /// Local self-issuance policy mode.
    pub self_issuance_mode: SelfIssuanceMode,
}

/// Rejection reasons for the preview guard.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RewardPayoutAuthorizationError {
    /// Service evidence tried to carry its own payout recipient.
    #[error("service evidence must not carry payout recipient override")]
    EvidencePayoutOverrideForbidden,

    /// Registry did not resolve a usable recipient.
    #[error("reward recipient is not resolved")]
    RecipientNotResolved,

    /// Resolution was for a different beneficiary service node.
    #[error("reward recipient resolution beneficiary mismatch")]
    BeneficiaryMismatch,

    /// A node attempted to issue/approve payout to its own bound recipient.
    #[error("self-issued payout to bound reward recipient is rejected")]
    SelfIssuedPayoutRejected,
}

/// Preview payout authorization using registry-derived recipient resolution.
///
/// This is intentionally not a wallet/ledger gate. It is an early shared rule
/// that later rewarder, policy, wallet, and ledger code should mirror and harden.
pub fn preview_reward_payout_authorization(
    input: RewardPayoutAuthorizationPreview<'_>,
) -> Result<(), RewardPayoutAuthorizationError> {
    if input.evidence_payout_override.is_some() {
        return Err(RewardPayoutAuthorizationError::EvidencePayoutOverrideForbidden);
    }

    if input.recipient_resolution.state != RewardRecipientResolutionStateV1::Resolved {
        return Err(RewardPayoutAuthorizationError::RecipientNotResolved);
    }

    if input.recipient_resolution.service_node_id != input.beneficiary_service_node_id {
        return Err(RewardPayoutAuthorizationError::BeneficiaryMismatch);
    }

    if input.issuer_service_node_id == input.beneficiary_service_node_id
        && !self_issuance_test_fixture_enabled(input.self_issuance_mode)
    {
        return Err(RewardPayoutAuthorizationError::SelfIssuedPayoutRejected);
    }

    Ok(())
}

fn self_issuance_test_fixture_enabled(mode: SelfIssuanceMode) -> bool {
    matches!(mode, SelfIssuanceMode::AllowTestFixtureOnly)
        && cfg!(feature = "test-self-issuance-fixtures")
}
