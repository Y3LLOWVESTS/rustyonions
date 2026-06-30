#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 2 preflight for svc-wallet.
# RO:WHY — Proves wallet paid-action idempotency and receipt lookup stability without adding a new ledger mutation path.
# RO:INTERACTS — svc-wallet tests, cargo fmt, cargo clippy.
# RO:INVARIANTS — svc-wallet remains the only mutation front-door; idempotency_key is retry identity only; no fake receipts, fake balances, fake finality, silent spend, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — crates/svc-wallet/scripts/dev-internal-roc-beta-phase2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-wallet"

cd "$ROOT_DIR"

echo "== Internal ROC Beta Phase 2 svc-wallet focused preflight =="
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
cargo fmt -p svc-wallet -- --check
echo

echo "== focused Internal ROC Beta Phase 2 tests =="
cargo test -p svc-wallet --test internal_roc_beta_phase2_paid_action_idempotency
cargo test -p svc-wallet --test internal_roc_beta_phase2_receipt_lookup_after_replay
echo

echo "== prior paid-content receipt-path regression =="
cargo test -p svc-wallet --test internal_roc_beta_paid_content_receipt_path
echo

echo "== strict clippy gate =="
cargo clippy -p svc-wallet --all-targets -- -D warnings
echo

printf '\n== Internal ROC Beta Phase 2 svc-wallet idempotency/receipt lookup preflight passed ==\n'
printf '== paid retries replay or safely conflict; cancelled paid actions do not mutate; receipt lookup rehydrates accepted backend receipts only ==\n'
