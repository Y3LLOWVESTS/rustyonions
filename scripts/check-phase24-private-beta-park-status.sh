#!/usr/bin/env bash
set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1
  pwd
)"

CHECKLIST="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md"
STATUS="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md"
PARK="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md"
RUNBOOK="$ROOT/docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md"

fail() {
  printf 'Phase 24 park-status check failed: %s\n' "$*" >&2
  exit 1
}

for file in "$CHECKLIST" "$STATUS" "$PARK" "$RUNBOOK"; do
  [[ -f "$file" ]] || fail "missing ${file#$ROOT/}"
done

grep -Fq \
  "[x] final Phase 24 private-beta acceptance sweep is green" \
  "$CHECKLIST" ||
  fail "final acceptance checkbox is not complete"

grep -Fq \
  "BUILD_PLAN_Z Phase 24 — Private Beta Readiness is now:" \
  "$STATUS" ||
  fail "status record does not declare Phase 24 completion"

grep -Fq \
  "CrabLink Node Layer / BUILD_PLAN_Z Phase 24 —" \
  "$PARK" ||
  fail "park record does not contain the Phase 24 safe label"

for file in "$STATUS" "$PARK" "$RUNBOOK"; do
  grep -Fq "COMPLETE / GREEN / PARKED" "$file" ||
    fail "${file#$ROOT/} does not contain the parked completion label"

  grep -Fq "PHASE24_FINAL_STATUS=GREEN_PARKED" "$file" ||
    fail "${file#$ROOT/} does not contain the reproducible final marker"
done

printf '%s\n' \
  "Phase 24 private-beta park-status check passed." \
  "Documentation, checklist, and final status consistently declare" \
  "BUILD_PLAN_Z Phase 24 COMPLETE / GREEN / PARKED."
