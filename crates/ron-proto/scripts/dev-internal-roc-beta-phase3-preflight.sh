#!/usr/bin/env bash
# RO:WHAT — Internal ROC Beta Phase 3 focused preflight for ron-proto accounting/reward-plan and approved-payout DTO references.
# RO:WHY — Proves Phase 3 planning and approved-payout receipt DTOs are strict references, not client/rewarder/policy/accounting authority.
# RO:INTERACTS — internal_roc_beta_phase3_accounting_reward_plan_dto.rs and internal_roc_beta_phase3_approved_payout_dto.rs.
# RO:INVARIANTS — DTO-only; integer money strings; no raw engagement minting; no fake receipts/balances/finality; no bridge/staking runtime.
# RO:METRICS — prints cargo output only.
# RO:CONFIG — run from repo root or anywhere under the repository.
# RO:SECURITY — test runner only; no secrets, network, wallet authority, or receipt fabrication.
# RO:TEST — bash crates/ron-proto/scripts/dev-internal-roc-beta-phase3-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-proto internal ROC beta phase3 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f crates/ron-proto/Cargo.toml ] || fail "repo root detection failed"
[ -f crates/ron-proto/src/quickchain/reward_plan.rs ] || fail "missing reward-plan DTO module"
[ -f crates/ron-proto/tests/internal_roc_beta_phase3_accounting_reward_plan_dto.rs ] || \
  fail "missing internal ROC phase3 accounting/reward-plan DTO test"
[ -f crates/ron-proto/tests/internal_roc_beta_phase3_approved_payout_dto.rs ] || \
  fail "missing internal ROC phase3 approved-payout DTO test"

if find crates/ron-proto -type f -name '*.py' | grep -q .; then
  find crates/ron-proto -type f -name '*.py' >&2
  fail "ron-proto internal ROC beta tooling must stay bash-only"
fi

cargo fmt -p ron-proto -- --check
cargo test -p ron-proto --test internal_roc_beta_phase3_accounting_reward_plan_dto
cargo test -p ron-proto --test internal_roc_beta_phase3_approved_payout_dto
cargo clippy -p ron-proto --all-targets --no-deps -- -D warnings

printf '\n== Internal ROC Beta Phase 3 ron-proto DTO preflight passed ==\n'
printf '== reward plans remain references and approved payout receipts remain backend wallet/ledger-derived DTOs, not client/rewarder/policy/accounting authority ==\n'
