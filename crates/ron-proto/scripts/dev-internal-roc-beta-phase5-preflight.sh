#!/usr/bin/env bash
# RO:WHAT — Internal ROC Beta Phase 5 focused preflight for ron-proto economics config DTOs.
# RO:WHY — Proves economics config DTOs are strict, integer-safe, inert data and not wallet/ledger authority.
# RO:INTERACTS — configs/roc-economics.toml and tests/internal_roc_beta_phase5_economics_config_dto.rs.
# RO:INVARIANTS — no floats; bps totals exact; bridge/staking inert; no receipt/balance/finality truth.
# RO:METRICS — prints cargo output only.
# RO:CONFIG — run from repo root or anywhere under the repository.
# RO:SECURITY — test runner only; no secrets, network, wallet authority, or receipt fabrication.
# RO:TEST — bash crates/ron-proto/scripts/dev-internal-roc-beta-phase5-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-proto internal ROC beta phase5 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f configs/roc-economics.toml ] || fail "missing configs/roc-economics.toml"
[ -f crates/ron-proto/src/econ/internal_roc_economics.rs ] || fail "missing internal ROC economics DTO module"
[ -f crates/ron-proto/tests/internal_roc_beta_phase5_economics_config_dto.rs ] || fail "missing Phase 5 economics config DTO test"

if grep -R --line-number -E 'f32|f64' crates/ron-proto/src/econ crates/ron-proto/tests/internal_roc_beta_phase5_economics_config_dto.rs; then
  fail "ron-proto economics config DTO/test must not use float types"
fi

cargo fmt -p ron-proto -- --check
cargo test -p ron-proto --test internal_roc_beta_phase5_economics_config_dto
cargo clippy -p ron-proto --all-targets --no-deps -- -D warnings

printf '\n== Internal ROC Beta Phase 5 ron-proto economics config DTO preflight passed ==\n'
printf '== economics config remains strict integer-safe DTO data, not wallet/ledger authority ==\n'
