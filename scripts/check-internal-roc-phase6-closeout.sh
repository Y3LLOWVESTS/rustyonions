#!/usr/bin/env bash
set -euo pipefail

FILE="docs/internal-roc-beta/PHASE6_CLOSEOUT.md"

test -f "$FILE"

needles=(
  "Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED."
  "Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness."
  "Bridge work: docs / threat-model / decision-gate only."
  "No ROX/Solana/bridge/staking/liquidity/external settlement runtime."
  "svc-wallet remains the only internal ROC mutation front-door"
  "ron-ledger remains durable economic truth"
  "CrabLink remains display/user-intent only"
)

for needle in "${needles[@]}"; do
  if ! grep -Fq "$needle" "$FILE"; then
    echo "missing closeout phrase: $needle" >&2
    exit 1
  fi
done

docs=(
  "docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md"
  "docs/roadmap/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md"
  "docs/roadmap/INTERNAL_ROC_BETA_BUILDPLAN.md"
)

for doc in "${docs[@]}"; do
  if [ ! -f "$doc" ]; then
    echo "missing expected internal ROC doc: $doc" >&2
    exit 1
  fi

  if ! grep -Fq "INTERNAL-ROC-PHASE6-SAFE-LABEL" "$doc"; then
    echo "missing safe label in $doc" >&2
    exit 1
  fi
done

echo "Internal ROC Phase 6 closeout docs check passed."
