#!/usr/bin/env bash
# RO:WHAT — Internal ROC Beta Phase 2 focused preflight for ron-proto DTO replay-shape proof.
# RO:WHY — Proves existing operation-history DTOs are enough for replay/conservation work without new authority.
# RO:INTERACTS — crates/ron-proto/tests/internal_roc_beta_phase2_operation_history_dto.rs.
# RO:INVARIANTS — DTO-only; no ledger mutation; no bridge/staking/ROX/Solana/external settlement.
# RO:METRICS — prints cargo output only.
# RO:CONFIG — run from repo root or anywhere under the repository.
# RO:SECURITY — test runner only; no secrets, network, wallet authority, or receipt fabrication.
# RO:TEST — bash crates/ron-proto/scripts/dev-internal-roc-beta-phase2-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-proto internal ROC beta phase2 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f crates/ron-proto/Cargo.toml ] || fail "repo root detection failed"
[ -f crates/ron-proto/tests/internal_roc_beta_phase2_operation_history_dto.rs ] || \
  fail "missing internal ROC phase2 DTO test"

if find crates/ron-proto -type f -name '*.py' | grep -q .; then
  find crates/ron-proto -type f -name '*.py' >&2
  fail "ron-proto internal ROC beta tooling must stay bash-only"
fi

cargo fmt -p ron-proto -- --check
cargo test -p ron-proto --test internal_roc_beta_phase2_operation_history_dto
cargo clippy -p ron-proto --all-targets -- -D warnings

printf '\n== Internal ROC Beta Phase 2 ron-proto DTO replay-shape preflight passed ==\n'
printf '== existing operation DTOs cover paid transfers and hold lifecycle without new authority ==\n'
