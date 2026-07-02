#!/usr/bin/env bash
# RO:WHAT — Focused svc-wallet Internal ROC Stabilization mutation front-door preflight.
# RO:WHY — Product beta readiness needs wallet idempotency, receipts, approved payouts, and paid flows locked to svc-wallet/ron-ledger truth.
# RO:INVARIANTS — svc-wallet mutation front-door only; ron-ledger truth; accepted-only receipts; idempotency retry safety.
# RO:SECURITY — no fake receipt/balance/finality, silent spend, cache-only unlock, bypass mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash crates/svc-wallet/scripts/dev-internal-roc-stabilization-mutation-frontdoor-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p svc-wallet -- --check
cargo test -p svc-wallet --test internal_roc_stabilization_mutation_frontdoor_boundary
cargo test -p svc-wallet --test internal_roc_beta_paid_content_receipt_path
cargo test -p svc-wallet --test internal_roc_beta_phase2_paid_action_idempotency
cargo test -p svc-wallet --test internal_roc_beta_phase2_receipt_lookup_after_replay
cargo test -p svc-wallet --test internal_roc_beta_phase3_approved_payout_execution_boundary
cargo test -p svc-wallet --test internal_roc_beta_phase3_accounting_observer_boundary
cargo test -p svc-wallet --test internal_roc_beta_phase5_config_non_authority
cargo clippy -p svc-wallet --all-targets --no-deps -- -D warnings

echo "== svc-wallet Internal ROC Stabilization mutation front-door preflight passed =="
echo "== idempotency, backend receipts, approved payouts, paid flow, and accounting observation boundaries locked =="
