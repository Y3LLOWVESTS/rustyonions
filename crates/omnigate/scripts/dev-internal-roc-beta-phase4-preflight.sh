#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 4 Round 2 omnigate preflight.
# RO:WHY — Parks omnigate confirmation/failure UX boundary without rerunning unrelated crates.
# RO:INTERACTS — content_view, site_visit, paid object routes, Phase 4 tests.
# RO:INVARIANTS — omnigate remains quote/access/hydration coordinator only; no ledger mutation; no fake receipt/balance/finality; no cache/policy/manifest/b3-only unlock.
# RO:SECURITY — no bridge/staking/liquidity/ROX/Solana/external settlement.
# RO:TEST — crates/omnigate/scripts/dev-internal-roc-beta-phase4-preflight.sh.

set -euo pipefail

cargo fmt -p omnigate -- --check
cargo test -p omnigate --test internal_roc_beta_phase4_confirmation_failure_boundary
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings

echo "== Internal ROC Beta Phase 4 omnigate confirmation/failure preflight passed =="
echo "== omnigate preserves display-safe quotes, redacted/source-labeled errors, no protected-body denial leaks, and no wallet/ledger/finality/cache entitlement authority =="
