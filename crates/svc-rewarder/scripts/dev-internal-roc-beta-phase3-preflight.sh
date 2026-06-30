#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 3 preflight for svc-rewarder.
# RO:WHY — Proves rewarder consumes deterministic accounting snapshots and emits capped planning/handoff material only, without wallet/ledger mutation or fake receipt/balance/finality truth.
# RO:INTERACTS — svc-rewarder Phase 3 tests, prior rewarder planning tests, QuickChain non-mutation/replay regressions, cargo fmt, cargo clippy.
# RO:INVARIANTS — svc-rewarder plans only; svc-wallet remains mutation front-door; ron-ledger remains truth; no raw-engagement direct ROC allocation; no bridge/staking/liquidity/external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-rewarder"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 3 svc-rewarder focused preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== tooling boundary: no checked-in Python helpers under svc-rewarder =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "svc-rewarder Internal ROC Beta preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
"$CARGO" fmt -p svc-rewarder -- --check
echo

echo "== focused Internal ROC Beta Phase 3 Round 2 approved payout intent boundary test =="
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
echo

echo "== focused Internal ROC Beta Phase 3 Round 1 reward-plan boundary test =="
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
echo

echo "== prior Internal ROC rewarder planning non-authority regression =="
"$CARGO" test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
echo

echo "== QuickChain rewarder no-direct-mutation regression =="
"$CARGO" test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
echo

echo "== QuickChain rewarder funding-source regression =="
"$CARGO" test -p svc-rewarder --test quickchain_preflight_funding_source
echo

echo "== QuickChain rewarder replay/no-double-issue regression =="
"$CARGO" test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p svc-rewarder --all-targets -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 3 svc-rewarder approved-payout intent preflight passed ==\n'
printf '== rewarder emits deterministic capped wallet handoff candidates only; no wallet/ledger mutation, receipt/balance/finality truth, raw-engagement direct ROC allocation, bridge, staking, liquidity, or external settlement ==\n'
