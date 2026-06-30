#!/usr/bin/env bash
# RO:WHAT — Internal ROC Beta Phase 2 focused preflight for ron-ledger replay/conservation core.
# RO:WHY — Proves deterministic accepted replay, ROC conservation, hold terminality, and ordering boundaries.
# RO:INTERACTS — crates/ron-ledger/tests/internal_roc_beta_phase2_replay_conservation.rs.
# RO:INVARIANTS — no new mutation path; ledger truth only; no bridge/staking/ROX/Solana/external settlement.
# RO:METRICS — prints cargo output only.
# RO:CONFIG — run from repo root or anywhere under the repository; requires quickchain-preflight feature.
# RO:SECURITY — test runner only; no secrets, network, wallet authority, or receipt fabrication.
# RO:TEST — bash crates/ron-ledger/scripts/dev-internal-roc-beta-phase2-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-ledger internal ROC beta phase2 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f crates/ron-ledger/Cargo.toml ] || fail "repo root detection failed"
[ -f crates/ron-ledger/tests/internal_roc_beta_phase2_replay_conservation.rs ] || \
  fail "missing internal ROC phase2 replay/conservation test"

if find crates/ron-ledger -type f -name '*.py' | grep -q .; then
  find crates/ron-ledger -type f -name '*.py' >&2
  fail "ron-ledger internal ROC beta tooling must stay bash-only"
fi

cargo fmt -p ron-ledger -- --check
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase2_replay_conservation
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings

printf '\n== Internal ROC Beta Phase 2 ron-ledger replay/conservation preflight passed ==\n'
printf '== replay equality, conservation, hold terminality, safe retries, and order-boundary proof complete for this crate ==\n'
