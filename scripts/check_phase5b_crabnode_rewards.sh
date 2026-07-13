#!/usr/bin/env bash
# RO:WHAT — Phase 5B crabnode reward-recipient operator surface preflight.
# RO:WHY — Locks CLI, HTTP, and live CLI→macronode reward-recipient binding checks.
# RO:INVARIANTS — no wallet mutation; no ledger mutation; no confirmed ROC; no fake payout finality.
# RO:TEST — bash scripts/check_phase5b_crabnode_rewards.sh.

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

echo "== Phase 5B crabnode rewards preflight =="
echo "Repo: $(pwd)"

echo
echo "---- BUILD_PLAN_Z reward-binding doctrine markers ----"
grep -n "Node Operator Reward Recipient Binding" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No service evidence can redirect payout" BUILD_PLAN_Z.md || failures=$((failures + 1))
grep -n "No node self-pay shortcut exists" BUILD_PLAN_Z.md || failures=$((failures + 1))

run_check cargo test -p macronode --test crabnode_rewards_live
run_check cargo test -p macronode --test rewards_http
run_check cargo test -p macronode --test crabnode_rewards_cli
run_check cargo test -p macronode --test crabnode_cli
run_check cargo test -p macronode --test crabnode_setup_cli
run_check cargo check -p macronode --bins
run_check cargo clippy -p macronode --all-targets --no-deps -- -D warnings

echo
echo "---- Phase 5A dependency preflight ----"
bash scripts/check_phase5a_reward_binding.sh
phase5a_status=$?
if [ "$phase5a_status" -ne 0 ]; then
  echo "FAILED: Phase 5A reward binding dependency preflight" >&2
  failures=$((failures + 1))
fi

run_check cargo check --workspace

echo
if [ "$failures" -eq 0 ]; then
  echo "✅ Phase 5B crabnode rewards preflight passed"
  echo "✅ CLI, HTTP endpoints, live CLI→macronode flow, and Phase 5A dependency checks are green"
  echo "✅ No wallet mutation, ledger mutation, confirmed ROC, or fake payout finality is exposed"
  exit 0
fi

echo "❌ Phase 5B crabnode rewards preflight failed: failures=$failures" >&2
exit 1
