#!/usr/bin/env bash
# RO:WHAT — Focused svc-rewarder Internal ROC Stabilization reward-policy gate preflight.
# RO:WHY — Product beta readiness needs reward planning to stay capped, policy-gated, deterministic, and non-mutating.
# RO:INVARIANTS — rewarder plans only; ron-policy gates; svc-wallet mutates; ron-ledger records truth.
# RO:SECURITY — no raw engagement payout, fake receipt/balance/finality, direct ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash crates/svc-rewarder/scripts/dev-internal-roc-stabilization-reward-policy-gate-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder --test internal_roc_stabilization_reward_policy_gate_boundary
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
cargo test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo clippy -p svc-rewarder --all-targets -- -D warnings

echo "== svc-rewarder Internal ROC Stabilization reward-policy gate preflight passed =="
echo "== capped planning, anti-farming, policy gate, wallet handoff, and non-mutation boundaries locked =="
