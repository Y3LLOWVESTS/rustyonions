#!/usr/bin/env bash
# RO:WHAT — Focused ron-ledger Internal ROC Stabilization replay/conservation truth preflight.
# RO:WHY — Product beta readiness needs accepted replay, receipt evidence, retries, and conservation boundaries locked.
# RO:INVARIANTS — ledger truth only; svc-wallet mutation front-door; accepted replay not root/proof/finality/settlement.
# RO:SECURITY — no fake receipt/balance/finality, no silent spend/cache-only unlock, no bridge/ROX/Solana/staking/liquidity/external settlement.
# RO:TEST — bash crates/ron-ledger/scripts/dev-internal-roc-stabilization-replay-truth-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p ron-ledger -- --check
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_stabilization_replay_truth_boundary
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase2_replay_conservation
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_approved_payout_replay
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_paid_content_replay_label
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_accepted_replay_boundary_retry_stability
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_accepted_replay_boundary_rejection_stability
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_accepted_replay_boundary_identity_rejection
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight --no-deps -- -D warnings

echo "== ron-ledger Internal ROC Stabilization replay/conservation truth preflight passed =="
echo "== accepted replay deterministic; retries/rejections stable; receipt evidence preserved; conservation locked =="
