#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

echo "== svc-storage QuickChain exhaustive preflight =="
echo "repo: $ROOT"

test -f crates/svc-storage/docs/quickchain-preflight.md \
  || fail "missing crates/svc-storage/docs/quickchain-preflight.md"

echo
echo "== forbidden helper scan =="
python_hits=0
while IFS= read -r path; do
  echo "forbidden Python helper under svc-storage: $path" >&2
  python_hits=1
done < <(find crates/svc-storage \
  -path '*/target/*' -prune -o \
  -type f \( -name '*.py' -o -name '*.pyc' \) -print)

if [ "$python_hits" -ne 0 ]; then
  fail "QuickChain preflight tooling must stay bash/cargo-only"
fi
echo "no checked-in Python helpers found under crates/svc-storage"

echo
echo "== format check =="
if ! cargo fmt -p svc-storage -- --check; then
  echo
  echo "cargo fmt --check failed."
  echo "Run:"
  echo "  cargo fmt -p svc-storage"
  echo "then rerun:"
  echo "  crates/svc-storage/scripts/dev-quickchain-preflight.sh"
  echo
  exit 1
fi

echo
echo "== discover focused QuickChain tests =="
QUICKCHAIN_TESTS=()
while IFS= read -r test_name; do
  QUICKCHAIN_TESTS+=("$test_name")
done < <(
  find crates/svc-storage/tests \
    -maxdepth 1 \
    -type f \
    -name 'quickchain*.rs' \
    -exec basename {} .rs \; \
  | sort
)

if [ "${#QUICKCHAIN_TESTS[@]}" -eq 0 ]; then
  fail "no QuickChain test targets discovered under crates/svc-storage/tests"
fi

printf 'discovered %s focused QuickChain tests:\n' "${#QUICKCHAIN_TESTS[@]}"
for test_name in "${QUICKCHAIN_TESTS[@]}"; do
  printf '  - %s\n' "$test_name"
done

echo
echo "== focused QuickChain preflight tests =="
for test_name in "${QUICKCHAIN_TESTS[@]}"; do
  echo
  echo "---- cargo test -p svc-storage --test $test_name ----"
  cargo test -p svc-storage --test "$test_name"
done

echo
echo "== svc-storage all-targets test =="
cargo test -p svc-storage --all-targets

echo
echo "== svc-storage clippy =="
cargo clippy -p svc-storage --all-targets -- -D warnings

echo
echo "== svc-storage forbidden-scope marker =="
echo "no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity"
echo "svc-storage remains bytes by canonical b3 only; cache is not paid-access authority; wallet/ledger truth stays backend-owned"

echo
echo "== svc-storage quickchain exhaustive preflight gate passed: tests=${#QUICKCHAIN_TESTS[@]} =="
