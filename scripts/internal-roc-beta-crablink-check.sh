#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRABLINK_ROOT="${CRABLINK_ROOT:-$ROOT/../crablink}"
NPM="${NPM:-npm}"

run() {
  printf '\n== %s ==\n' "$*"
  "$@"
}

printf '== Internal ROC Beta Phase 6 CrabLink Tauri proof target ==\n'

if [[ ! -d "$CRABLINK_ROOT/apps/crablink-tauri" ]]; then
  printf 'CrabLink root not found at: %s\n' "$CRABLINK_ROOT" >&2
  printf 'Set CRABLINK_ROOT=/absolute/path/to/crablink and rerun.\n' >&2
  exit 1
fi

if [[ ! -f "$CRABLINK_ROOT/apps/crablink-tauri/package.json" ]]; then
  printf 'missing CrabLink app package: %s/apps/crablink-tauri/package.json\n' "$CRABLINK_ROOT" >&2
  exit 1
fi

if [[ ! -f "$CRABLINK_ROOT/scripts/check-internal-roc-beta.mjs" ]]; then
  printf 'missing CrabLink Phase 6 checker: %s/scripts/check-internal-roc-beta.mjs\n' "$CRABLINK_ROOT" >&2
  exit 1
fi

run node --check "$CRABLINK_ROOT/scripts/check-internal-roc-beta.mjs"
run "$NPM" --prefix "$CRABLINK_ROOT/apps/crablink-tauri" run check:internal-roc-beta

printf '\n== Internal ROC Beta Phase 6 CrabLink Tauri proof target passed ==\n'
printf '== CrabLink remains display/user intent only with backend-derived receipts/balances/access ==\n'
