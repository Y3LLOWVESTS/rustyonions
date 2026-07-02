#!/usr/bin/env bash
# RO:WHAT — Focused omnigate Internal ROC Stabilization paid-access error-boundary preflight.
# RO:WHY — Product beta readiness needs omnigate paid access routes to source-label failures and deny protected render without backend truth.
# RO:INVARIANTS — no fake receipt/balance/finality, no cache-only unlock, no wallet/ledger mutation, no bridge/ROX/Solana/staking/liquidity/external settlement.
# RO:TEST — bash crates/omnigate/scripts/dev-internal-roc-stabilization-paid-access-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo test -p omnigate --test internal_roc_stabilization_paid_access_error_boundary
cargo test -p omnigate --test internal_roc_beta_paid_content_access_boundary
cargo test -p omnigate --test content_view
cargo test -p omnigate --test site_visit

echo "== omnigate Internal ROC Stabilization paid access error-boundary preflight passed =="
echo "== source-labeled paid access errors; quote read-only; pay via svc-wallet only; protected body stays locked on denial =="
