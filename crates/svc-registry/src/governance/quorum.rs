//! RO:WHAT — Registry-backed fresh-node caps and weighted Service Node quorum review.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 18 requires permissionless participation without allowing
//! newly created identities to dominate an existing Service Node quorum.
//!
//! RO:INTERACTS — `ron-proto` Phase 15 quorum validation and `svc-registry` canonical
//! Service Node eligibility descriptors.
//!
//! RO:INVARIANTS — only lifecycle-eligible nodes count; registry identity fields must match;
//! fresh aggregate weight remains a strict minority; input registry order is irrelevant.
//!
//! RO:CONFIG — every age, weight, and fresh-share value is caller supplied and validated;
//! this module defines no production economics or deployment defaults.
//!
//! RO:SECURITY — structural and policy review only; no signature cryptography, reward,
//! wallet, ledger, balance, mint, burn, settlement, or finality mutation.
//!
//! RO:TEST — `tests/internal_roc_beta_phase18_quorum_weighting.rs`.

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashSet};

use ron_proto::{
    ServiceNodeEligibilityStateV1, ServiceNodeEligibilityValidationError,
    ServiceNodeIdentityDescriptorV1, ServiceNodeQuorumV1, ServiceNodeQuorumValidationError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::eligibility::ServiceNodeEligibilityRegistry;

/// Current fresh-node quorum-weighting policy version.
pub const SERVICE_NODE_QUORUM_WEIGHT_POLICY_VERSION: u16 = 1;

const BPS_DENOMINATOR: u16 = 10_000;
const STRICT_MINORITY_BPS: u16 = BPS_DENOMINATOR / 2;

/// Reviewed configuration for deterministic Service Node quorum weighting.
///
/// These values are protocol-policy inputs. They are not ROC reward rates and
/// this type does not define production defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeQuorumWeightPolicyV1 {
    /// Policy schema version.
    pub version: u16,

    /// Registration age required before full mature weight is available.
    pub minimum_mature_registration_age_epochs: u64,

    /// Continuous eligible-state age required before full mature weight.
    pub minimum_mature_eligibility_age_epochs: u64,

    /// Weight assigned to one mature eligible Service Node.
    pub mature_weight_units: u64,

    /// Raw weight assigned to one fresh eligible Service Node before the
    /// aggregate fresh-population cap is applied.
    pub fresh_weight_units: u64,

    /// Maximum aggregate fresh weight as a share of total counted weight.
    ///
    /// This must remain below 5,000 basis points so fresh identities can never
    /// form a weighted majority by themselves.
    pub maximum_fresh_weight_bps: u16,
}

impl ServiceNodeQuorumWeightPolicyV1 {
    /// Validate the reviewed quorum-weight policy.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeQuorumWeightError::InvalidPolicy`] for unsupported
    /// versions, zero maturity ages or weights, fresh weight above mature
    /// weight, or a fresh aggregate cap that is not a strict minority.
    pub const fn validate(self) -> Result<(), ServiceNodeQuorumWeightError> {
        if self.version != SERVICE_NODE_QUORUM_WEIGHT_POLICY_VERSION {
            return invalid_policy("version", "unsupported policy version");
        }

        if self.minimum_mature_registration_age_epochs == 0 {
            return invalid_policy(
                "minimum_mature_registration_age_epochs",
                "must be greater than zero",
            );
        }

        if self.minimum_mature_eligibility_age_epochs == 0 {
            return invalid_policy(
                "minimum_mature_eligibility_age_epochs",
                "must be greater than zero",
            );
        }

        if self.mature_weight_units == 0 {
            return invalid_policy("mature_weight_units", "must be greater than zero");
        }

        if self.fresh_weight_units == 0 {
            return invalid_policy("fresh_weight_units", "must be greater than zero");
        }

        if self.fresh_weight_units > self.mature_weight_units {
            return invalid_policy("fresh_weight_units", "must not exceed mature_weight_units");
        }

        if self.maximum_fresh_weight_bps >= STRICT_MINORITY_BPS {
            return invalid_policy(
                "maximum_fresh_weight_bps",
                "must remain below 5000 basis points",
            );
        }

        Ok(())
    }

