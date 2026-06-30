#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"

run() {
  printf '\n== %s ==\n' "$*"
  "$@"
}

require_file() {
  if [[ ! -f "$1" ]]; then
    printf 'missing required Phase 6 wallet/ledger proof file: %s\n' "$1" >&2
    exit 1
  fi
}

printf '== Internal ROC Beta Phase 6 wallet/ledger proof target ==\n'
printf '== scope: svc-wallet mutation front-door + ron-ledger durable truth ==\n'

require_file crates/svc-wallet/tests/internal_roc_beta_paid_content_receipt_path.rs
require_file crates/svc-wallet/tests/internal_roc_beta_phase2_paid_action_idempotency.rs
require_file crates/svc-wallet/tests/internal_roc_beta_phase2_receipt_lookup_after_replay.rs
require_file crates/svc-wallet/tests/internal_roc_beta_phase3_approved_payout_execution_boundary.rs
require_file crates/ron-ledger/tests/internal_roc_beta_paid_content_replay_label.rs
require_file crates/ron-ledger/tests/internal_roc_beta_phase2_replay_conservation.rs
require_file crates/ron-ledger/tests/internal_roc_beta_phase3_approved_payout_replay.rs

run "$CARGO" fmt -p svc-wallet -- --check
run "$CARGO" fmt -p ron-ledger -- --check

run "$CARGO" test -p svc-wallet --test internal_roc_beta_paid_content_receipt_path
run "$CARGO" test -p svc-wallet --test internal_roc_beta_phase2_paid_action_idempotency
run "$CARGO" test -p svc-wallet --test internal_roc_beta_phase2_receipt_lookup_after_replay
run "$CARGO" test -p svc-wallet --test internal_roc_beta_phase3_approved_payout_execution_boundary

run "$CARGO" test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_paid_content_replay_label
run "$CARGO" test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase2_replay_conservation
run "$CARGO" test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_approved_payout_replay

printf '\n== Internal ROC Beta Phase 6 wallet/ledger proof target passed ==\n'
printf '== svc-wallet remains mutation front-door; ron-ledger remains durable receipt/balance truth ==\n'
