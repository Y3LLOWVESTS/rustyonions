#!/usr/bin/env bash
# RO:WHAT — Exhaustive local ron-accounting QuickChain Phase-0 preflight gate.
# RO:WHY — Proves accounting remains derivative metering/snapshot infrastructure, not balance truth.
# RO:INTERACTS — ron-accounting docs, tests, WAL feature tests, cargo, bash tooling.
# RO:INVARIANTS — no balance mutation, no wallet/ledger mutation, no roots, checkpoints, validators, settlement, anchors, bridges, staking, liquidity, or pruning.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — rejects hidden helper drift and keeps preflight bash/cargo-only.
# RO:TEST — run from workspace root or through this script path.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-accounting"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

echo "== ron-accounting QuickChain Phase-0 exhaustive preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== forbidden scope marker =="
echo "ron-accounting QuickChain Phase-0 forbids: balance truth, wallet mutation, ledger mutation, roots, validators, settlement, external anchors, bridges, staking, liquidity, pruning, and live chain authority"
echo

echo "== tooling boundary: no checked-in Python helpers under ron-accounting =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "ron-accounting QuickChain preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== docs presence =="
DOC="$CRATE_DIR/docs/quickchain-preflight.md"
test -s "$DOC"
grep -q "Accounting is not balance truth" "$DOC"
grep -q "Handoff to svc-rewarder" "$DOC"
grep -q "no roots" "$DOC"
grep -q "no checkpoints" "$DOC"
grep -q "no validators" "$DOC"
grep -q "no settlement" "$DOC"
grep -q "no fake balances" "$DOC"
grep -q "no fake receipts" "$DOC"
grep -q "no wallet mutation" "$DOC"
grep -q "no ledger mutation" "$DOC"
echo "crate-local QuickChain runbook exists and preserves accounting boundaries"
echo

echo "== fmt check =="
"$CARGO" fmt -p ron-accounting -- --check
echo

echo "== clippy strict gate =="
"$CARGO" clippy -p ron-accounting --all-targets -- -D warnings
echo

echo "== all-target tests =="
"$CARGO" test -p ron-accounting --all-targets
echo

echo "== WAL feature all-target tests =="
"$CARGO" test -p ron-accounting --all-targets --features wal
echo

echo "== discover focused QuickChain tests =="
quickchain_tests="$(find "$CRATE_DIR/tests" \
  -maxdepth 1 \
  -type f \
  -name 'quickchain*.rs' \
  -exec basename {} .rs \; \
  | sort)"

quickchain_count="$(printf '%s\n' "$quickchain_tests" | sed '/^$/d' | wc -l | tr -d ' ')"

if [ "$quickchain_count" -lt 8 ]; then
  echo "$quickchain_tests"
  echo "expected at least 8 ron-accounting QuickChain test targets, found $quickchain_count"
  exit 1
fi

if ! printf '%s\n' "$quickchain_tests" | grep -qx "quickchain_preflight_docs"; then
  echo "$quickchain_tests"
  echo "quickchain_preflight_docs must be part of the discovered QuickChain test matrix"
  exit 1
fi

printf '%s\n' "$quickchain_tests"
echo "discovered ron-accounting QuickChain tests: $quickchain_count"
echo

echo "== run discovered QuickChain tests =="
for test_name in $quickchain_tests; do
  echo "-- $CARGO test -p ron-accounting --test $test_name"
  "$CARGO" test -p ron-accounting --test "$test_name"
done
echo

echo "== ron-accounting quickchain exhaustive preflight gate passed: tests=$quickchain_count =="
