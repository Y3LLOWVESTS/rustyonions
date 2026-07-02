#!/usr/bin/env bash
# RO:WHAT — Aggregate svc-storage + svc-index Internal ROC Stabilization park gate.
# RO:WHY — Product beta readiness needs b3 storage and pointer lookup parked together before final aggregate sweep.
# RO:INVARIANTS — storage stores bytes; index stores pointers; neither creates receipt/balance/unlock/finality truth.
# RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, direct wallet/ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash scripts/dev-internal-roc-stabilization-storage-index-park.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Internal ROC Stabilization svc-storage + svc-index park: shell syntax =="
bash -n crates/svc-storage/scripts/dev-internal-roc-stabilization-b3-artifact-preflight.sh
bash -n crates/svc-index/scripts/dev-internal-roc-stabilization-pointer-preflight.sh
bash -n scripts/dev-internal-roc-stabilization-storage-index-park.sh

echo "== Internal ROC Stabilization svc-storage + svc-index park: focused preflights =="
bash crates/svc-storage/scripts/dev-internal-roc-stabilization-b3-artifact-preflight.sh
bash crates/svc-index/scripts/dev-internal-roc-stabilization-pointer-preflight.sh

echo "== svc-storage + svc-index Internal ROC Stabilization park gate passed =="
echo "== storage b3/artifact/admission boundary locked; index pointer/lookup boundary locked; backend receipt/access truth preserved =="
echo "== no fake receipt, fake balance, fake finality, paid unlock truth, cache-only unlock, direct wallet/ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement introduced =="
