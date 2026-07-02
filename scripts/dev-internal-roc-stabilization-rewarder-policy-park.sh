#!/usr/bin/env bash
# RO:WHAT — Aggregate svc-rewarder + ron-policy Internal ROC Stabilization park gate.
# RO:WHY — Product beta readiness needs capped reward planning and declarative policy gating parked together.
# RO:INVARIANTS — rewarder plans only; policy gates only; svc-wallet mutates; ron-ledger records truth.
# RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, direct ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Internal ROC Stabilization svc-rewarder + ron-policy park: shell syntax =="
bash -n crates/svc-rewarder/scripts/dev-internal-roc-stabilization-reward-policy-gate-preflight.sh
bash -n crates/ron-policy/scripts/dev-internal-roc-stabilization-policy-gate-preflight.sh
bash -n scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh

echo "== Internal ROC Stabilization svc-rewarder + ron-policy park: focused preflights =="
bash crates/svc-rewarder/scripts/dev-internal-roc-stabilization-reward-policy-gate-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-stabilization-policy-gate-preflight.sh

echo "== svc-rewarder + ron-policy Internal ROC Stabilization park gate passed =="
echo "== rewarder capped planning locked; policy declarative gate locked; svc-wallet/ron-ledger truth boundary preserved =="
echo "== no fake receipt, fake balance, fake finality, raw-engagement payout, direct ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement introduced =="
