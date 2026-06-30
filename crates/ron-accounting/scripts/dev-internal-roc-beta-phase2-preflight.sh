#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 2 preflight for ron-accounting.
# RO:WHY — Proves accounting snapshots observe accepted receipts only and cannot alter replay, balances, receipts, entitlement, or finality truth.
# RO:INTERACTS — ron-accounting tests, cargo fmt, cargo clippy.
# RO:INVARIANTS — ron-accounting remains derivative snapshot/report infrastructure; no wallet mutation, ledger mutation, fake receipts, fake balances, fake finality, paid unlock authority, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/ron-accounting/scripts/dev-internal-roc-beta-phase2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-accounting"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 2 ron-accounting focused preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== tooling boundary: no checked-in Python helpers under ron-accounting =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "ron-accounting Internal ROC Beta preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
"$CARGO" fmt -p ron-accounting -- --check
echo

echo "== focused Internal ROC Beta Phase 2 test =="
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase2_snapshot_cannot_change_replay
echo

echo "== prior paid-content snapshot non-authority regression =="
"$CARGO" test -p ron-accounting --test internal_roc_beta_paid_content_snapshot_non_authority
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p ron-accounting --all-targets -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 2 ron-accounting snapshot/replay non-authority preflight passed ==\n'
printf '== accepted receipts are observation input only; cancelled actions do not become receipt input; snapshots cannot alter replay or create balance/receipt truth ==\n'
