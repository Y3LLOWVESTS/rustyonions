#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 3 preflight for ron-accounting.
# RO:WHY — Proves accounting observes executed payout receipts as deterministic derivative snapshot material without balance, receipt, payout, finality, bridge, staking, liquidity, or external-settlement authority.
# RO:INTERACTS — ron-accounting Phase 3 tests, prior Internal ROC regressions, cargo fmt, cargo clippy.
# RO:INVARIANTS — ron-accounting remains derivative snapshot/report infrastructure; no wallet mutation, ledger mutation, fake receipts, fake balances, fake finality, paid unlock authority, raw-engagement direct ROC minting, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/ron-accounting/scripts/dev-internal-roc-beta-phase3-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-accounting"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 3 ron-accounting focused preflight =="
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

echo "== focused Internal ROC Beta Phase 3 Round 2 approved payout observation boundary test =="
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase3_approved_payout_observation_boundary
echo

echo "== focused Internal ROC Beta Phase 3 Round 1 accounting snapshot/event-class boundary test =="
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary
echo

echo "== prior Internal ROC Phase 2 snapshot/replay regression =="
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase2_snapshot_cannot_change_replay
echo

echo "== prior Internal ROC paid-content snapshot non-authority regression =="
"$CARGO" test -p ron-accounting --test internal_roc_beta_paid_content_snapshot_non_authority
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p ron-accounting --all-targets -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 3 ron-accounting approved-payout observation preflight passed ==\n'
printf '== accounting observes executed payout receipts as derivative snapshot material only; no balance/receipt/payout/finality truth and no raw-engagement direct ROC allocation ==\n'
