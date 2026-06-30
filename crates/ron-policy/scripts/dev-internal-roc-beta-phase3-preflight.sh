#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 3 preflight for ron-policy.
# RO:WHY — Proves policy can gate reward plans and approved payout candidates declaratively without creating receipts, balances, payout execution, wallet mutation, ledger mutation, finality, bridge, staking, liquidity, or external settlement.
# RO:INTERACTS — ron-policy Phase 3 tests, prior paid-content policy non-authority tests, decision non-authority tests, cargo fmt, cargo clippy.
# RO:INVARIANTS — policy validates/gates only; svc-wallet remains mutation front-door; ron-ledger remains truth; policy allow/deny/obligation/config is not economic truth.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-policy"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 3 ron-policy focused preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== tooling boundary: no checked-in Python helpers under ron-policy =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "ron-policy Internal ROC Beta preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
"$CARGO" fmt -p ron-policy -- --check
echo

echo "== focused Internal ROC Beta Phase 3 Round 2 approved payout policy gate test =="
"$CARGO" test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
echo

echo "== focused Internal ROC Beta Phase 3 Round 1 reward-plan policy gate test =="
"$CARGO" test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
echo

echo "== prior Internal ROC paid-content policy non-authority regression =="
"$CARGO" test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
echo

echo "== QuickChain decision non-authority regression =="
"$CARGO" test -p ron-policy --test quickchain_preflight_decision_non_authority
echo

echo "== economics policy regression =="
"$CARGO" test -p ron-policy --test economics_policy
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p ron-policy --all-targets --no-deps -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 3 ron-policy approved-payout gate preflight passed ==\n'
printf '== policy remains declarative gate only; no receipt/balance/payout/finality truth, wallet/ledger mutation, bridge, staking, liquidity, or external settlement ==\n'
