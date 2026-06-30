#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 Round 2 preflight for svc-rewarder anti-farming and event-class gates.
# RO:WHY — Proves rewarder consumes only verified/capped inputs, rejects raw engagement/direct metering, and keeps ad-budgeted material outside protocol-pool emission.
# RO:INTERACTS — anti_farming input gates, config-driven planning, Phase 3 reward-plan/payout intent boundaries.
# RO:INVARIANTS — rewarder plans only; no wallet/ledger mutation, fake receipt, fake balance, fake finality, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — caps are validated input fixtures until economics config wiring is extended.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets or new mutation path.
# RO:TEST — bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-rewarder"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'svc-rewarder internal ROC beta phase5 round2 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f "$CRATE_DIR/src/inputs/anti_farming.rs" ] || fail "missing anti-farming input gate"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_antifarming_event_gates.rs" ] || fail "missing Phase 5 Round 2 anti-farming test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "svc-rewarder Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p svc-rewarder -- --check
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
"$CARGO" clippy -p svc-rewarder --all-targets -- -D warnings

printf '\n== Internal ROC Beta Phase 5 Round 2 svc-rewarder anti-farming/event-class preflight passed ==\n'
printf '== rewarder consumes verified/capped/policy-gated inputs only; no raw-engagement protocol payout, wallet/ledger authority, bridge, staking, liquidity, or external settlement ==\n'
