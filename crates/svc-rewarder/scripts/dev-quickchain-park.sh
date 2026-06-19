#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

echo "== svc-rewarder QuickChain parking gate =="

test -f crates/svc-rewarder/docs/quickchain-preflight.md
test -f crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
test -f crates/svc-rewarder/tests/quickchain_tooling_boundary.rs

crates/svc-rewarder/scripts/dev-quickchain-preflight.sh

echo
echo "== svc-rewarder QuickChain parking gate passed =="
