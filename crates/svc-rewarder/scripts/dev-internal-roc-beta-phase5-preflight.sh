#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 preflight for svc-rewarder config-driven planning.
# RO:WHY — Proves rewarder consumes validated tokenomics config for planning only and has no hard-coded payout constants.
# RO:INTERACTS — configs/roc-economics.toml, inputs/economics.rs, Phase 5 rewarder tests.
# RO:INVARIANTS — rewarder plans only; no wallet/ledger mutation; no fake receipt/balance/finality; bridge/staking inert.
# RO:METRICS — none.
# RO:CONFIG — validates canonical economics TOML as planning input.
# RO:SECURITY — bash/cargo-only; no secrets, bridge, staking, liquidity, or external settlement.
# RO:TEST — bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-rewarder"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'svc-rewarder internal ROC beta phase5 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f configs/roc-economics.toml ] || fail "missing configs/roc-economics.toml"
[ -f "$CRATE_DIR/src/inputs/economics.rs" ] || fail "missing rewarder economics planning projection"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_config_driven_planning.rs" ] || fail "missing Phase 5 config-driven planning test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "svc-rewarder Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p svc-rewarder -- --check
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
"$CARGO" test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
"$CARGO" clippy -p svc-rewarder --all-targets -- -D warnings

printf '\n== Internal ROC Beta Phase 5 svc-rewarder config-driven planning preflight passed ==\n'
printf '== rewarder consumes validated economics config for planning only; no hard-coded payout constants or wallet/ledger authority ==\n'
