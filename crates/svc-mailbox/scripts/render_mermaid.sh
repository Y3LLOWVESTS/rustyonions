#!/usr/bin/env bash
set -euo pipefail
for f in $(git ls-files "crates/svc-mailbox2/docs/diagrams/*.mmd" 2>/dev/null || true); do
  out="${f%.mmd}.svg"
  echo "render: $f -> $out"
  mmdc -i "$f" -o "$out"
done

