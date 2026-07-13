//! RO:WHAT — Policy-gated review of provider counts for repair-candidate creation.
//! RO:WHY — Phase 12 must identify under-replication without repairing refused content.
//! RO:INTERACTS — ron-policy moderation, provider::Store, types::{B3Cid, CrabNodeId}.
//! RO:INVARIANTS — canonical policy first; bounded target; selectable providers only.
//! RO:SECURITY — denied, tombstoned, blocked, and quarantined content cannot repair.
//! RO:TEST — tests/provider_repair_candidate.rs.

use crate::{
    provider::Store,
    types::{B3Cid, CrabNodeId},
};
use ron_policy::{B3Id as ModerationB3Id, ModerationPolicy, ModerationReasonCode};
use std::fmt;

/// Upper bound for one local repair review.
///
/// This matches the bounded DHT lookup posture and prevents callers from
/// requesting an unbounded provider target.
pub const MAX_REPLICATION_TARGET: usize = 32;

/// A truthful, policy-gated review of whether an object needs provider repair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepairReview {
    /// Canonical moderation policy forbids repair consideration.
    ///
    /// No repair candidate was created. The reason comes directly from
    /// `ron-policy`; `svc-dht` does not maintain a duplicate policy taxonomy.
    Suppressed { reason: ModerationReasonCode },

    /// At least the requested number of selectable providers exists.
    ///
    /// This does not claim that every provider recently served valid bytes.
    TargetSatisfied { target_provider_count: usize },

    /// The object has fewer selectable providers than requested.
    ///
    /// This is a candidate for later scheduling. It is not evidence that
    /// another provider accepted, stored, verified, or advertised the object.
    Candidate(RepairCandidate),
}

/// Read-only description of one policy-eligible under-replicated object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairCandidate {
    /// Canonical object identifier under review.
    pub cid: B3Cid,

    /// Requested selectable-provider target.
    pub target_provider_count: usize,

    /// Current count of live and selectable providers.
    pub current_provider_count: usize,

    /// Additional providers required to meet the requested target.
    pub missing_provider_count: usize,

    /// Existing privacy-safe providers that may later supply verified bytes.
    ///
    /// An empty list means no selectable network source is currently known.
    /// A later recovery process may require an authoritative creator source.
    pub source_providers: Vec<CrabNodeId>,
}

/// Invalid repair-review input or an internal canonical-ID contract mismatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairReviewError {
    /// Replication targets must be nonzero.
    ZeroTarget,

    /// Replication targets must remain within the bounded DHT posture.
    TargetTooLarge { requested: usize, maximum: usize },

    /// `svc-dht` and `ron-policy` disagreed on canonical B3 syntax.
    ///
    /// Both currently require `b3:<64 lowercase hex>`, so reaching this state
    /// indicates an internal contract regression and must fail closed.
    ModerationIdentifierMismatch,
}

impl RepairReviewError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZeroTarget => "repair_target_must_be_nonzero",
            Self::TargetTooLarge { .. } => "repair_target_exceeds_maximum",
            Self::ModerationIdentifierMismatch => "repair_moderation_identifier_mismatch",
        }
    }
}

impl fmt::Display for RepairReviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTarget | Self::ModerationIdentifierMismatch => f.write_str(self.as_str()),
            Self::TargetTooLarge { requested, maximum } => {
                write!(f, "{}: requested={requested}, maximum={maximum}", self.as_str())
            }
        }
    }
}

impl std::error::Error for RepairReviewError {}

/// Review one object against canonical moderation policy and a bounded
/// selectable-provider target.
///
/// Evaluation order is intentionally fail-closed:
///
/// 1. validate the requested target
/// 2. convert the already-canonical DHT CID into the canonical policy ID
/// 3. evaluate `ron-policy` moderation
/// 4. suppress every non-serving decision
/// 5. only then count selectable providers and consider repair
pub fn review_repair_need(
    store: &Store,
    cid: &B3Cid,
    target_provider_count: usize,
    moderation_policy: &ModerationPolicy,
) -> Result<RepairReview, RepairReviewError> {
    validate_target(target_provider_count)?;

    let moderation_id = cid
        .as_str()
        .parse::<ModerationB3Id>()
        .map_err(|_| RepairReviewError::ModerationIdentifierMismatch)?;

    let moderation_decision = moderation_policy.evaluate(&moderation_id);

    if !moderation_decision.permits_serve() {
        return Ok(RepairReview::Suppressed { reason: moderation_decision.reason });
    }

    let source_providers = store.select_candidates(cid.as_str(), target_provider_count);

    if source_providers.len() == target_provider_count {
        return Ok(RepairReview::TargetSatisfied { target_provider_count });
    }

    let current_provider_count = source_providers.len();
    let missing_provider_count = target_provider_count - current_provider_count;

    Ok(RepairReview::Candidate(RepairCandidate {
        cid: cid.clone(),
        target_provider_count,
        current_provider_count,
        missing_provider_count,
        source_providers,
    }))
}

fn validate_target(target: usize) -> Result<(), RepairReviewError> {
    if target == 0 {
        return Err(RepairReviewError::ZeroTarget);
    }

    if target > MAX_REPLICATION_TARGET {
        return Err(RepairReviewError::TargetTooLarge {
            requested: target,
            maximum: MAX_REPLICATION_TARGET,
        });
    }

    Ok(())
}
