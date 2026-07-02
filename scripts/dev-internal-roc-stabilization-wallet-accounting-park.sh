#!/usr/bin/env bash
# RO:WHAT — Aggregate svc-wallet + ron-accounting Internal ROC Stabilization park gate.
# RO:WHY — Product beta readiness needs mutation front-door and derivative snapshot/report boundaries parked together.
# RO:INVARIANTS — svc-wallet mutates through ron-ledger; ron-accounting observes only; no new ledger mutation path.
# RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, accounting/rewarder/policy/client mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash scripts/dev-internal-roc-stabilization-wallet-accounting-park.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Internal ROC Stabilization svc-wallet + ron-accounting park: shell syntax =="
bash -n crates/svc-wallet/scripts/dev-internal-roc-stabilization-mutation-frontdoor-preflight.sh
bash -n crates/ron-accounting/scripts/dev-internal-roc-stabilization-snapshot-non-authority-preflight.sh
bash -n scripts/dev-internal-roc-stabilization-wallet-accounting-park.sh

echo "== Internal ROC Stabilization svc-wallet + ron-accounting park: focused preflights =="
bash crates/svc-wallet/scripts/dev-internal-roc-stabilization-mutation-frontdoor-preflight.sh
bash crates/ron-accounting/scripts/dev-internal-roc-stabilization-snapshot-non-authority-preflight.sh

echo "== svc-wallet + ron-accounting Internal ROC Stabilization park gate passed =="
echo "== wallet mutation front-door locked; accounting snapshot/report non-authority locked; backend receipt/balance truth preserved =="
echo "== no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, accounting/rewarder/policy/client mutation, bridge, ROX/Solana, staking, liquidity, or external settlement introduced =="
