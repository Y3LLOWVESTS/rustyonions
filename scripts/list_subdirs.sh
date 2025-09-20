#!/usr/bin/env bash
# list_subdirs.sh — list first-level subdirectories alphabetically

set -euo pipefail

DIR="${1:-.}"

# Validate input directory
if [[ ! -d "$DIR" ]]; then
  echo "Error: '$DIR' is not a directory or doesn't exist." >&2
  exit 1
fi

# Find first-level directories (including hidden), strip path, sort, and print
# -print0 handles spaces/newlines in names safely
# LC_ALL=C ensures bytewise, predictable sort order
LC_ALL=C \
find "$DIR" -mindepth 1 -maxdepth 1 -type d -print0 \
| while IFS= read -r -d '' path; do
    basename "$path"
  done \
| LC_ALL=C sort
