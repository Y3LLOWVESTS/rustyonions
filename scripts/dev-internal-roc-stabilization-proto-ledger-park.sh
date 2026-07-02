#!/usr/bin/env bash
# RO:WHAT — Aggregate ron-proto + ron-ledger Internal ROC Stabilization receipt/replay/conservation park gate.
# RO:WHY — Runs DTO truth and ledger truth pair together after CrabLink and gateway/omnigate stabilization.
# RO:INVARIANTS — ron-proto DTO-only; ron-ledger durable truth; svc-wallet mutation front-door; no new mutation path.
# RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, direct non-wallet ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash scripts/dev-internal-roc-stabilization-proto-ledger-park.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Internal ROC Stabilization ron-proto + ron-ledger park: shell syntax =="
bash -n crates/ron-proto/scripts/dev-internal-roc-stabilization-receipt-dto-preflight.sh
bash -n crates/ron-ledger/scripts/dev-internal-roc-stabilization-replay-truth-preflight.sh
bash -n scripts/dev-internal-roc-stabilization-proto-ledger-park.sh

echo "== Internal ROC Stabilization ron-proto + ron-ledger park: focused preflights =="
bash crates/ron-proto/scripts/dev-internal-roc-stabilization-receipt-dto-preflight.sh
bash crates/ron-ledger/scripts/dev-internal-roc-stabilization-replay-truth-preflight.sh

echo "== ron-proto + ron-ledger Internal ROC Stabilization receipt/replay/conservation park gate passed =="
echo "== DTO receipt truth strict; ledger accepted replay deterministic; conservation/retry boundaries locked =="
echo "== no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, direct non-wallet ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement introduced =="
