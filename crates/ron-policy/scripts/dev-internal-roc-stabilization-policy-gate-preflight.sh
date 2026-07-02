#!/usr/bin/env bash
# RO:WHAT — Focused ron-policy Internal ROC Stabilization declarative gate preflight.
# RO:WHY — Product beta readiness needs policy decisions and economics config to remain declarative gates only.
# RO:INVARIANTS — policy allows/denies/explains only; no receipt/balance/payout/finality/wallet/ledger authority.
# RO:SECURITY — no fake receipt/balance/finality, paid unlock, payout execution, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash crates/ron-policy/scripts/dev-internal-roc-stabilization-policy-gate-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p ron-policy -- --check
cargo test -p ron-policy --test internal_roc_stabilization_policy_gate_non_authority_boundary
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
cargo test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
cargo test -p ron-policy --test economics_policy
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings

echo "== ron-policy Internal ROC Stabilization policy-gate preflight passed =="
echo "== declarative decisions, economics validation, anti-farming gates, and non-authority boundaries locked =="
