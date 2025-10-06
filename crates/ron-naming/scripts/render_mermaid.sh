#!/usr/bin/env bash
set -euo pipefail

# Render Mermaid diagrams locally (optional).
# Requires 'mmdc' (mermaid-cli): npm install -g @mermaid-js/mermaid-cli
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/docs/diagrams/arch.mmd"
DST="$ROOT/docs/diagrams/arch.svg"

if command -v mmdc >/dev/null 2>&1; then
  mmdc -i "$SRC" -o "$DST"
  echo "Rendered $DST"
else
  echo "mermaid-cli (mmdc) not found; skipped render."
fi
