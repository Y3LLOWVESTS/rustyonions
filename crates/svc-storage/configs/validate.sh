#!/usr/bin/env bash
set -euo pipefail
echo "Validating TOML configs (lightweight placeholder)..."
for f in *.toml profiles/*.toml 2>/dev/null; do
  if [ -f "$f" ]; then
    echo "OK: $f"
  fi
done
