#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 3 preflight for svc-wallet.
# RO:WHY — Proves approved payouts execute only through svc-wallet's wallet/ledger mutation path, and accounting/reward-plan material cannot create balances, receipts, payouts, finality, bridge, staking, liquidity, or external settlement.
# RO:INTERACTS — svc-wallet Phase 3 tests, prior Internal ROC regressions, cargo fmt, cargo clippy.
# RO:INVARIANTS — svc-wallet remains the only approved mutation front-door; accounting observes only; reward plans/snapshots cannot smuggle wallet operations; no fake balances, fake receipts, fake finality, raw-engagement minting, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/svc-wallet/scripts/dev-internal-roc-beta-phase3-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-wallet"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 3 svc-wallet focused preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== tooling boundary: no checked-in Python helpers under svc-wallet =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "svc-wallet Internal ROC Beta preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
"$CARGO" fmt -p svc-wallet -- --check
echo

echo "== focused Internal ROC Beta Phase 3 Round 2 approved payout execution boundary test =="
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase3_approved_payout_execution_boundary
echo

echo "== focused Internal ROC Beta Phase 3 Round 1 accounting observer boundary test =="
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase3_accounting_observer_boundary
echo

echo "== prior Internal ROC paid-content receipt path regression =="
"$CARGO" test -p svc-wallet --test internal_roc_beta_paid_content_receipt_path
echo

echo "== prior Internal ROC Phase 2 idempotency regression =="
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase2_paid_action_idempotency
echo

echo "== prior Internal ROC Phase 2 receipt lookup replay regression =="
"$CARGO" test -p svc-wallet --test internal_roc_beta_phase2_receipt_lookup_after_replay
echo

echo "== strict clippy gate =="
"$CARGO" clippy -p svc-wallet --all-targets -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 3 svc-wallet approved-payout preflight passed ==\n'
printf '== approved payouts execute only through svc-wallet wallet-ledger receipts; accounting/reward-plan material cannot bypass wallet or create balance/receipt/payout/finality truth ==\n'
