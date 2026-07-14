//! RO:WHAT — Pure reward computation module facade.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/GOV. Keeps deterministic math isolated from HTTP and IO.
//! RO:INTERACTS — inputs, outputs, http handlers.
//! RO:INVARIANTS — no floats; checked arithmetic; no ledger mutation.
//! RO:METRICS — callers time compute and classify errors.
//! RO:CONFIG — idempotency salt and policy knobs are passed in.
//! RO:SECURITY — malformed inputs fail before egress.
//! RO:TEST — unit/integration tests.

pub mod algebra;
pub mod compute;
pub mod invariants;

pub use algebra::{checked_mul_div_floor, AmountMinor};
pub use compute::{compute_manifest, compute_manifest_with_economics, run_key, ComputeInput};
pub use invariants::{validate_payouts, InvariantReport};

pub mod service_node_eligibility_plan;
pub mod service_node_enforcement_review;
pub mod service_node_plan;

pub use service_node_eligibility_plan::{
    compute_service_node_reward_plan_with_eligibility, ServiceNodeEligibilityRewardPlan,
    SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_SCHEMA, SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_VERSION,
};

pub use service_node_enforcement_review::{
    compute_service_node_reward_review_with_enforcement, ServiceNodeEnforcementRewardReview,
    ServiceNodeRewardDenialV1, SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_SCHEMA,
    SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_VERSION,
};

pub use service_node_plan::{
    compute_service_node_reward_plan, ServiceNodeRewardAllocation, ServiceNodeRewardCandidate,
    ServiceNodeRewardEvidenceClass, ServiceNodeRewardPlan, ServiceNodeRewardPlanInput,
    ServiceNodeRewardPlanTotals, SERVICE_NODE_REWARD_PLAN_SCHEMA, SERVICE_NODE_REWARD_PLAN_VERSION,
};
