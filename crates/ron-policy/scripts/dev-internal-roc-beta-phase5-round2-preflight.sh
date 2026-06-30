#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 Round 2 preflight for ron-policy anti-farming policy gates.
# RO:WHY — Proves policy gates reward eligibility declaratively and rejects raw engagement, analytics-only, direct metering, uncapped, unverified, and unfunded ad-budgeted reward material.
# RO:INTERACTS — ron-policy parser/evaluator, Phase 5 economics TOML validation, Phase 3 reward-plan/approved-payout policy gates.
# RO:INVARIANTS — policy is a declarative gate only; no receipt, balance, payout execution, finality, wallet/ledger mutation, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets or new mutation path.
# RO:TEST — bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-policy"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'ron-policy internal ROC beta phase5 round2 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_antifarming_policy_gate.rs" ] || fail "missing Phase 5 Round 2 anti-farming policy gate test"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs" ] || fail "missing Phase 5 economics TOML policy validation test"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase3_reward_plan_policy_gate.rs" ] || fail "missing Phase 3 reward-plan policy gate test"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase3_approved_payout_policy_gate.rs" ] || fail "missing Phase 3 approved-payout policy gate test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "ron-policy Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p ron-policy -- --check
"$CARGO" test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate
"$CARGO" test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
"$CARGO" test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
"$CARGO" test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
"$CARGO" test -p ron-policy --test quickchain_preflight_decision_non_authority
"$CARGO" clippy -p ron-policy --all-targets --no-deps -- -D warnings

printf '\n== Internal ROC Beta Phase 5 Round 2 ron-policy anti-farming policy-gate preflight passed ==\n'
printf '== policy gates reward eligibility only; no raw-engagement protocol payout, receipt/balance/finality truth, wallet/ledger mutation, bridge, staking, liquidity, or external settlement ==\n'
