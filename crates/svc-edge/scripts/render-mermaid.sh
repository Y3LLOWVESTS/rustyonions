#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
for f in $(git ls-files 'docs/mmd/*.mmd'); do
  out=${f/mmd/svg}
  out=${out%.mmd}.svg
  mkdir -p $(dirname "$out")
  mmdc -i "$f" -o "$out"
done
