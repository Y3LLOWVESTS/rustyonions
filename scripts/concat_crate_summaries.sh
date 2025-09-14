#!/usr/bin/env bash
# Concatenate all crate summary markdown files into one big file with a TOC.
# Works on macOS Bash 3.2 (no readarray/mapfile).

set -euo pipefail

# ---- Resolve source/output ----
# If $1 is given, that's the source dir. Otherwise try docs/crate-summaries, then crate-summaries.
if [[ $# -ge 1 && -n "${1:-}" ]]; then
  SRC_DIR="$1"
else
  if [[ -d "docs/crate-summaries" ]]; then
    SRC_DIR="docs/crate-summaries"
  elif [[ -d "crate-summaries" ]]; then
    SRC_DIR="crate-summaries"
  else
    echo "Source dir not found: docs/crate-summaries or crate-summaries" >&2
    exit 1
  fi
fi

# $2 (optional) is output path; default is inside SRC_DIR
OUT="${2:-$SRC_DIR/ALL_SUMMARIES.md}"
STAMP="$(date -u '+%Y-%m-%d %H:%M:%SZ')"

# Safety: ensure source dir exists
if [[ ! -d "$SRC_DIR" ]]; then
  echo "Source dir not found: $SRC_DIR" >&2
  exit 1
fi

# ---- Build header + TOC ----
{
  echo "<!-- Generated: $STAMP -->"
  echo "# Crate Summaries (Combined)"
  echo
  echo "This file is generated from markdown files in \`$SRC_DIR\`."
  echo
  echo "## Table of Contents"
} > "$OUT"

# First pass: TOC (alphabetical). Skip template and the output file itself.
find "$SRC_DIR" -maxdepth 1 -type f -name '*.md' \
  ! -name "$(basename "$OUT")" \
  ! -iname '*template*' \
  -print | LC_ALL=C sort | while IFS= read -r f; do
    base="$(basename "$f")"
    title="${base%.md}"
    title="${title//_/ }"  # underscores -> spaces
    # GitHub-like slug: lowercase, spaces -> -, keep [a-z0-9-]
    slug="$(printf '%s' "$title" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-')"
    printf -- "- [%s](#%s)\n" "$title" "$slug" >> "$OUT"
done

# Second pass: append content with dividers and H1 per file
find "$SRC_DIR" -maxdepth 1 -type f -name '*.md' \
  ! -name "$(basename "$OUT")" \
  ! -iname '*template*' \
  -print | LC_ALL=C sort | while IFS= read -r f; do
    base="$(basename "$f")"
    title="${base%.md}"
    title="${title//_/ }"
    printf '\n\n---\n\n# %s\n\n' "$title" >> "$OUT"
    cat "$f" >> "$OUT"
    printf '\n' >> "$OUT"
done

echo "Wrote: $OUT"
echo "Source: $SRC_DIR"
