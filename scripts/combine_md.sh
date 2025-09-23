#!/usr/bin/env bash
# combine_md.sh — Combine Markdown files in a folder into a single Markdown file.
# - Skips CHANGELOG.MD, NOTE.MD, README.MD (case-insensitive).
# - Sorts alphabetically (case-insensitive).
# - Adds a Table of Contents and per-file H2 headers.
# - Avoids including the output file itself.
# - Usage:
#     bash combine_md.sh [SOURCE_DIR] [OUTPUT_FILE] [--recursive]
#   Defaults:
#     SOURCE_DIR  = .
#     OUTPUT_FILE = Combined.md

set -euo pipefail

# --- Config ---
SKIP_LOWER=("changelog.md" "note.md" "readme.md")

# --- Args ---
SRC_DIR="."
OUT_FILE="Combined.md"
RECURSIVE=0

# crude arg parse
for arg in "$@"; do
  if [[ "$arg" == "--recursive" ]]; then
    RECURSIVE=1
  fi
done

# Positional (without --recursive)
positional=()
for arg in "$@"; do
  [[ "$arg" == "--recursive" ]] && continue
  positional+=("$arg")
done

if [[ ${#positional[@]} -ge 1 ]]; then
  SRC_DIR="${positional[0]}"
fi
if [[ ${#positional[@]} -ge 2 ]]; then
  OUT_FILE="${positional[1]}"
fi

if [[ ! -d "$SRC_DIR" ]]; then
  echo "Source directory does not exist or is not a directory: $SRC_DIR" >&2
  exit 1
fi

# Ensure output dir exists
OUT_DIR=$(dirname -- "$OUT_FILE")
mkdir -p "$OUT_DIR"

# --- Collect files ---
files=()

if [[ "$RECURSIVE" -eq 1 ]]; then
  # Recursive collection with find. We’ll read line-based (assumes no newlines in file names).
  while IFS= read -r f; do
    files+=("$f")
  done < <(find "$SRC_DIR" -type f \( -iname '*.md' \) | sort -f)
else
  shopt -s nullglob
  tmp=("$SRC_DIR"/*.md "$SRC_DIR"/*.MD)
  shopt -u nullglob
  # Sort case-insensitive
  if [[ ${#tmp[@]} -gt 0 ]]; then
    while IFS= read -r f; do
      files+=("$f")
    done < <(printf '%s\n' "${tmp[@]}" | sort -f)
  fi
fi

# Skip list + avoid OUT_FILE itself
filtered=()
for f in "${files[@]}"; do
  # Skip output file itself if paths happen to collide
  if [[ -e "$OUT_FILE" ]] && [[ "$f" -ef "$OUT_FILE" ]]; then
    continue
  fi

  base=$(basename -- "$f")
  base_lower=$(printf '%s' "$base" | tr '[:upper:]' '[:lower:]')

  skip=0
  for s in "${SKIP_LOWER[@]}"; do
    if [[ "$base_lower" == "$s" ]]; then
      skip=1
      break
    fi
  done
  [[ $skip -eq 1 ]] && continue

  case "$base" in
    *.md|*.MD) filtered+=("$f");;
    *) ;;
  esac
done

if [[ ${#filtered[@]} -eq 0 ]]; then
  {
    echo "# Combined Markdown"
    echo
    echo "_Source directory_: \`$SRC_DIR\`  "
    echo "_Files combined_: 0  "
    echo "_Recursive_: $RECURSIVE"
    echo
  } > "$OUT_FILE"
  echo "No markdown files found to combine (after applying skip rules)."
  exit 0
fi

# --- Write output ---
{
  echo "# Combined Markdown"
  echo
  echo "_Source directory_: \`$SRC_DIR\`  "
  echo "_Files combined_: ${#filtered[@]}  "
  echo "_Recursive_: $RECURSIVE"
  echo
  echo "---"
  echo
  echo "### Table of Contents"
  echo
  for f in "${filtered[@]}"; do
    rel="${f#$SRC_DIR/}"   # strip leading SRC_DIR/ if present
    [[ "$rel" == "$f" ]] && rel="$f"
    echo "- $rel"
  done
  echo
  echo "---"
  echo

  total=${#filtered[@]}
  idx=0
  for f in "${filtered[@]}"; do
    idx=$((idx+1))
    rel="${f#$SRC_DIR/}"
    [[ "$rel" == "$f" ]] && rel="$(basename -- "$f")"

    echo "## $rel"
    echo "_File $idx of ${total}_"
    echo

    # Use UTF-8; replace invalid bytes
    if iconv -f UTF-8 -t UTF-8 "$f" >/dev/null 2>&1; then
      cat "$f"
    else
      echo "> **Note:** Non-UTF-8 content detected; including raw bytes."
      cat "$f"
    fi

    echo
    if [[ $idx -lt $total ]]; then
      echo
      echo "---"
      echo
    fi
  done
} > "$OUT_FILE"

echo "Combined ${#filtered[@]} file(s) into: $OUT_FILE"
