#!/usr/bin/env bash
set -euo pipefail
echo "Rendering mermaid (requires mmdc) ..."
for f in "$(dirname "$0")"/../docs/*.mmd; do
  [ -f "$f" ] || continue
  out="${f%.mmd}.svg"
  mmdc -i "$f" -o "$out"
  echo "Rendered: $out"
done
