#!/usr/bin/env bash
set -euo pipefail
# Renders docs/*.mmd to SVG using mermaid-cli if available.
ROOT=${1:-"./crates"}
cd "./crates/ryker"
if ! command -v mmdc >/dev/null 2>&1; then
  echo "mmdc not found. Install: npm i -g @mermaid-js/mermaid-cli"
  exit 1
fi
for f in docs/*.mmd; do
  [ -e "$f" ] || continue
  out=${f%.mmd}.svg
  mmdc -i "$f" -o "$out"
  echo "rendered: $out"
done

