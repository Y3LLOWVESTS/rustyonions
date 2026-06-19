#!/usr/bin/env bash
# RO:WHAT — Parking gate wrapper for ron-policy QuickChain Phase-0.
# RO:WHY — Provides one stable command proving ron-policy remains declarative policy only.
# RO:INTERACTS — dev-quickchain-preflight.sh, docs/quickchain-preflight.md, quickchain_tooling_boundary.rs.
# RO:INVARIANTS — delegates to preflight; does not create roots/checkpoints/validators/settlement behavior.
# RO:TEST — run `bash crates/ron-policy/scripts/dev-quickchain-park.sh [--check]`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

mode="${1:-}"
if [[ -n "$mode" && "$mode" != "--check" ]]; then
  echo "usage: $0 [--check]" >&2
  exit 2
fi

required=(
  "$CRATE_ROOT/docs/quickchain-preflight.md"
  "$CRATE_ROOT/scripts/dev-quickchain-preflight.sh"
  "$CRATE_ROOT/tests/quickchain_tooling_boundary.rs"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing parking requirement: $path" >&2
    exit 1
  fi
done

if [[ -n "$mode" ]]; then
  "$SCRIPT_DIR/dev-quickchain-preflight.sh" "$mode"
else
  "$SCRIPT_DIR/dev-quickchain-preflight.sh"
fi

echo "== ron-policy QuickChain parking gate passed =="
