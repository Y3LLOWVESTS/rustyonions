#!/usr/bin/env bash
# RO:WHAT — Focused ron-proto Internal ROC Stabilization receipt DTO preflight.
# RO:WHY — Product beta readiness needs receipt DTO identity/money/status fields to stay strict and non-authoritative.
# RO:INVARIANTS — DTO-only; operation_id durable backend identity; idempotency_key retry metadata; integer minor-unit strings only.
# RO:SECURITY — no receipt/balance/finality fabrication, no wallet/ledger mutation, no bridge/ROX/Solana/staking/liquidity/external settlement.
# RO:TEST — bash crates/ron-proto/scripts/dev-internal-roc-stabilization-receipt-dto-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p ron-proto -- --check
cargo test -p ron-proto --test internal_roc_stabilization_receipt_dto_boundary
cargo test -p ron-proto --test internal_roc_beta_paid_content_dto
cargo test -p ron-proto --test quickchain_receipt_dto
cargo test -p ron-proto --test quickchain_ids_and_money
cargo clippy -p ron-proto --all-targets --no-deps -- -D warnings

echo "== ron-proto Internal ROC Stabilization receipt DTO preflight passed =="
echo "== receipt DTOs strict/display-only; operation_id durable; idempotency_key retry-only; integer money only =="
