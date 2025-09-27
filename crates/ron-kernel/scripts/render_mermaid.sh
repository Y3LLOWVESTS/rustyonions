#!/usr/bin/env bash
set -euo pipefail
which mmdc >/dev/null || npm i -g @mermaid-js/mermaid-cli
for f in $(git ls-files 'crates/ron-kernel2/docs/*.mmd' 2>/dev/null); do
  mmdc -i "$f" -o "${f%.mmd}.svg"
done
