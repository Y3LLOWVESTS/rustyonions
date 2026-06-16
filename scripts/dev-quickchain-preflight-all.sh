#!/usr/bin/env bash
# RO:WHAT — Compatibility wrapper for the workspace QuickChain Phase-0 gate.
# RO:WHY — Keeps the older/preferred naming variants from drifting.
# RO:INTERACTS — scripts/dev-quickchain-phase0.sh.
# RO:INVARIANTS — delegates only; does not add roots/checkpoints/validators/settlement logic.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$SCRIPT_DIR/dev-quickchain-phase0.sh" "$@"
