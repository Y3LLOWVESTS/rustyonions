#!/usr/bin/env bash
# RO:WHAT — Focused svc-storage Internal ROC Stabilization b3 artifact non-authority preflight.
# RO:WHY — Product beta readiness needs storage to remain bytes/artifacts/admission/metering only.
# RO:INVARIANTS — b3 proves bytes only; svc-wallet mutates; ron-ledger records truth; accounting export is metering only.
# RO:SECURITY — no direct wallet/ledger mutation, fake receipt/balance/finality, cache-only unlock, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash crates/svc-storage/scripts/dev-internal-roc-stabilization-b3-artifact-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p svc-storage -- --check
cargo test -p svc-storage --test internal_roc_stabilization_b3_artifact_non_authority_boundary
cargo test -p svc-storage --test internal_roc_beta_paid_content_artifact_boundary
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --test paid_write_estimate
cargo test -p svc-storage --test paid_write_verifier
cargo test -p svc-storage --test paid_write_settlement
cargo test -p svc-storage --test paid_write_wallet_mode
cargo test -p svc-storage --test paid_write_accounting_export
cargo test -p svc-storage --test web3_paid_storage_loop
cargo test -p svc-storage --test quickchain_preflight_b3_integrity
cargo test -p svc-storage --test quickchain_preflight_paid_cache
cargo test -p svc-storage --test quickchain_preflight_no_direct_mutation
cargo test -p svc-storage --test quickchain_preflight_value_loop_boundary
cargo clippy -p svc-storage --all-targets -- -D warnings

echo "== svc-storage Internal ROC Stabilization b3 artifact preflight passed =="
echo "== b3 byte truth, paid admission evidence, wallet capture/release handoff, and accounting metering non-authority boundaries locked =="
