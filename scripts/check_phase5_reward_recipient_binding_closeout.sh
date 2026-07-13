#!/usr/bin/env bash
# RO:WHAT — Phase 5 reward-recipient binding closeout preflight.
# RO:WHY — Chains Phase 5A/5B/5C into one green checkpoint before carry-over notes.
# RO:INVARIANTS — no wallet mutation; no ledger mutation; no confirmed ROC; no fake payout finality.
# RO:TEST — bash scripts/check_phase5_reward_recipient_binding_closeout.sh.

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

echo "== Phase 5 reward-recipient binding closeout =="
echo "Repo: $(pwd)"

echo
echo "---- BUILD_PLAN_Z Phase 5 doctrine markers ----"
grep -n "## Phase 5 — Node Operator Reward Recipient Binding" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No service evidence can redirect payout" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No node self-pay shortcut exists" BUILD_PLAN_Z.md || failures=$((failures + 1))

echo
echo "---- Phase 5A: DTOs, registry, payout guard ----"
bash scripts/check_phase5a_reward_binding.sh
phase5a_status=$?
if [ "$phase5a_status" -ne 0 ]; then
  echo "FAILED: Phase 5A reward binding preflight" >&2
  failures=$((failures + 1))
fi

echo
echo "---- Phase 5B: crabnode/macronode rewards surface ----"
bash scripts/check_phase5b_crabnode_rewards.sh
phase5b_status=$?
if [ "$phase5b_status" -ne 0 ]; then
  echo "FAILED: Phase 5B crabnode rewards preflight" >&2
  failures=$((failures + 1))
fi

echo
echo "---- Phase 5C: registry-backed intake ----"
bash scripts/check_phase5c_registry_reward_binding_intake.sh
phase5c_status=$?
if [ "$phase5c_status" -ne 0 ]; then
  echo "FAILED: Phase 5C registry intake preflight" >&2
  failures=$((failures + 1))
fi

echo
echo "---- Focused direct closeout checks ----"
run_check cargo test -p ron-proto --test service_node_reward_binding
run_check cargo test -p svc-registry --test reward_binding_request_intake
run_check cargo test -p svc-registry --test reward_binding_registry
run_check cargo test -p svc-registry --test reward_payout_guard
run_check cargo test -p macronode --test crabnode_rewards_live
run_check cargo test -p macronode --test rewards_http
run_check cargo test -p macronode --test crabnode_rewards_cli

echo
echo "---- Focused compile/clippy closeout checks ----"
run_check cargo check -p ron-proto
run_check cargo check -p svc-registry
run_check cargo check -p macronode --bins
run_check cargo clippy -p svc-registry --all-targets --no-deps -- -D warnings
run_check cargo clippy -p macronode --all-targets --no-deps -- -D warnings

echo
echo "---- Workspace closeout check ----"
run_check cargo check --workspace

echo
if [ "$failures" -eq 0 ]; then
  echo "✅ Phase 5 reward-recipient binding closeout passed"
  echo "✅ Phase 5A/5B/5C are green"
  echo "✅ Operators can request/display reward-recipient binding and rotation through crabnode/macronode"
  echo "✅ svc-registry can intake strict binding/rotation requests and reject payout override/self-pay shortcuts"
  echo "✅ No wallet mutation, ledger mutation, confirmed ROC, evidence payout override, or fake payout finality is exposed"
  exit 0
fi

echo "❌ Phase 5 reward-recipient binding closeout failed: failures=$failures" >&2
exit 1
