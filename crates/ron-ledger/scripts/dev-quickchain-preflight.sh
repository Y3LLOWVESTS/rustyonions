#!/usr/bin/env bash
# RO:WHAT — Bash-only exhaustive QuickChain preflight gate for ron-ledger.
# RO:WHY — ECON/RES/GOV: every gated ron-ledger QuickChain integration test must run before claiming preflight green.
# RO:INTERACTS — ron-ledger quickchain-preflight feature, ron-proto DTOs, ron-ledger tests/quickchain_*.rs.
# RO:INVARIANTS — discovers all quickchain_*.rs tests; local deterministic roots are allowed only in ron-ledger tree material projection; no validators, anchors, settlement, bridges, external chain logic, or runtime authority are produced.
# RO:METRICS — prints discovered test count and cargo output only.
# RO:CONFIG — run from repo root or from anywhere inside the repository.
# RO:SECURITY — test runner only; no secrets, no network, no wallet mutation, no service mutation, no ledger-service authority.
# RO:TEST — bash crates/ron-ledger/scripts/dev-quickchain-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-ledger quickchain preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f crates/ron-ledger/Cargo.toml ] || fail "repo root detection failed"
[ -d crates/ron-ledger/tests ] || fail "missing crates/ron-ledger/tests"

if find crates/ron-ledger -type f -name '*.py' | grep -q .; then
  find crates/ron-ledger -type f -name '*.py' >&2
  fail "ron-ledger preflight tooling must stay bash-only"
fi

cargo fmt -p ron-ledger -- --check

test_count=0

while IFS= read -r test_path; do
  test_name="${test_path##*/}"
  test_name="${test_name%.rs}"

  printf '\n[ron-ledger quickchain] running %s\n' "${test_name}"
  cargo test -p ron-ledger --features quickchain-preflight --test "${test_name}"

  test_count=$((test_count + 1))
done < <(find crates/ron-ledger/tests -maxdepth 1 -type f -name 'quickchain_*.rs' | sort)

[ "${test_count}" -ge 20 ] || fail "expected at least 20 quickchain integration tests, discovered ${test_count}"

printf '\n[ron-ledger quickchain] clippy preflight\n'
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings

printf '\nron-ledger quickchain exhaustive preflight gate passed: tests=%s\n' "${test_count}"
