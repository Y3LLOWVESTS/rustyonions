#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

echo "== svc-rewarder QuickChain exhaustive preflight =="
echo "repo: $ROOT"

test -f crates/svc-rewarder/docs/quickchain-preflight.md \
  || fail "missing crates/svc-rewarder/docs/quickchain-preflight.md"

echo
echo "== forbidden helper scan =="
python_hits=0
while IFS= read -r path; do
  echo "forbidden Python helper under svc-rewarder: $path" >&2
  python_hits=1
done < <(find crates/svc-rewarder \
  -path '*/target/*' -prune -o \
  -type f \( -name '*.py' -o -name '*.pyc' \) -print)

if [ "$python_hits" -ne 0 ]; then
  fail "QuickChain preflight tooling must stay bash/cargo-only"
fi
echo "no checked-in Python helpers found under crates/svc-rewarder"

echo
echo "== format check =="
if ! cargo fmt -p svc-rewarder -- --check; then
  echo
  echo "cargo fmt --check failed."
  echo "Run:"
  echo "  cargo fmt -p svc-rewarder"
  echo "then rerun:"
  echo "  crates/svc-rewarder/scripts/dev-quickchain-preflight.sh"
  echo
  exit 1
fi

echo
echo "== discover focused QuickChain tests =="
QUICKCHAIN_TESTS=()
while IFS= read -r test_name; do
  QUICKCHAIN_TESTS+=("$test_name")
done < <(
  find crates/svc-rewarder/tests \
    -maxdepth 1 \
    -type f \
    -name 'quickchain*.rs' \
    -exec basename {} .rs \; \
  | sort
)

if [ "${#QUICKCHAIN_TESTS[@]}" -eq 0 ]; then
  fail "no QuickChain test targets discovered under crates/svc-rewarder/tests"
fi

printf 'discovered %s focused QuickChain tests:\n' "${#QUICKCHAIN_TESTS[@]}"
for test_name in "${QUICKCHAIN_TESTS[@]}"; do
  printf '  - %s\n' "$test_name"
done

echo
echo "== focused QuickChain preflight tests =="
for test_name in "${QUICKCHAIN_TESTS[@]}"; do
  echo
  echo "---- cargo test -p svc-rewarder --test $test_name ----"
  cargo test -p svc-rewarder --test "$test_name"
done

echo
echo "== svc-rewarder all-targets test =="
cargo test -p svc-rewarder --all-targets

echo
echo "== svc-rewarder clippy =="
cargo clippy -p svc-rewarder --all-targets -- -D warnings

echo
echo "== svc-rewarder forbidden-scope marker =="
echo "no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity"
echo "svc-rewarder remains deterministic payout planning only; svc-wallet remains mutation front-door; ron-ledger remains truth"

echo
echo "== svc-rewarder quickchain exhaustive preflight gate passed: tests=${#QUICKCHAIN_TESTS[@]} =="
