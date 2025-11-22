#!/usr/bin/env bash
# RO:WHAT — Render Mermaid diagrams for macronode docs.
# RO:WHY  — Keep architecture docs (graphs/flows) in sync and easy to regenerate.
# RO:INVARIANTS —
#   - Operates only inside this crate (crates/macronode/docs).
#   - No-op if docs/ or any *.mmd files do not exist.
#   - Requires mermaid-cli (mmdc) if you want actual renders.

set -euo pipefail

CRATE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOCS_DIR="${CRATE_ROOT}/docs"

say() { printf '[macronode] %s\n' "$*"; }
warn() { printf '[macronode][warn] %s\n' "$*" >&2; }

if [[ ! -d "${DOCS_DIR}" ]]; then
  warn "docs/ directory not found at ${DOCS_DIR} — nothing to render."
  exit 0
fi

if ! command -v mmdc >/dev/null 2>&1; then
  warn "mermaid-cli (mmdc) not found."
  warn "Install via: npm install -g @mermaid-js/mermaid-cli"
  exit 1
fi

shopt -s nullglob
files=( "${DOCS_DIR}"/*.mmd )
if (( ${#files[@]} == 0 )); then
  warn "No *.mmd files found under ${DOCS_DIR} — nothing to render."
  exit 0
fi

for src in "${files[@]}"; do
  out="${src%.mmd}.svg"
  say "rendering ${src} -> ${out}"
  mmdc -i "${src}" -o "${out}"
done

say "All Mermaid diagrams rendered."
