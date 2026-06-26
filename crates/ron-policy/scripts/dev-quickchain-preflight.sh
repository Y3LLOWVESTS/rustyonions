#!/usr/bin/env bash
# RO:WHAT — Crate-local QuickChain Phase-0 preflight runner for ron-policy.
# RO:WHY — Keeps policy declarative and proves it is not wallet/ledger/root/settlement authority.
# RO:INTERACTS — docs, schema, quickchain*.rs tests, existing ron-policy unit/economics tests.
# RO:INVARIANTS — no roots/checkpoints/validators/settlement; no fake receipts/balances/unlocks.
# RO:TEST — run from repo root with `bash crates/ron-policy/scripts/dev-quickchain-preflight.sh [--check]`.
# quickchain_phase4_bond_dispute_boundary

set -euo pipefail

PKG="ron-policy"
CARGO="${CARGO:-cargo}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT="$(cd "$CRATE_ROOT/../.." && pwd)"
TEST_DIR="$CRATE_ROOT/tests"
DOC_PATH="$CRATE_ROOT/docs/quickchain-preflight.md"

mode="${1:-}"
if [[ -n "$mode" && "$mode" != "--check" ]]; then
  echo "usage: $0 [--check]" >&2
  exit 2
fi

cd "$ROOT"

echo "== ron-policy QuickChain Phase-0 preflight =="
echo "repo: $ROOT"
echo "crate: $CRATE_ROOT"

if [[ ! -f "$DOC_PATH" ]]; then
  echo "missing required docs file: $DOC_PATH" >&2
  exit 1
fi

if [[ ! -d "$TEST_DIR" ]]; then
  echo "missing required tests dir: $TEST_DIR" >&2
  exit 1
fi

python_hit="$(
  find "$CRATE_ROOT" \
    -path "$CRATE_ROOT/target" -prune -o \
    -type f \( -name '*.py' -o -name '*.pyc' \) -print -quit
)"
if [[ -n "$python_hit" ]]; then
  echo "forbidden Python helper drift under $PKG: $python_hit" >&2
  exit 1
fi

echo
echo "== format =="
if [[ "$mode" == "--check" ]]; then
  "$CARGO" fmt -p "$PKG" -- --check
else
  "$CARGO" fmt -p "$PKG"
fi

echo
echo "== focused QuickChain preflight tests =="
quickchain_count=0
while IFS= read -r test_file; do
  test_name="$(basename "$test_file" .rs)"
  quickchain_count=$((quickchain_count + 1))
  echo "[ron-policy quickchain] cargo test -p $PKG --test $test_name"
  "$CARGO" test -p "$PKG" --test "$test_name"
done < <(find "$TEST_DIR" -maxdepth 1 -type f -name 'quickchain*.rs' | sort)

if [[ "$quickchain_count" -eq 0 ]]; then
  echo "no quickchain*.rs tests found under $TEST_DIR" >&2
  exit 1
fi

echo
echo "== existing ron-policy regressions =="
for regression in \
  economics_policy \
  unit_model_serde_strict \
  unit_eval_determinism \
  unit_first_match_wins \
  golden_reasons
do
  if [[ -f "$TEST_DIR/$regression.rs" ]]; then
    echo "[ron-policy regression] cargo test -p $PKG --test $regression"
    "$CARGO" test -p "$PKG" --test "$regression"
  else
    echo "[ron-policy regression] skipped missing $TEST_DIR/$regression.rs"
  fi
done

echo
echo "== all targets =="
"$CARGO" test -p "$PKG" --all-targets

echo
echo "== clippy =="
"$CARGO" clippy -p "$PKG" --all-targets --no-deps -- -D warnings

echo
echo "ron-policy forbidden QuickChain runtime scope remains parked:"
echo "- no roots"
echo "- no checkpoints"
echo "- no validators"
echo "- no settlement"
echo "- no bridges"
echo "- no wallet or ledger mutation"
echo "- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions"
echo
echo "== ron-policy quickchain exhaustive preflight gate passed: tests=$quickchain_count =="
echo "ron-policy QuickChain Phase-0 preflight passed."