    fn classify(
        self,
        descriptor: &ServiceNodeIdentityDescriptorV1,
        current_epoch: u64,
    ) -> Result<ServiceNodeQuorumWeightClassV1, ServiceNodeQuorumWeightError> {
        let registration_age = current_epoch
            .checked_sub(descriptor.registered_at_epoch)
            .ok_or_else(|| ServiceNodeQuorumWeightError::EpochBeforeRegistration {
                service_node_id: descriptor.service_node_id.clone(),
                current_epoch,
                registered_at_epoch: descriptor.registered_at_epoch,
            })?;

        let eligibility_age = current_epoch
            .checked_sub(descriptor.state_effective_epoch)
            .ok_or_else(
                || ServiceNodeQuorumWeightError::EpochBeforeStateActivation {
                    service_node_id: descriptor.service_node_id.clone(),
                    current_epoch,
                    state_effective_epoch: descriptor.state_effective_epoch,
                },
            )?;

        if registration_age < self.minimum_mature_registration_age_epochs
            || eligibility_age < self.minimum_mature_eligibility_age_epochs
        {
            Ok(ServiceNodeQuorumWeightClassV1::FreshEligible)
        } else {
            Ok(ServiceNodeQuorumWeightClassV1::MatureEligible)
        }
    }
}

/// Weight class assigned from objective registration and eligible-state age.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceNodeQuorumWeightClassV1 {
    /// Recently registered or recently promoted Service Node.
    FreshEligible,

    /// Service Node that has satisfied both maturity-age requirements.
    MatureEligible,
}

/// Deterministic member-level weighting record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeQuorumWeightedMemberV1 {
    /// Canonical Service Node identity.
    pub service_node_id: String,

    /// Objective age-based weight class.
    pub weight_class: ServiceNodeQuorumWeightClassV1,

    /// Raw configured weight before the aggregate fresh cap.
    pub raw_weight_units: u64,

    /// Whether this Service Node supplied a structurally accepted signature.
    pub signed: bool,
}

/// Successful registry-backed weighted quorum review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeQuorumWeightReviewV1 {
    /// Registry epoch used for lifecycle-age calculations.
    pub current_epoch: u64,

    /// Number of registry-eligible Service Nodes.
    pub eligible_member_count: u16,

    /// Number classified as mature.
    pub mature_member_count: u16,

    /// Number classified as fresh.
    pub fresh_member_count: u16,

    /// Raw aggregate mature weight.
    pub raw_mature_weight: u64,

    /// Raw aggregate fresh weight before capping.
    pub raw_fresh_weight: u64,

    /// Maximum fresh weight permitted by the strict-minority cap.
    pub maximum_counted_fresh_weight: u64,

    /// Fresh weight actually included in total quorum weight.
    pub counted_fresh_weight: u64,

    /// Total mature plus capped-fresh quorum weight.
    pub total_counted_weight: u64,

    /// Weighted signature requirement derived from the existing quorum basis
    /// points.
    pub required_signed_weight: u64,

    /// Mature signed weight plus capped fresh signed weight.
    pub actual_signed_weight: u64,

    /// Structurally accepted signature count.
    pub signature_count: u16,

    /// Canonically ordered member review records.
    pub members: Vec<ServiceNodeQuorumWeightedMemberV1>,
}

impl ServiceNodeQuorumWeightReviewV1 {
    /// A successful review has satisfied the weighted threshold.
    #[must_use]
    pub const fn reaches_weighted_quorum(&self) -> bool {
        self.actual_signed_weight >= self.required_signed_weight
    }

    /// Quorum review grants no economic or ledger mutation authority.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

/// Registry-backed weighted quorum rejection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeQuorumWeightError {
    /// The caller-supplied weight policy was malformed or unsafe.
    #[error("invalid Service Node quorum-weight policy field {field}: {reason}")]
    InvalidPolicy {
        /// Invalid policy field.
        field: &'static str,

        /// Stable validation reason.
        reason: &'static str,
    },

