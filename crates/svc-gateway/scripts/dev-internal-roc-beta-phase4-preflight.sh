#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 4 Round 2 svc-gateway preflight.
# RO:WHY — Parks gateway confirmation/failure UX boundary without rerunning unrelated crates.
# RO:INTERACTS — svc-gateway product routes, proxy headers, Phase 4 tests.
# RO:INVARIANTS — gateway remains proxy/admission boundary only; no wallet/ledger mutation; no fake receipt/balance/finality; no cache/header-only unlock.
# RO:SECURITY — no bridge/staking/liquidity/ROX/Solana/external settlement.
# RO:TEST — crates/svc-gateway/scripts/dev-internal-roc-beta-phase4-preflight.sh.

set -euo pipefail

cargo fmt -p svc-gateway -- --check
cargo test -p svc-gateway --test internal_roc_beta_phase4_confirmation_failure_boundary
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings

echo "== Internal ROC Beta Phase 4 svc-gateway confirmation/failure preflight passed =="
echo "== gateway preserves display-safe quotes, redacted/source-labeled errors, no protected-body denial leaks, and no wallet/ledger/finality/cache entitlement authority =="
