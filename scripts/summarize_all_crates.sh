#!/usr/bin/env bash
# summarize_all_crates.sh — run new_crate_summary.sh for every crate under crates/*/
# Works on macOS Bash 3.2 (no mapfile/readarray) and GNU Bash.
#
# Usage examples:
#   scripts/summarize_all_crates.sh --lean
#   scripts/summarize_all_crates.sh --lean --force --owner "Stevan White"
#
# Notes:
# - For strict workspace members, this script will prefer `cargo metadata` + `jq` if available.
# - Otherwise it falls back to scanning crates/*/Cargo.toml.
# - Forwards any flags you pass to scripts/new_crate_summary.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GEN="$ROOT/scripts/new_crate_summary.sh"

if [ ! -x "$GEN" ]; then
  echo "error: $GEN not found or not executable. Make sure new_crate_summary.sh exists and chmod +x it." >&2
  exit 1
fi

# Collect crate names via cargo metadata (preferred) or filesystem fallback.
collect_crates() {
  local crates_list=""
  if command -v cargo >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
    # Use cargo metadata to avoid non-member directories
    crates_list="$(cargo metadata --no-deps --format-version 1 \
      | jq -r --arg root "$ROOT" '
          .packages[]
          | select(.manifest_path | startswith($root + "/crates/") and endswith("/Cargo.toml"))
          | .name
        ' | sort -u || true)"
  else
    # Fallback: any directory under crates/ with a Cargo.toml is treated as a crate
    crates_list="$(find "$ROOT/crates" -mindepth 1 -maxdepth 1 -type d 2>/dev/null \
      -exec test -f "{}/Cargo.toml" \; -print \
      | xargs -I{} basename {} \
      | sort -u || true)"
  fi

  # Echo one name per line (safe for command substitution)
  if [ -n "$crates_list" ]; then
    printf "%s\n" "$crates_list"
  fi
}

main() {
  # Gather crates safely without mapfile/readarray
  IFS=$'\n' CRATES_ARR=($(collect_crates || true))
  unset IFS

  if [ "${#CRATES_ARR[@]}" -eq 0 ]; then
    echo "error: no crates found under crates/* (with Cargo.toml)" >&2
    exit 2
  fi

  echo "Found ${#CRATES_ARR[@]} crates:"
  for c in "${CRATES_ARR[@]}"; do
    echo "  - $c"
  done

  # Forward all user-provided flags/args ("$@") to the generator, then append crate names.
  "$GEN" "$@" "${CRATES_ARR[@]}"
}

main "$@"
