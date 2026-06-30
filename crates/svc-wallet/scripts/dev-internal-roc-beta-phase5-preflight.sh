#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 preflight for svc-wallet economics-config non-authority.
# RO:WHY — Proves canonical tokenomics config cannot directly become wallet mutation, balance, receipt, payout, finality, bridge, staking, liquidity, or external-settlement authority.
# RO:INTERACTS — configs/roc-economics.toml, svc-wallet Phase 5 tests, prior Internal ROC regressions, cargo fmt, cargo clippy.
# RO:INVARIANTS — svc-wallet remains mutation front-door; config is label/input only, never wallet authority.
# RO:METRICS — none.
# RO:CONFIG — validates canonical economics config remains outside wallet mutation DTOs.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets or new ledger mutation path.
# RO:TEST — bash crates/svc-wallet/scripts/dev-internal-roc-beta-phase5-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-wallet"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'svc-wallet internal ROC beta phase5 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f configs/roc-economics.toml ] || fail "missing configs/roc-economics.toml"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_config_non_authority.rs" ] || fail "missing Phase 5 config non-authority test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "svc-wallet Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p svc-wallet -- --check
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase5_config_non_authority
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase3_approved_payout_execution_boundary
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase3_accounting_observer_boundary
"$CARGO" test -p svc-wallet --test internal_roc_beta_paid_content_receipt_path
"$CARGO" clippy -p svc-wallet --all-targets -- -D warnings

printf '\n== Internal ROC Beta Phase 5 svc-wallet config non-authority preflight passed ==\n'
printf '== economics config cannot directly mutate wallet/ledger or create receipt/balance/payout/unlock/finality truth ==\n'
