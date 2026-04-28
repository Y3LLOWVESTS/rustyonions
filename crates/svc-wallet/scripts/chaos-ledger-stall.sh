#!/usr/bin/env bash
# RO:WHAT — Placeholder chaos drill for simulating ledger stalls against svc-wallet.
# RO:WHY — Ops/RES; Concerns: RES/ECON. Wallet writes must shed/fail-closed when ledger truth is unavailable.
# RO:INTERACTS — future ronctl chaos, /readyz, /metrics.
# RO:INVARIANTS — writes degrade before collapse; reads may remain available when configured.
# RO:METRICS — inspect wallet_rejects_total and wallet_ready after the drill.
# RO:CONFIG — uses SVC_WALLET_ADDR if present.
# RO:SECURITY — no secrets echoed.
# RO:TEST — future chaos CI/manual runbook.

set -euo pipefail

ADDR="${SVC_WALLET_ADDR:-127.0.0.1:8088}"

echo "Simulated: ledger stall injection placeholder for svc-wallet"
echo "Check readiness:"
curl -fsS "http://${ADDR}/readyz"
echo
echo "Check wallet metrics:"
curl -fsS "http://${ADDR}/metrics" | grep -E 'wallet_ready|wallet_rejects_total|wallet_requests_total' || true