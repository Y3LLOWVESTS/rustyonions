#!/usr/bin/env bash
set -euo pipefail

CARGO="${CARGO:-cargo}"
PKG="svc-gateway"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$CRATE_DIR/../.." && pwd)"

cd "$REPO_ROOT"

DOC="$CRATE_DIR/docs/quickchain-preflight.md"
TEST_DIR="$CRATE_DIR/tests"

if [ ! -f "$DOC" ]; then
  echo "missing QuickChain preflight doc: $DOC" >&2
  exit 1
fi

if [ ! -d "$TEST_DIR" ]; then
  echo "missing test directory: $TEST_DIR" >&2
  exit 1
fi

python_helpers="$(find "$CRATE_DIR" -type f -name '*.py' -print)"
if [ -n "$python_helpers" ]; then
  echo "unexpected Python helper drift under $CRATE_DIR:" >&2
  echo "$python_helpers" >&2
  exit 1
fi

echo "[svc-gateway quickchain] fmt check"
"$CARGO" fmt -p "$PKG" -- --check

# Known focused preflight suites include:
# quickchain_preflight_boundary
# quickchain_preflight_docs
# quickchain_preflight_no_fake_receipts
# quickchain_preflight_cache_boundary
# quickchain_preflight_paid_access
# quickchain_preflight_transport_authority
# quickchain_tooling_boundary
# quickchain_preflight_value_loop_boundary
echo "[svc-gateway quickchain] dynamic quickchain test discovery"
test_count=0
while IFS= read -r test_path; do
  test_name="$(basename "$test_path" .rs)"
  test_count=$((test_count + 1))
  echo "[svc-gateway quickchain] cargo test -p $PKG --test $test_name"
  "$CARGO" test -p "$PKG" --test "$test_name"
done < <(find "$TEST_DIR" -type f -name 'quickchain*.rs' -print | sort)

if [ "$test_count" -eq 0 ]; then
  echo "no quickchain*.rs tests discovered under $TEST_DIR" >&2
  exit 1
fi

echo "[svc-gateway quickchain] product/proxy regressions"
for regression in \
  app_proxy \
  paid_storage_estimate_proxy \
  paid_storage_write_proxy \
  product_routes_proxy \
  site_visit_routes_proxy
do
  if [ -f "$TEST_DIR/$regression.rs" ]; then
    "$CARGO" test -p "$PKG" --test "$regression"
  fi
done

echo "[svc-gateway quickchain] all-targets"
"$CARGO" test -p "$PKG" --all-targets

echo "[svc-gateway quickchain] clippy"
"$CARGO" clippy -p "$PKG" --all-targets --no-deps -- -D warnings

echo "[svc-gateway quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances"
echo "svc-gateway QuickChain preflight gate passed"
echo "== svc-gateway quickchain exhaustive preflight gate passed: tests=$test_count =="
