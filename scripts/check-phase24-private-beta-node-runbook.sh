#!/usr/bin/env bash
set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1
  pwd
)"

DOC="$ROOT/docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md"
STATUS="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md"
CHECKLIST="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md"
PARK="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md"

fail() {
  printf 'Phase 24A documentation check failed: %s\n' "$*" >&2
  exit 1
}

for file in "$DOC" "$STATUS" "$CHECKLIST" "$PARK"; do
  [[ -f "$file" ]] || fail "missing ${file#$ROOT/}"
done

required_headings=(
  "## 1. Status and scope"
  "## 2. Two-node model"
  "## 3. CrabLink User Node UX guide"
  "## 4. Service Node operator quickstart"
  "## 5. crabnode CLI guide"
  "## 6. Optional admin UI guide"
  "## 7. First-run setup guide"
  "## 8. Reward @ address binding guide"
  "## 9. ROC mining and reward explanation"
  "## 10. Service-node quorum explanation"
  "## 11. IP privacy explanation"
  "## 12. Moderation, pruning, and policy operations"
  "## 13. Persistence policy guide"
  "## 14. Policy and denylist operations"
  "## 15. Incident response guide"
  "## 16. Known limitations"
  "## 17. Private-beta acceptance proof"
)

for heading in "${required_headings[@]}"; do
  grep -Fqx "$heading" "$DOC" ||
    fail "missing required heading: $heading"
done

required_truth=(
  "There is no third public Economic Node product."
  "A single Service Node cannot execute a reward transition."
  "Confirmed ROC may appear only after CrabLink receives backend-derived receipt"
  "admin UI is optional and local-only"
  "unvetted content is amnesia-first"
  "only ledger receipts confirm ROC"
  'crab://node/<node-id>'
  "There is no central password recovery."
  "Phase 23 final acceptance: GREEN / PARKED"
)

for truth in "${required_truth[@]}"; do
  grep -Fq "$truth" "$DOC" ||
    fail "missing required private-beta truth: $truth"
done

for legacy_scheme in 'relay://' 'onion://' 'service://'; do
  count="$(grep -Foc "$legacy_scheme" "$DOC" || true)"

  [[ "$count" -eq 1 ]] ||
    fail "$legacy_scheme must appear exactly once in the explicit rejection list"
done

for central in "$STATUS" "$CHECKLIST" "$PARK"; do
  grep -Fq "PRIVATE_BETA_NODE_RUNBOOK.md" "$central" ||
    fail "${central#$ROOT/} does not reference the private-beta node runbook"
done

printf '%s\n' \
  "Phase 24A private-beta node documentation check passed." \
  "Two-node product posture, user UX, headless operator workflow," \
  "reward binding, ROC/quorum truth, privacy, moderation, persistence," \
  "incident recovery, limitations, and acceptance evidence are present."
