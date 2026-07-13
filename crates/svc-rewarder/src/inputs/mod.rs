//! RO:WHAT — Input resolvers and DTOs for reward computation.
//! RO:WHY — Keep accounting, policy, economics config, and content-id validation at ingress edges.

pub mod accounting;
pub mod accounting_epoch;
pub mod anti_farming;
pub mod cache;
pub mod cid;
pub mod economics;
pub mod ledger_snapshot;
pub mod policy;

pub use accounting::{
    canonical_snapshot_cid, resolve_accounting_snapshot, AccountContribution, AccountingSnapshot,
};
pub use accounting_epoch::{
    accounting_epoch_reward_planning_handoff, AccountingEpochRewardPlanningHandoffV1,
    UserVerificationPlannedPointV1, UserVerificationPointPlanV1,
    ACCOUNTING_EPOCH_REWARD_HANDOFF_SCHEMA, ACCOUNTING_EPOCH_REWARD_HANDOFF_VERSION,
    USER_VERIFICATION_POINT_PLAN_SCHEMA, USER_VERIFICATION_POINT_PLAN_VERSION,
};
pub use anti_farming::{
    capped_contributions_from_candidates, capped_contributions_from_candidates_with_economics,
    AntiFarmingCapPolicy, CappedRewardInputCandidate, RewardInputEventClass,
};
pub use cid::ContentCid;
pub use economics::{
    load_canonical_internal_roc_planning_economics, load_internal_roc_planning_economics_toml,
    load_internal_roc_planning_economics_toml_str, InternalRocRewardCategoryPlanningCap,
    InternalRocRewardPlanningEconomics,
};
pub use ledger_snapshot::LedgerSnapshot;
pub use policy::{
    policy_hash_is_canonical, resolve_reward_policy, validate_reward_policy, RewardFundingSource,
    RewardPolicy,
};
