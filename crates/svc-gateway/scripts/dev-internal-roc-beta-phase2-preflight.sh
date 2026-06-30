#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 2 Round 2 preflight for svc-gateway.
# RO:WHY — Proves downstream replay/conservation/audit visibility remains read-only display metadata and cannot become paid unlock, receipt, balance, finality, settlement, bridge, staking, or liquidity authority.
# RO:INTERACTS — svc-gateway tests, paid-content route regression, QuickChain replay/status boundary regressions, cargo fmt, cargo clippy.
# RO:INVARIANTS — no gateway direct ledger mutation; no fake receipts, fake balances, fake finality, silent spend; no replay-status unlock; no bridge, staking, liquidity, exchange-facing, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/svc-gateway/scripts/dev-internal-roc-beta-phase2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-gateway"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 2 Round 2 svc-gateway focused preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== tooling boundary: no checked-in Python helpers under svc-gateway =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "svc-gateway Internal ROC Beta preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
"$CARGO" fmt -p svc-gateway -- --check
echo

echo "== focused Internal ROC Beta Phase 2 Round 2 replay visibility boundary =="
"$CARGO" test -p svc-gateway --test internal_roc_beta_phase2_replay_visibility_boundary
echo

echo "== paid content route regression =="
"$CARGO" test -p svc-gateway --test internal_roc_beta_paid_content_route_boundary
echo

echo "== replay/status authority regressions =="
"$CARGO" test -p svc-gateway --test quickchain_phase2_replay_boundary
"$CARGO" test -p svc-gateway --test quickchain_phase5_anchor_status_boundary
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p svc-gateway --all-targets --no-deps -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 2 Round 2 svc-gateway replay/audit visibility preflight passed ==\n'
printf '== replay status remains read-only display metadata; no fake receipts, fake balances, fake finality, silent spend, replay-status unlock, bridge, staking, liquidity, or external settlement ==\n'
