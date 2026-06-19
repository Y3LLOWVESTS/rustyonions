#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Help/list-only placeholder for a future QuickChain Phase-0 umbrella runner.
# RO:WHY — Records the intended crate order without triggering disk-heavy all-crate rebuilds.
# RO:INTERACTS — crate-local QuickChain preflight/park scripts; future low-disk runner work.
# RO:INVARIANTS — no full workspace test sweep by default; no roots/checkpoints/validators/bridges.
# RO:SECURITY — does not mutate services, wallets, ledgers, receipts, or external settlement state.
# RO:TEST — bash -n scripts/dev-quickchain-phase0.sh; scripts/dev-quickchain-phase0.sh --list.

CRATES=(
  "ron-proto"
  "ron-ledger"
  "svc-wallet"
  "ron-accounting"
  "svc-rewarder"
  "svc-storage"
  "svc-gateway"
  "omnigate"
  "svc-index"
  "ron-policy"
  "CrabLink Tauri/client adapters"
)

print_help() {
  cat <<'HELP'
QuickChain Phase-0 umbrella runner placeholder

Status:
  Deferred / help-only / list-only.

Why:
  Full all-crate QuickChain validation is intentionally not run from this script yet.
  The current dev machine has disk/time constraints, and each crate already has focused
  preflight / park gates. This file records the intended order without causing rebuild churn.

Allowed now:
  --help
  --list
  --print-crate-order

Forbidden/default behavior:
  This script does not run all crate tests.
  This script does not run all crate clippy gates.
  This script does not produce roots, checkpoints, validators, settlement, bridges, ROX,
  Solana integration, staking, liquidity, or public chain behavior.

Future low-disk design:
  per-crate isolated CARGO_TARGET_DIR
  CARGO_INCREMENTAL=0
  RUSTFLAGS="-C debuginfo=0"
  clean per-crate target after success
  fail fast with crate-specific failure output
HELP
}

print_crates() {
  local index=1
  for crate in "${CRATES[@]}"; do
    printf '%02d. %s\n' "$index" "$crate"
    index=$((index + 1))
  done
}

case "${1:-}" in
  ""|"-h"|"--help")
    print_help
    ;;
  "--list"|"--print-crate-order")
    print_crates
    ;;
  *)
    cat >&2 <<'ERR'
QuickChain Phase-0 umbrella execution is intentionally deferred.

Use:
  scripts/dev-quickchain-phase0.sh --help
  scripts/dev-quickchain-phase0.sh --list

Run crate-local preflight/park scripts instead of triggering a full workspace sweep.
ERR
    exit 2
    ;;
esac
