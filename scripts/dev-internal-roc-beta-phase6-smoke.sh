#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="full"
for arg in "$@"; do
  case "$arg" in
    --core-only|--rust-only)
      MODE="core"
      ;;
    --full)
      MODE="full"
      ;;
    --help|-h)
      cat <<'USAGE'
Usage:
  bash scripts/dev-internal-roc-beta-phase6-smoke.sh [--full|--core-only]

Default:
  --full       Run RustyOnions core/access checks plus CrabLink Tauri Phase 6 checker.

Options:
  --core-only  Run RustyOnions Rust checks only. This is useful for early Batch 0/1 validation.
USAGE
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$arg" >&2
      exit 1
      ;;
  esac
done

printf '== Internal ROC Beta Phase 6 reproducible smoke suite ==\n'
printf '== mode: %s ==\n' "$MODE"
printf '== safe scope: internal ROC only; no bridge/staking/liquidity/external settlement ==\n'

export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export RUSTFLAGS="${RUSTFLAGS:--C debuginfo=0}"

require_file() {
  if [[ ! -f "$1" ]]; then
    printf 'missing required Phase 6 inventory file: %s\n' "$1" >&2
    exit 1
  fi
}

run_script() {
  local script="$1"
  require_file "$script"
  printf '\n== bash %s ==\n' "$script"
  bash "$script"
}

require_file docs/internal-roc-beta/PHASE6_REPRODUCIBLE_BETA_SMOKE.md
require_file configs/roc-economics.toml
require_file scripts/internal-roc-beta-wallet-ledger-check.sh
require_file scripts/internal-roc-beta-replay-check.sh
require_file scripts/internal-roc-beta-tokenomics-check.sh
require_file scripts/internal-roc-beta-access-check.sh
require_file scripts/internal-roc-beta-crablink-check.sh

printf '\n== shell syntax checks ==\n'
bash -n scripts/internal-roc-beta-wallet-ledger-check.sh
bash -n scripts/internal-roc-beta-replay-check.sh
bash -n scripts/internal-roc-beta-tokenomics-check.sh
bash -n scripts/internal-roc-beta-access-check.sh
bash -n scripts/internal-roc-beta-crablink-check.sh
bash -n scripts/internal-roc-beta-smoke.sh
bash -n scripts/dev-internal-roc-beta-phase6-smoke.sh

run_script scripts/internal-roc-beta-wallet-ledger-check.sh
run_script scripts/internal-roc-beta-replay-check.sh
run_script scripts/internal-roc-beta-tokenomics-check.sh
run_script scripts/internal-roc-beta-access-check.sh

if [[ "$MODE" == "full" ]]; then
  run_script scripts/internal-roc-beta-crablink-check.sh
  printf '\nInternal ROC Beta Phase 6 reproducible smoke suite complete.\n'
  printf 'Phase 6 status: COMPLETE / GREEN / PARKED candidate, pending notes/terminal review.\n'
else
  printf '\nInternal ROC Beta Phase 6 Batch 0/1 core smoke gate passed.\n'
  printf 'Phase 6 full completion still requires CrabLink Tauri proof and final notes.\n'
fi

printf '\nStill deferred: ROX, Solana, bridge, staking runtime, liquidity, exchange-facing logic, public validator economy, external settlement.\n'
