#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Live CrabLink/RustyOnions Internal ROC probe runner.
# RO:WHY — Makes manual hardening probes repeatable while stack is running.
# RO:INTERACTS — scripts/probes/internal-roc-live-*.sh.
# RO:INVARIANTS — default run is non-mutating; mutating idempotency requires RUN_MUTATING=1.
# RO:TEST — bash scripts/dev-internal-roc-beta-live-probes.sh with CRAB_URL and account env.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_probe() {
  local script="$1"
  printf '\n== bash %s ==\n' "$script"
  bash "$ROOT/$script"
}

: "${CRAB_URL:?set CRAB_URL=crab://<64hex>.image}"
: "${VISITOR_ACCOUNT:?set VISITOR_ACCOUNT}"
: "${CREATOR_ACCOUNT:?set CREATOR_ACCOUNT}"

run_probe scripts/probes/internal-roc-live-readonly.sh
run_probe scripts/probes/internal-roc-live-negative-content-view.sh

PAYER_ACCOUNT="${PAYER_ACCOUNT:-$VISITOR_ACCOUNT}" \
VIEWER_PASSPORT="${VIEWER_PASSPORT:-passport:main:probe}" \
run_probe scripts/probes/internal-roc-live-quote-tamper.sh

if [[ "${RUN_MUTATING:-0}" = "1" ]]; then
  PAYER_ACCOUNT="${PAYER_ACCOUNT:-$VISITOR_ACCOUNT}" \
  VIEWER_PASSPORT="${VIEWER_PASSPORT:-passport:main:probe}" \
  run_probe scripts/probes/internal-roc-live-idempotency.sh
else
  printf '\n== mutating idempotency probe skipped ==\n'
  printf 'Set RUN_MUTATING=1 to spend exactly AMOUNT once and prove duplicate retry does not double-spend.\n'
fi

printf '\nInternal ROC Beta live probes passed.\n'