    /// The existing Phase 15 structural quorum was invalid.
    #[error(transparent)]
    StructuralQuorum(#[from] ServiceNodeQuorumValidationError),

    /// A canonical registry descriptor was invalid.
    #[error(transparent)]
    InvalidDescriptor(#[from] ServiceNodeEligibilityValidationError),

    /// The quorum claimed a Service Node not held by registry custody.
    #[error("Service Node quorum member is missing from registry: {service_node_id}")]
    DescriptorNotFound {
        /// Missing Service Node identity.
        service_node_id: String,
    },

    /// The quorum claimed a lifecycle state that cannot count toward quorum.
    #[error(
        "Service Node lifecycle state cannot count toward quorum: {service_node_id} state={state:?}"
    )]
    LifecycleIneligible {
        /// Rejected Service Node identity.
        service_node_id: String,

        /// Canonical registry lifecycle state.
        state: ServiceNodeEligibilityStateV1,
    },

    /// A quorum eligibility identity field disagreed with registry truth.
    #[error("Service Node quorum eligibility field mismatch for {service_node_id}: {field}")]
    EligibilityFieldMismatch {
        /// Service Node identity.
        service_node_id: String,

        /// Mismatched identity field.
        field: &'static str,
    },

    /// The quorum omitted or added lifecycle-eligible registry identities.
    #[error("Service Node quorum eligibility set mismatch: expected={expected}, actual={actual}")]
    EligibilitySetMismatch {
        /// Registry-owned eligible count.
        expected: usize,

        /// Quorum-claimed eligible count.
        actual: usize,
    },

    /// Review epoch preceded registration.
    #[error(
        "Service Node quorum epoch {current_epoch} precedes registration epoch {registered_at_epoch}: {service_node_id}"
    )]
    EpochBeforeRegistration {
        /// Service Node identity.
        service_node_id: String,

        /// Review epoch.
        current_epoch: u64,

        /// Registration epoch.
        registered_at_epoch: u64,
    },

    /// Review epoch preceded lifecycle activation.
    #[error(
        "Service Node quorum epoch {current_epoch} precedes state-effective epoch {state_effective_epoch}: {service_node_id}"
    )]
    EpochBeforeStateActivation {
        /// Service Node identity.
        service_node_id: String,

        /// Review epoch.
        current_epoch: u64,

        /// Lifecycle activation epoch.
        state_effective_epoch: u64,
    },

    /// No mature weight existed from which a safe fresh minority could be
    /// derived.
    #[error("weighted Service Node quorum has no mature eligible weight")]
    NoMatureEligibleWeight,

    /// Structural signature count passed, but weighted authority did not.
    #[error("insufficient weighted Service Node quorum: required={required}, actual={actual}")]
    InsufficientWeightedQuorum {
        /// Required signed weight.
        required: u64,

        /// Actual counted signed weight.
        actual: u64,
    },

    /// Checked weight arithmetic exceeded representable bounds.
    #[error("Service Node quorum weight arithmetic overflow")]
    ArithmeticOverflow,
}

