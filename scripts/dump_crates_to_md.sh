#!/usr/bin/env bash
# dump_crates_to_md.sh — Workspace code dumper (.rs + .toml only)
# Usage:
#   bash scripts/dump_crates_to_md.sh [OUTPUT_MD] [ROOT_DIR]
# Examples:
#   bash scripts/dump_crates_to_md.sh
#   bash scripts/dump_crates_to_md.sh workspace_code_dump.md .
# Notes:
# - macOS Bash 3.2 compatible (no readarray).
# - Uses process substitution instead of a pipe to avoid subshell var loss.
# - Prints literal code fences with printf.

set -euo pipefail

OUT="${1:-workspace_code_dump.md}"
ROOT="${2:-.}"

# Resolve OUT to absolute path
ORIG_DIR="$(pwd)"
if [[ "$OUT" = /* ]]; then
  OUT_ABS="$OUT"
else
  OUT_ABS="$ORIG_DIR/$OUT"
fi
OUT_DIR="$(dirname "$OUT_ABS")"
mkdir -p "$OUT_DIR"

# Dirs to ignore
IGNORE_DIRS=(
  ".git" "target" "node_modules" "dist" "build" ".venv" ".idea" ".vscode"
  "__pycache__" "coverage" "tmp" "logs"
  ".onions" ".objects" ".object" ".testrun_store"
)

# Build a BSD/macOS compatible find command (printed to stdout)
build_find() {
  printf '%s' "find . "
  printf '%s' "\\( "
  local first=1
  for d in "${IGNORE_DIRS[@]}"; do
    if [[ $first -eq 0 ]]; then printf ' %s ' "-o"; fi
    # prune the dir itself and its contents
    printf "%s" "-path './$d' -o -path './$d/*'"
    first=0
  done
  printf ' %s ' "\\) -prune -o -type f \\( -name '*.rs' -o -name '*.toml' \\) -print0"
}

# Global array for files (must be initialized before any expansion under set -u)
FILES=()

collect_files() {
  local cmd
  cmd="$(build_find)"
  # Use process substitution to keep the while loop in the current shell (no subshell).
  # shellcheck disable=SC2294
  while IFS= read -r -d '' f; do
    FILES+=("${f#./}")
  done < <(eval "$cmd")

  # Sort (only if there are entries)
  if ((${#FILES[@]} > 0)); then
    IFS=$'\n' FILES=($(printf "%s\n" "${FILES[@]}" | LC_ALL=C sort))
    unset IFS
  fi
}

emit_tree() {
  if command -v tree >/dev/null 2>&1; then
    local ignore_pat
    ignore_pat="$(IFS='|'; echo "${IGNORE_DIRS[*]}")"
    tree -a --noreport -I "$ignore_pat" -P '*.rs|*.toml' .
  else
    printf "tree not found; showing a flat list instead.\n\n"
    if ((${#FILES[@]} > 0)); then
      for f in "${FILES[@]}"; do
        printf "%s\n" "$f"
      done
    fi
  fi
}

lang_for() {
  case "$1" in
    *.rs)   printf 'rust' ;;
    *.toml) printf 'toml' ;;
    *)      printf 'text' ;;
  esac
}

# Main
if [[ ! -d "$ROOT" ]]; then
  echo "Root directory not found: $ROOT" >&2
  exit 1
fi

pushd "$ROOT" >/dev/null

collect_files

{
  printf "# Workspace Code Dump (.rs + .toml only)\n"
  printf "_Generated: %s_\n\n" "$(date -u +"%Y-%m-%d %H:%M:%SZ")"
  printf "**Root:** \`%s\`\n\n" "$ROOT"
  printf "## File tree (.rs and .toml only)\n\n"
  printf '```text\n'
} > "$OUT_ABS"

emit_tree >> "$OUT_ABS"

{
  printf '```\n\n'
  printf "## Files\n"
} >> "$OUT_ABS"

if ((${#FILES[@]} > 0)); then
  for rel in "${FILES[@]}"; do
    {
      printf "\n### %s\n\n" "$rel"
      lang="$(lang_for "$rel")"
      printf '```%s\n' "$lang"
      # Use cat -- for safety; works on macOS too
      cat -- "$rel"
      printf '\n```\n'
    } >> "$OUT_ABS"
  done
fi

popd >/dev/null

printf "Wrote: %s\n" "$OUT_ABS"
