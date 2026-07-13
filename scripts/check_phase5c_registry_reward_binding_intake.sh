#!/usr/bin/env bash
# RO:WHAT — Phase 5C registry-backed reward binding request-intake preflight.
# RO:WHY — Locks registry-backed binding/rotation intake over strict ron-proto DTOs.
# RO:INVARIANTS — no wallet mutation; no ledger mutation; no confirmed ROC; no evidence payout override.
# RO:TEST — bash scripts/check_phase5c_registry_reward_binding_intake.sh.

failures=0

run_check() {
  echo
  echo "---- $* ----"
  "$@"
  status=$?
  if [ "$status" -ne 0 ]; then
    echo "FAILED: $*" >&2
    failures=$((failures + 1))
  fi
}

echo "== Phase 5C registry reward binding intake preflight =="
echo "Repo: $(pwd)"

echo
echo "---- BUILD_PLAN_Z reward-binding doctrine markers ----"
grep -n "Node Operator Reward Recipient Binding" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No service evidence can redirect payout" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No node self-pay shortcut exists" BUILD_PLAN_Z.md || failures=$((failures + 1))

run_check cargo test -p svc-registry --test reward_binding_request_intake
run_check cargo test -p svc-registry --test reward_binding_registry
run_check cargo test -p svc-registry --test reward_payout_guard
run_check cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
run_check cargo test -p ron-proto --test service_node_reward_binding
run_check cargo check -p ron-proto
run_check cargo check -p svc-registry
run_check cargo clippy -p svc-registry --all-targets --no-deps -- -D warnings

echo
echo "---- Phase 5B dependency preflight ----"
bash scripts/check_phase5b_crabnode_rewards.sh
phase5b_status=$?
if [ "$phase5b_status" -ne 0 ]; then
  echo "FAILED: Phase 5B crabnode rewards dependency preflight" >&2
  failures=$((failures + 1))
fi

run_check cargo check --workspace

echo
if [ "$failures" -eq 0 ]; then
  echo "✅ Phase 5C registry reward binding intake preflight passed"
  echo "✅ Registry-backed binding/rotation intake and Phase 5B operator surface are green"
  echo "✅ No wallet mutation, ledger mutation, confirmed ROC, evidence payout override, or fake payout finality is exposed"
  exit 0
fi

echo "❌ Phase 5C registry reward binding intake preflight failed: failures=$failures" >&2
exit 1
