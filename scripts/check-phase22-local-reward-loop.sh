#!/usr/bin/env bash
# RO:WHAT — Focused Phase 22G local economic-loop acceptance runner.
# RO:WHY — Proves the existing economic crates compose into one real local loop.
# RO:INVARIANTS — canonical economics; registry-derived recipients; quorum-only
# execution; durable replay; no single-node or self-issued payout.
# RO:TEST — bash scripts/check-phase22-local-reward-loop.sh.

set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.."
  pwd
)"

cd "$ROOT"

echo
echo "== Phase 22G: integrated local ROC reward loop =="
echo

cargo test \
  -p svc-rewarder \
  --test internal_roc_beta_phase22_local_reward_loop \
  -- \
  --nocapture

echo
echo "== Phase 22G: accounting regression =="
echo

cargo test \
  -p ron-accounting \
  --test epoch_accounting_snapshot

echo
echo "== Phase 22G: reward planning regressions =="
echo

cargo test \
  -p svc-rewarder \
  --test internal_roc_beta_phase14d_accounting_epoch_handoff

cargo test \
  -p svc-rewarder \
  --test internal_roc_beta_phase14d_service_node_reward_plan

echo
echo "== Phase 22G: registry regressions =="
echo

cargo test \
  -p svc-registry \
  --test reward_binding_registry

cargo test \
  -p svc-registry \
  --test reward_payout_guard

echo
echo "== Phase 22G: wallet and ledger regression =="
echo

cargo test \
  -p svc-wallet \
  --test internal_roc_beta_phase16_quorum_execution

echo
echo "== Phase 22G: User Node replay regression =="
echo

cargo test \
  -p micronode \
  --test internal_roc_beta_phase17_epoch_replay

echo
echo "Phase 22G local reward loop passed."
echo "Evidence, accounting, policy, capped planning, registry resolution,"
echo "quorum, wallet, ledger, replay, and challenge behavior composed."
echo "Self-issuance and single-node minting were rejected."
