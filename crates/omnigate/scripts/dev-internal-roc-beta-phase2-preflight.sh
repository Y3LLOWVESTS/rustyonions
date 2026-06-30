#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 2 Round 2 preflight for omnigate.
# RO:WHY — Proves hydrated replay/conservation/audit visibility remains read-only display context and cannot become paid unlock, receipt, balance, finality, settlement, bridge, staking, or liquidity authority.
# RO:INTERACTS — omnigate tests, paid-content access regression, QuickChain replay/hydration boundary regressions, cargo fmt, cargo clippy.
# RO:INVARIANTS — no omnigate direct ledger mutation; no fake receipts, fake balances, fake finality, silent spend; no replay-hydration unlock; no bridge, staking, liquidity, exchange-facing, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/omnigate/scripts/dev-internal-roc-beta-phase2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/omnigate"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 2 Round 2 omnigate focused preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== tooling boundary: no checked-in Python helpers under omnigate =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "omnigate Internal ROC Beta preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
"$CARGO" fmt -p omnigate -- --check
echo

echo "== focused Internal ROC Beta Phase 2 Round 2 replay visibility boundary =="
"$CARGO" test -p omnigate --test internal_roc_beta_phase2_replay_visibility_boundary
echo

echo "== paid content access regression =="
"$CARGO" test -p omnigate --test internal_roc_beta_paid_content_access_boundary
echo

echo "== replay/hydration authority regressions =="
"$CARGO" test -p omnigate --test quickchain_phase2_replay_boundary
"$CARGO" test -p omnigate --test quickchain_phase5_anchor_hydration_boundary
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p omnigate --all-targets --no-deps -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 2 Round 2 omnigate replay/audit hydration preflight passed ==\n'
printf '== replay hydration remains read-only display context; no fake receipts, fake balances, fake finality, silent spend, replay-hydration unlock, bridge, staking, liquidity, or external settlement ==\n'
