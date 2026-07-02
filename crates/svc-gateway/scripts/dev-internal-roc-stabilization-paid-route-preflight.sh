#!/usr/bin/env bash
# RO:WHAT — Focused svc-gateway Internal ROC Stabilization paid-route error-boundary preflight.
# RO:WHY — Product beta readiness needs gateway paid routes to stay proxy-only while errors are source-labeled and non-authoritative.
# RO:INVARIANTS — no fake receipt/balance/finality, no cache-only unlock, no wallet/ledger mutation, no bridge/ROX/Solana/staking/liquidity/external settlement.
# RO:TEST — bash crates/svc-gateway/scripts/dev-internal-roc-stabilization-paid-route-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo test -p svc-gateway --test internal_roc_stabilization_paid_route_error_boundary
cargo test -p svc-gateway --test internal_roc_beta_paid_content_route_boundary
cargo test -p svc-gateway --test content_view_routes_proxy
cargo test -p svc-gateway --test site_visit_routes_proxy

echo "== svc-gateway Internal ROC Stabilization paid route error-boundary preflight passed =="
echo "== proxy-only paid routes; source-labeled transport errors; no protected body leakage or gateway authority creep =="
