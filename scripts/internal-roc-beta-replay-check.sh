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
    printf 'missing required Phase 6 replay proof file: %s\n' "$1" >&2
    exit 1
  fi
}

printf '== Internal ROC Beta Phase 6 replay/conservation proof target ==\n'

require_file crates/ron-ledger/tests/internal_roc_beta_phase2_replay_conservation.rs
require_file crates/ron-ledger/tests/internal_roc_beta_phase3_approved_payout_replay.rs
require_file crates/ron-accounting/tests/internal_roc_beta_phase2_snapshot_cannot_change_replay.rs
require_file crates/svc-wallet/tests/internal_roc_beta_phase2_receipt_lookup_after_replay.rs

run "$CARGO" fmt -p ron-ledger -- --check
run "$CARGO" fmt -p ron-accounting -- --check
run "$CARGO" fmt -p svc-wallet -- --check

run "$CARGO" test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase2_replay_conservation
run "$CARGO" test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_approved_payout_replay
run "$CARGO" test -p ron-accounting --test internal_roc_beta_phase2_snapshot_cannot_change_replay
run "$CARGO" test -p svc-wallet --test internal_roc_beta_phase2_receipt_lookup_after_replay

printf '\n== Internal ROC Beta Phase 6 replay/conservation proof target passed ==\n'
printf '== accepted replay stays deterministic; accounting remains derivative; receipt lookup does not fabricate truth ==\n'
