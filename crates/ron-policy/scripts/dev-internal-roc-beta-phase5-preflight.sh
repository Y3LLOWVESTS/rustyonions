#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 preflight for ron-policy economics TOML validation.
# RO:WHY — Proves policy validates canonical tokenomics config without becoming receipt, balance, payout, or finality truth.
# RO:INTERACTS — configs/roc-economics.toml, ron-policy economics validators, Phase 5 tests.
# RO:INVARIANTS — no floats; exact bps totals; explicit remainder sink; bridge/staking inert; policy is gate only.
# RO:METRICS — none.
# RO:CONFIG — validates configs/roc-economics.toml as caller-provided bytes.
# RO:SECURITY — bash/cargo-only; no secrets, wallet mutation, ledger mutation, bridge, staking, or external settlement.
# RO:TEST — bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-policy"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'ron-policy internal ROC beta phase5 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f configs/roc-economics.toml ] || fail "missing configs/roc-economics.toml"
[ -f "$CRATE_DIR/src/economics/internal_roc.rs" ] || fail "missing internal ROC economics validator"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs" ] || fail "missing Phase 5 policy validation test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "ron-policy Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p ron-policy -- --check
"$CARGO" test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
"$CARGO" test -p ron-policy --test economics_policy
"$CARGO" clippy -p ron-policy --all-targets --no-deps -- -D warnings

printf '\n== Internal ROC Beta Phase 5 ron-policy economics TOML validation preflight passed ==\n'
printf '== policy validates tokenomics config only; no receipt/balance/payout/finality truth, bridge, staking, liquidity, or external settlement ==\n'
