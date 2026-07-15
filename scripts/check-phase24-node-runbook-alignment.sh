#!/usr/bin/env bash
set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1
  pwd
)"

GUIDE="docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md"

MICRO="$ROOT/crates/micronode/docs/RUNBOOK.MD"
MACRO="$ROOT/crates/macronode/docs/RUNBOOK.MD"
ADMIN="$ROOT/crates/svc-admin/docs/RUNBOOK.MD"
CHECKLIST="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md"

fail() {
  printf 'Phase 24B runbook alignment check failed: %s\n' "$*" >&2
  exit 1
}

for file in "$MICRO" "$MACRO" "$ADMIN" "$CHECKLIST"; do
  [[ -f "$file" ]] || fail "missing ${file#$ROOT/}"
done

for file in "$MICRO" "$MACRO" "$ADMIN"; do
  count="$(
    grep -Foc \
      "## BUILD_PLAN_Z Phase 24 private-beta posture" \
      "$file" || true
  )"

  [[ "$count" -eq 1 ]] ||
    fail "${file#$ROOT/} must contain exactly one Phase 24 posture"

  grep -Fq "$GUIDE" "$file" ||
    fail "${file#$ROOT/} does not reference the authoritative guide"
done

micro_truth=(
  "The CrabLink desktop owns the User Node lifecycle."
  "loopback-only"
  "Direct \`serve\` use does not convert the User Node into a public Service Node."
  "Pending evidence is not a reward."
  "Confirmed ROC appears only after backend-derived wallet and ledger receipt"
)

for truth in "${micro_truth[@]}"; do
  grep -Fq "$truth" "$MICRO" ||
    fail "micronode runbook is missing: $truth"
done

macro_truth=(
  "The current \`macronode\` product role is the **CrabLink Service Node**."
  "crab://node/<node-id>"
  "A CLI-only deployment is valid."
  "A single Service Node cannot mint ROC or execute an epoch payout."
  "Unvetted content remains ephemeral by default."
  "The optional admin surface remains disabled by default and loopback-only"
)

for truth in "${macro_truth[@]}"; do
  grep -Fq "$truth" "$MACRO" ||
    fail "macronode runbook is missing: $truth"
done

admin_truth=(
  "\`svc-admin\` is an **optional local Service Node operator UI**."
  "disabled by default"
  "The Service Node continues running when \`svc-admin\` is unavailable."
  "There is no centralized password-reset flow."
  "Local admin credentials are not wallet credentials"
  "display pending evidence as confirmed ROC"
)

for truth in "${admin_truth[@]}"; do
  grep -Fq "$truth" "$ADMIN" ||
    fail "svc-admin runbook is missing: $truth"
done

grep -Fq \
  "[x] User Node, Service Node, and optional admin runbooks defer to the authoritative private-beta posture" \
  "$CHECKLIST" ||
  fail "central readiness checklist does not record runbook alignment"

printf '%s\n' \
  "Phase 24B node runbook alignment check passed." \
  "Micronode now defers to the CrabLink User Node posture;" \
  "macronode now defers to the headless Service Node posture;" \
  "svc-admin now defers to the optional loopback-only UI posture."
