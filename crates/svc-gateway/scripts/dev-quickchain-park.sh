#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

test -f "$CRATE_DIR/docs/quickchain-preflight.md"
test -f "$CRATE_DIR/scripts/dev-quickchain-preflight.sh"
test -f "$CRATE_DIR/tests/quickchain_tooling_boundary.rs"

"$CRATE_DIR/scripts/dev-quickchain-preflight.sh"

echo "== svc-gateway QuickChain parking gate passed =="
