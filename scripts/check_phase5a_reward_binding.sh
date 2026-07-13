#!/usr/bin/env bash
# RO:WHAT — Phase 5A reward-recipient binding preflight.
# RO:WHY — Locks the DTO, registry, rotation, payout-override, and self-pay guard checkpoint.
# RO:INVARIANTS — no wallet mutation; no ledger mutation; no live payout; no fake reward finality.
# RO:TEST — bash scripts/check_phase5a_reward_binding.sh.

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

echo "== Phase 5A reward binding preflight =="
echo "Repo: $(pwd)"

echo
echo "---- BUILD_PLAN_Z reward-binding doctrine markers ----"
grep -n "Node Operator Reward Recipient Binding" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No service evidence can redirect payout" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No node self-pay shortcut exists" BUILD_PLAN_Z.md || failures=$((failures + 1))

run_check cargo test -p ron-proto --test service_node_reward_binding
run_check cargo test -p svc-registry --test reward_binding_registry
run_check cargo test -p svc-registry --test reward_payout_guard
run_check cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
run_check cargo check -p ron-proto
run_check cargo check -p svc-registry
run_check cargo check --workspace

echo
if [ "$failures" -eq 0 ]; then
  echo "✅ Phase 5A reward binding preflight passed"
  echo "✅ DTOs, registry resolution, rotation, payout override rejection, and self-pay guard are green"
  exit 0
fi

echo "❌ Phase 5A reward binding preflight failed: failures=$failures" >&2
exit 1
