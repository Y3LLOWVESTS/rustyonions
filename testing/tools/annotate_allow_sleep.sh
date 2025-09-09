#!/usr/bin/env bash
# Annotate long sleeps (>=0.5s) in test scripts with " # allow-sleep"
# - Bash 3.2 compatible (macOS)
# - Uses the same ripgrep filter as CI
# - Normalizes any accidental ".../testing/testing/..." -> ".../testing/..."
# - Only annotates non-comment lines not already containing "allow-sleep"
# - Skips testing/lib, testing/tools, and annotate_* scripts
# - PRESERVES ORIGINAL FILE PERMISSIONS

set -euo pipefail

# repo root (robust)
if git rev-parse --show-toplevel >/dev/null 2>&1; then
  REPO_ROOT="$(git rev-parse --show-toplevel)"
else
  THIS="$0"
  case "$THIS" in
    /*) ABS="$THIS" ;;
    *)  ABS="$PWD/$THIS" ;;
  esac
  THIS_DIR="$(cd "$(dirname "$ABS")" && pwd)"
  REPO_ROOT="$(cd "$THIS_DIR/../../" && pwd)"
fi

TESTING_DIR="$REPO_ROOT/testing"

normalize_path() {
  local p="$1"
  while printf '%s' "$p" | grep -q '/testing/testing/'; do
    p="${p//\/testing\/testing\//\/testing\/}"
  done
  p="$(printf '%s' "$p" | sed -e 's#\([^:]\)//#\1/#g')"
  echo "$p"
}

RG_PATTERN='^[^#]*\bsleep[[:space:]]+([1-9][0-9]*([.][0-9]+)?|0\.(5|[6-9][0-9]*))\b'
RG_OUT="$(rg -n --pcre2 "$RG_PATTERN" "$TESTING_DIR" -S \
            -g '!lib/ready.sh' \
            -g '!tools/**' \
            -g '!**/annotate_allow_sleep*.sh' || true)"

if [ -z "$RG_OUT" ]; then
  echo "No long sleeps found under testing/ (matching CI's query)."
  echo "Done. Re-run: bash testing/ci_invariants.sh"
  exit 0
fi

FILES_TMP="$(mktemp)"
echo "$RG_OUT" | awk -F: '{print $1}' | sort -u > "$FILES_TMP"

# Get octal mode on macOS (Bash 3.2): %OLp prints permission bits only (e.g., 0755)
get_mode() {
  stat -f %OLp "$1" 2>/dev/null || echo 0644
}

annotate_file() {
  local file="$1"
  local tmp; tmp="$(mktemp)"
  local mode; mode="$(get_mode "$file")"

  awk '
    BEGIN { OFS=""; }
    /^[[:space:]]*#/ { print $0; next }
    {
      if ($0 ~ /\bsleep[[:space:]]+([1-9][0-9]*([.][0-9]+)?|0\.(5|[6-9][0-9]*))\b/ && $0 !~ /allow-sleep/) {
        print $0, "  # allow-sleep";
      } else {
        print $0;
      }
    }
  ' "$file" > "$tmp"

  chmod "$mode" "$tmp"
  mv "$tmp" "$file"
  echo "[annotated] ${file#"$REPO_ROOT/"} (mode preserved $mode)"
}

any=0
while IFS= read -r f; do
  [ -n "$f" ] || continue
  nf="$(normalize_path "$f")"
  if [ ! -f "$nf" ]; then
    base="$(basename "$nf")"
    found="$(find "$TESTING_DIR" -type f -name "$base" 2>/dev/null | head -n 1 || true)"
    [ -n "$found" ] && nf="$found"
  fi
  if [ ! -f "$nf" ]; then
    echo "[skip] $f (normalized to $nf but not found)"
    continue
  fi
  annotate_file "$nf"
  any=1
done < "$FILES_TMP"

rm -f "$FILES_TMP"
[ "$any" -eq 0 ] && echo "Nothing annotated." || echo "Done. Re-run: bash testing/ci_invariants.sh"
