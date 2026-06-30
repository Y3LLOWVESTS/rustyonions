#!/usr/bin/env bash
# RO:WHAT — Internal ROC Beta Phase 5 focused preflight for ron-ledger economics config non-authority.
# RO:WHY — Proves configs/roc-economics.toml and config refs cannot create receipt, balance, supply, or finality truth.
# RO:INTERACTS — quickchain/types.rs and tests/internal_roc_beta_phase5_economics_config_non_authority.rs.
# RO:INVARIANTS — svc-wallet receipt evidence only; config refs reject atomically; bridge/staking inert.
# RO:METRICS — prints cargo output only.
# RO:CONFIG — requires configs/roc-economics.toml and ron-ledger quickchain-preflight feature.
# RO:SECURITY — test runner only; no secrets, network, bridge, staking, liquidity, or external settlement.
# RO:TEST — bash crates/ron-ledger/scripts/dev-internal-roc-beta-phase5-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-ledger internal ROC beta phase5 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f configs/roc-economics.toml ] || fail "missing configs/roc-economics.toml"
[ -f crates/ron-ledger/tests/internal_roc_beta_phase5_economics_config_non_authority.rs ] || fail "missing Phase 5 economics config non-authority test"

if ! grep -q 'roc_economics_config:' crates/ron-ledger/src/quickchain/types.rs; then
  fail "ron-ledger receipt evidence guard must reject economics config references"
fi

cargo fmt -p ron-ledger -- --check
cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase5_economics_config_non_authority
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight --no-deps -- -D warnings

printf '\n== Internal ROC Beta Phase 5 ron-ledger economics config non-authority preflight passed ==\n'
printf '== configs and economics refs cannot become receipt, balance, supply, or finality truth ==\n'