/// Extend Phase 15 structural validation with registry lifecycle and Sybil caps.
///
/// This function first invokes [`ServiceNodeQuorumV1::validate`]. It then
/// requires the quorum eligibility set to exactly equal the registry's current
/// lifecycle-eligible set, verifies every identity field against registry
/// truth, applies deterministic age-based weights, caps aggregate fresh weight,
/// and evaluates the same quorum basis points against counted weight.
///
/// It does not perform cryptographic signature verification. Existing wallet
/// and replay paths remain responsible for verifying signature bytes and all
/// economic transition commitments.
///
/// # Errors
///
/// Returns [`ServiceNodeQuorumWeightError`] when structural validation fails,
/// registry identity or lifecycle truth disagrees with the quorum, the policy
/// is unsafe, arithmetic overflows, no mature weight exists, or signed weight
/// does not reach the deterministic threshold.
pub fn review_service_node_quorum_weight(
    registry: &ServiceNodeEligibilityRegistry,
    quorum: &ServiceNodeQuorumV1,
    current_epoch: u64,
    policy: ServiceNodeQuorumWeightPolicyV1,
) -> Result<ServiceNodeQuorumWeightReviewV1, ServiceNodeQuorumWeightError> {
    policy.validate()?;
    quorum.validate()?;

    let expected_eligible_ids = registry
        .descriptors()
        .map(|descriptor| {
            descriptor.validate()?;
            Ok(descriptor)
        })
        .collect::<Result<Vec<_>, ServiceNodeQuorumWeightError>>()?
        .into_iter()
        .filter(|descriptor| descriptor.state.counts_toward_quorum())
        .map(|descriptor| descriptor.service_node_id.clone())
        .collect::<BTreeSet<_>>();

    let signed_ids = quorum
        .signatures
        .iter()
        .map(|signature| signature.service_node_id.as_str())
        .collect::<HashSet<_>>();

    let mut claimed_eligible_ids = BTreeSet::new();
    let mut members = Vec::with_capacity(quorum.eligibilities.len());

    let mut mature_member_count = 0_u16;
    let mut fresh_member_count = 0_u16;
    let mut raw_mature_weight = 0_u64;
    let mut raw_fresh_weight = 0_u64;
    let mut signed_mature_weight = 0_u64;
    let mut signed_fresh_weight = 0_u64;

    for eligibility in &quorum.eligibilities {
        let descriptor = registry
            .descriptor(&eligibility.service_node_id)
            .ok_or_else(|| ServiceNodeQuorumWeightError::DescriptorNotFound {
                service_node_id: eligibility.service_node_id.clone(),
            })?;

        if !descriptor.state.counts_toward_quorum() {
            return Err(ServiceNodeQuorumWeightError::LifecycleIneligible {
                service_node_id: descriptor.service_node_id.clone(),
                state: descriptor.state,
            });
        }

        require_equal(
            &descriptor.service_node_id,
            &eligibility.service_node_id,
            &descriptor.service_node_id,
            "service_node_id",
        )?;
        require_equal(
            &descriptor.registry_entry_id,
            &eligibility.registry_entry_id,
            &descriptor.service_node_id,
            "registry_entry_id",
        )?;
        require_equal(
            &descriptor.reward_binding_id,
            &eligibility.reward_binding_id,
            &descriptor.service_node_id,
            "reward_binding_id",
        )?;
        require_equal(
            &descriptor.key_id,
            &eligibility.key_id,
            &descriptor.service_node_id,
            "key_id",
        )?;

        let weight_class = policy.classify(descriptor, current_epoch)?;
        let signed = signed_ids.contains(descriptor.service_node_id.as_str());

        let raw_weight_units = match weight_class {
            ServiceNodeQuorumWeightClassV1::MatureEligible => {
                mature_member_count = mature_member_count
                    .checked_add(1)
                    .ok_or(ServiceNodeQuorumWeightError::ArithmeticOverflow)?;

                raw_mature_weight = checked_add(raw_mature_weight, policy.mature_weight_units)?;

                if signed {
                    signed_mature_weight =
                        checked_add(signed_mature_weight, policy.mature_weight_units)?;
                }

                policy.mature_weight_units
            }
            ServiceNodeQuorumWeightClassV1::FreshEligible => {
                fresh_member_count = fresh_member_count
                    .checked_add(1)
                    .ok_or(ServiceNodeQuorumWeightError::ArithmeticOverflow)?;

                raw_fresh_weight = checked_add(raw_fresh_weight, policy.fresh_weight_units)?;

                if signed {
                    signed_fresh_weight =
                        checked_add(signed_fresh_weight, policy.fresh_weight_units)?;
                }

                policy.fresh_weight_units
            }
        };

        claimed_eligible_ids.insert(descriptor.service_node_id.clone());

        members.push(ServiceNodeQuorumWeightedMemberV1 {
            service_node_id: descriptor.service_node_id.clone(),
            weight_class,
            raw_weight_units,
            signed,
        });
    }

    if claimed_eligible_ids != expected_eligible_ids {
        return Err(ServiceNodeQuorumWeightError::EligibilitySetMismatch {
            expected: expected_eligible_ids.len(),
            actual: claimed_eligible_ids.len(),
        });
    }

    if raw_mature_weight == 0 {
        return Err(ServiceNodeQuorumWeightError::NoMatureEligibleWeight);
    }

    let maximum_counted_fresh_weight =
        maximum_fresh_weight(raw_mature_weight, policy.maximum_fresh_weight_bps)?;

    let counted_fresh_weight = raw_fresh_weight.min(maximum_counted_fresh_weight);

    let total_counted_weight = checked_add(raw_mature_weight, counted_fresh_weight)?;

    let required_signed_weight = apply_bps_ceil(total_counted_weight, quorum.threshold.quorum_bps)?;

    let counted_signed_fresh_weight = signed_fresh_weight.min(maximum_counted_fresh_weight);

    let actual_signed_weight = checked_add(signed_mature_weight, counted_signed_fresh_weight)?;

    if actual_signed_weight < required_signed_weight {
        return Err(ServiceNodeQuorumWeightError::InsufficientWeightedQuorum {
            required: required_signed_weight,
            actual: actual_signed_weight,
        });
    }

    let eligible_member_count = u16::try_from(members.len())
        .map_err(|_| ServiceNodeQuorumWeightError::ArithmeticOverflow)?;

    let signature_count = u16::try_from(quorum.signatures.len())
        .map_err(|_| ServiceNodeQuorumWeightError::ArithmeticOverflow)?;

    Ok(ServiceNodeQuorumWeightReviewV1 {
        current_epoch,
        eligible_member_count,
        mature_member_count,
        fresh_member_count,
        raw_mature_weight,
        raw_fresh_weight,
        maximum_counted_fresh_weight,
        counted_fresh_weight,
        total_counted_weight,
        required_signed_weight,
        actual_signed_weight,
        signature_count,
        members,
    })
}

