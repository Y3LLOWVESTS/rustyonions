#!/usr/bin/env bash
# RO:WHAT — Exhaustive local svc-wallet gate including inert QuickChain preflight compatibility.
# RO:WHY — Keeps wallet as mutation front-door while verifying every wallet QuickChain boundary test.
# RO:INTERACTS — svc-wallet, ron-ledger quickchain-preflight feature, cargo, bash tooling.
# RO:INVARIANTS — no roots, validators, settlement, external anchors, bridges, staking, liquidity, or live chain authority.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — QuickChain remains feature-gated and disabled by default.
# RO:TEST — run from workspace root or through this script path.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/svc-wallet"

cd "$ROOT_DIR"

echo "== svc-wallet QuickChain Phase-0 exhaustive preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== forbidden scope marker =="
echo "svc-wallet QuickChain Phase-0 forbids: roots, validators, settlement, external anchors, bridges, staking, liquidity, and live chain authority"
echo

echo "== tooling boundary: no checked-in *.py helpers under svc-wallet =="
python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  echo "svc-wallet QuickChain preflight must remain bash/cargo-only"
  exit 1
fi
echo "tooling boundary clean"
echo

echo "== fmt check =="
cargo fmt -p svc-wallet -- --check
echo

echo "== normal strict clippy gate =="
cargo clippy -p svc-wallet --all-targets -- -D warnings
echo

echo "== normal all-target tests =="
cargo test -p svc-wallet --all-targets
echo

echo "== discover focused QuickChain tests =="
quickchain_tests="$(find "$CRATE_DIR/tests" \
  -maxdepth 1 \
  -type f \
  -name 'quickchain*.rs' \
  -exec basename {} .rs \; | sort)"

quickchain_count="$(printf '%s\n' "$quickchain_tests" | sed '/^$/d' | wc -l | tr -d ' ')"

if [ "$quickchain_count" -lt 11 ]; then
  echo "$quickchain_tests"
  echo "expected at least 11 svc-wallet QuickChain test targets, found $quickchain_count"
  exit 1
fi

required_quickchain_tests=(
  quickchain_phase1_receipt_root_material_interlock
  quickchain_preflight_accounting_observer_boundary
  quickchain_preflight_boundary
  quickchain_preflight_docs
  quickchain_preflight_idempotency_identity_boundary
  quickchain_preflight_live_route_matrix
  quickchain_preflight_no_runtime_authority
  quickchain_preflight_phase1_pair_interlock
  quickchain_preflight_projection_validation_matrix
  quickchain_preflight_request_poisoning_matrix
  quickchain_tooling_boundary
)

for required_test in "${required_quickchain_tests[@]}"; do
  if ! printf '%s\n' "$quickchain_tests" | grep -qx "$required_test"; then
    echo "$quickchain_tests"
    echo "missing required svc-wallet QuickChain test target: $required_test"
    exit 1
  fi
done

printf '%s\n' "$quickchain_tests"
echo "discovered svc-wallet QuickChain tests: $quickchain_count"
echo

echo "== run discovered QuickChain tests with feature gate =="
for test_name in $quickchain_tests; do
  echo "-- cargo test -p svc-wallet --features quickchain-preflight --test $test_name"
  cargo test -p svc-wallet --features quickchain-preflight --test "$test_name"
done
echo

echo "== feature-gated strict clippy gate =="
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
echo

echo "== feature-gated all-target tests =="
cargo test -p svc-wallet --all-targets --features quickchain-preflight
echo

echo "== svc-wallet quickchain exhaustive preflight gate passed: tests=$quickchain_count =="
