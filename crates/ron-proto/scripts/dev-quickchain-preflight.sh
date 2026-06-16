#!/usr/bin/env bash
# RO:WHAT — Bash-only exhaustive QuickChain preflight gate for ron-proto.
# RO:WHY — ECON/GOV/RES: every ron-proto QuickChain DTO/vector/canonicalization test must run before claiming vector preflight green.
# RO:INTERACTS — ron-proto tests/quickchain_*.rs and tests/tools/verify_quickchain_*.sh.
# RO:INVARIANTS — discovers all quickchain_*.rs tests; runs bash vector verifiers; no roots, validators, settlement, anchors, bridges, or runtime authority are produced.
# RO:METRICS — prints discovered test count and cargo output only.
# RO:CONFIG — run from repo root or from anywhere inside the repository.
# RO:SECURITY — test runner only; no secrets, no network, no wallet mutation, no ledger mutation, no service authority.
# RO:TEST — bash crates/ron-proto/scripts/dev-quickchain-preflight.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
cd "${repo_root}"

fail() {
  printf 'ron-proto quickchain preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f crates/ron-proto/Cargo.toml ] || fail "repo root detection failed"
[ -d crates/ron-proto/tests ] || fail "missing crates/ron-proto/tests"
[ -d crates/ron-proto/tests/tools ] || fail "missing crates/ron-proto/tests/tools"

if find crates/ron-proto -type f -name '*.py' | grep -q .; then
  find crates/ron-proto -type f -name '*.py' >&2
  fail "ron-proto QuickChain tooling must stay bash-only"
fi

cargo fmt -p ron-proto -- --check

test_count=0

while IFS= read -r test_path; do
  test_name="${test_path##*/}"
  test_name="${test_name%.rs}"

  printf '\n[ron-proto quickchain] running %s\n' "${test_name}"
  cargo test -p ron-proto --test "${test_name}"

  test_count=$((test_count + 1))
done < <(find crates/ron-proto/tests -maxdepth 1 -type f -name 'quickchain_*.rs' | sort)

[ "${test_count}" -ge 20 ] || fail "expected at least 20 quickchain integration tests, discovered ${test_count}"

printf '\n[ron-proto quickchain] bash vector inventory verifier\n'
bash crates/ron-proto/tests/tools/verify_quickchain_vector_inventory.sh "${repo_root}"

printf '\n[ron-proto quickchain] bash hash payload verifier\n'
bash crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.sh "${repo_root}"

printf '\n[ron-proto quickchain] clippy preflight\n'
cargo clippy -p ron-proto --all-targets -- -D warnings

printf '\nron-proto quickchain exhaustive preflight gate passed: tests=%s\n' "${test_count}"