fn require_equal(
    expected: &str,
    actual: &str,
    service_node_id: &str,
    field: &'static str,
) -> Result<(), ServiceNodeQuorumWeightError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ServiceNodeQuorumWeightError::EligibilityFieldMismatch {
            service_node_id: service_node_id.to_string(),
            field,
        })
    }
}

fn maximum_fresh_weight(
    mature_weight: u64,
    maximum_fresh_weight_bps: u16,
) -> Result<u64, ServiceNodeQuorumWeightError> {
    if maximum_fresh_weight_bps == 0 {
        return Ok(0);
    }

    let numerator = mature_weight
        .checked_mul(u64::from(maximum_fresh_weight_bps))
        .ok_or(ServiceNodeQuorumWeightError::ArithmeticOverflow)?;

    let denominator = u64::from(BPS_DENOMINATOR - maximum_fresh_weight_bps);

    Ok(numerator / denominator)
}

fn apply_bps_ceil(value: u64, basis_points: u16) -> Result<u64, ServiceNodeQuorumWeightError> {
    value
        .checked_mul(u64::from(basis_points))
        .map(|numerator| numerator.div_ceil(u64::from(BPS_DENOMINATOR)))
        .ok_or(ServiceNodeQuorumWeightError::ArithmeticOverflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, ServiceNodeQuorumWeightError> {
    left.checked_add(right)
        .ok_or(ServiceNodeQuorumWeightError::ArithmeticOverflow)
}

const fn invalid_policy<T>(
    field: &'static str,
    reason: &'static str,
) -> Result<T, ServiceNodeQuorumWeightError> {
    Err(ServiceNodeQuorumWeightError::InvalidPolicy { field, reason })
}
