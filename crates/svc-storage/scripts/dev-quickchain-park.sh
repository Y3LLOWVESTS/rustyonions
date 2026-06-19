#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

echo "== svc-storage QuickChain parking gate =="

test -f crates/svc-storage/docs/quickchain-preflight.md
test -f crates/svc-storage/scripts/dev-quickchain-preflight.sh
test -f crates/svc-storage/tests/quickchain_tooling_boundary.rs

crates/svc-storage/scripts/dev-quickchain-preflight.sh

echo
echo "== svc-storage QuickChain parking gate passed =="
