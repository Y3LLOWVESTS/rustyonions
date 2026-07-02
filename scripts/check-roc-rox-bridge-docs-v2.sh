#!/usr/bin/env bash
set -euo pipefail

files=(
  "docs/roadmap/POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md"
  "docs/blueprints/ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md"
  "docs/buildplans/ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md"
  "docs/threat-models/ROC_ROX_BRIDGE_THREAT_MODEL.md"
)

for file in "${files[@]}"; do
  test -f "$file"
done

must_have=(
  "DOCS / THREAT-MODEL ONLY"
  "No ROX/Solana/bridge/staking/liquidity/external settlement runtime."
  "cross-domain nonce"
  "multi-RPC"
  "cluster binding"
  "program id binding"
  "mint binding"
  "halt"
  "pending finalizations"
  "stuck challenge"
  "recovery account"
  "CPI"
  "upgrade authority"
  "verifiable build"
  "forbidden UI"
  "NO-VALUE-BEARING-DEVNET-PHASE9-GATE"
  "No value-bearing devnet"
  "Phase 9 audit/recovery drills"
)

for needle in "${must_have[@]}"; do
  if ! grep -RFiq "$needle" "${files[@]}"; then
    echo "missing bridge v2 phrase: $needle" >&2
    exit 1
  fi
done

forbidden_positive_phrases=(
  "This file approves bridge runtime"
  "Bridge runtime authorized by this file"
  "ROX live token launch authorized"
  "Solana deployment authorized"
  "liquidity runtime authorized"
  "staking runtime authorized"
)

for phrase in "${forbidden_positive_phrases[@]}"; do
  if grep -RFiq "$phrase" "${files[@]}"; then
    echo "forbidden positive runtime authorization phrase found: $phrase" >&2
    exit 1
  fi
done

echo "ROC/ROX bridge docs v2 boundary check passed."
