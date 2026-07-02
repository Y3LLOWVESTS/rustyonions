#!/usr/bin/env bash
# RO:WHAT — Aggregate svc-gateway + omnigate Internal ROC Stabilization paid-route/access park gate.
# RO:WHY — Runs the backend public/enforcement pair together after CrabLink Tauri paid-access stabilization was parked.
# RO:INVARIANTS — gateway/omnigate remain routing/hydration/access coordinators; svc-wallet mutates; ron-ledger records truth.
# RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, protected body leakage, gateway/omnigate wallet/ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash scripts/dev-internal-roc-stabilization-gateway-omnigate-paid-route-park.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Internal ROC Stabilization backend paid route/access park: shell syntax =="
bash -n crates/svc-gateway/scripts/dev-internal-roc-stabilization-paid-route-preflight.sh
bash -n crates/omnigate/scripts/dev-internal-roc-stabilization-paid-access-preflight.sh
bash -n scripts/dev-internal-roc-stabilization-gateway-omnigate-paid-route-park.sh

echo "== Internal ROC Stabilization backend paid route/access park: focused tests =="
bash crates/svc-gateway/scripts/dev-internal-roc-stabilization-paid-route-preflight.sh
bash crates/omnigate/scripts/dev-internal-roc-stabilization-paid-access-preflight.sh

echo "== Internal ROC Stabilization backend paid route/access park: clippy =="
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings

echo "== svc-gateway + omnigate Internal ROC Stabilization paid route/access park gate passed =="
echo "== source-labeled/redacted errors; no protected leakage; proxy/hydration only; wallet/ledger truth preserved =="
echo "== no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, gateway/omnigate direct wallet/ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement introduced =="
