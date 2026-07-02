#!/usr/bin/env bash
# RO:WHAT — Focused svc-index Internal ROC Stabilization pointer non-authority preflight.
# RO:WHY — Product beta readiness needs asset/site/manifest/provider pointers to remain lookup/navigation metadata only.
# RO:INVARIANTS — index points only; owner metadata is reference-only; backend wallet/ledger/gateway/omnigate enforce paid access.
# RO:SECURITY — no receipt/balance/finality/paid-unlock/wallet/ledger/bridge/staking/liquidity/external-settlement authority.
# RO:TEST — bash crates/svc-index/scripts/dev-internal-roc-stabilization-pointer-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p svc-index -- --check
cargo test -p svc-index --test internal_roc_stabilization_pointer_non_authority_boundary
cargo test -p svc-index --test internal_roc_beta_paid_content_pointer_boundary
cargo test -p svc-index --test quickchain_preflight_pointer_authority
cargo test -p svc-index --test quickchain_preflight_routes
cargo test -p svc-index --test quickchain_preflight_boundary
cargo test -p svc-index --test quickchain_preflight_value_loop_boundary
cargo test -p svc-index --test http_contract
cargo test -p svc-index --test integration
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings

echo "== svc-index Internal ROC Stabilization pointer preflight passed =="
echo "== asset/site manifest pointers, provider lookups, owner references, and lookup non-authority boundaries locked =="
