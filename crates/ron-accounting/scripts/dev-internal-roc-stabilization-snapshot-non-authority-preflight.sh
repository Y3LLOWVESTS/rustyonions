#!/usr/bin/env bash
# RO:WHAT — Focused ron-accounting Internal ROC Stabilization snapshot non-authority preflight.
# RO:WHY — Product beta readiness needs accounting snapshots/reports to remain deterministic derivative artifacts only.
# RO:INVARIANTS — accounting observes accepted receipts only; no wallet/ledger mutation; no receipt/balance/payout/finality truth.
# RO:SECURITY — no fake receipt/balance/finality, paid unlock, payout execution, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash crates/ron-accounting/scripts/dev-internal-roc-stabilization-snapshot-non-authority-preflight.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

cargo fmt -p ron-accounting -- --check
cargo test -p ron-accounting --test internal_roc_stabilization_snapshot_non_authority_boundary
cargo test -p ron-accounting --test internal_roc_beta_paid_content_snapshot_non_authority
cargo test -p ron-accounting --test internal_roc_beta_phase2_snapshot_cannot_change_replay
cargo test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary
cargo test -p ron-accounting --test internal_roc_beta_phase3_approved_payout_observation_boundary
cargo test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority
cargo test -p ron-accounting --test internal_roc_beta_phase5_event_class_antifarming
cargo clippy -p ron-accounting --all-targets --no-deps -- -D warnings

echo "== ron-accounting Internal ROC Stabilization snapshot non-authority preflight passed =="
echo "== derivative snapshots, accepted receipt observations, anti-farming lanes, and non-authority boundaries locked =="
