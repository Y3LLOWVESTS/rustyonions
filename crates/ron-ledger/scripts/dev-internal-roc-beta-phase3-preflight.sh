#!/usr/bin/env bash
# RO:WHAT — Internal ROC Beta Phase 3 focused preflight for ron-ledger reward-plan non-authority and approved-payout replay/receipt boundaries.
# RO:WHY — Proves reward plans/events/snapshots cannot be committed as ledger truth, while approved payout issues replay only as accepted wallet/ledger evidence.
# RO:INTERACTS — internal_roc_beta_phase3_reward_plan_non_authority.rs and internal_roc_beta_phase3_approved_payout_replay.rs.
# RO:INVARIANTS — svc-wallet remains mutation front-door; ron-ledger remains durable truth; no direct accounting/rewarder/policy ledger mutation.
# RO:METRICS — prints cargo output only.
# RO:CONFIG — run from repo root or anywhere under the repository.
# RO:SECURITY — test runner only; no secrets, network, bridge, staking, or external settlement.
# RO:TEST — bash crates/ron-ledger/scripts/dev-internal-roc-beta-phase3-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-ledger internal ROC beta phase3 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f crates/ron-ledger/Cargo.toml ] || fail "repo root detection failed"
[ -f crates/ron-ledger/tests/internal_roc_beta_phase3_reward_plan_non_authority.rs ] || \
  fail "missing internal ROC phase3 reward-plan non-authority test"
[ -f crates/ron-ledger/tests/internal_roc_beta_phase3_approved_payout_replay.rs ] || \
  fail "missing internal ROC phase3 approved-payout replay test"

if find crates/ron-ledger -type f -name '*.py' | grep -q .; then
  find crates/ron-ledger -type f -name '*.py' >&2
  fail "ron-ledger internal ROC beta tooling must stay bash-only"
fi

cargo fmt -p ron-ledger -- --check
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_reward_plan_non_authority
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_approved_payout_replay
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase2_replay_conservation
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings

printf '\n== Internal ROC Beta Phase 3 ron-ledger payout/reward preflight passed ==\n'
printf '== reward plans cannot become ledger receipt/balance truth; approved payouts replay only as durable wallet/ledger accepted issue receipts with duplicate prevention ==\n'
