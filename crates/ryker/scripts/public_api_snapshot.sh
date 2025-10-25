#!/usr/bin/env bash
# Takes a public API snapshot for the `ryker` crate and optionally diffs it.
# Requirements:
#   - cargo-public-api  (install: cargo install cargo-public-api)
# Usage:
#   scripts/public_api_snapshot.sh save <name>        # save snapshot to target/public-api/<name>.txt
#   scripts/public_api_snapshot.sh diff <old> <new>   # show diff between two saved snapshots
#   scripts/public_api_snapshot.sh now                # print current API (no file)
# Env:
#   CARGO_FEATURES="..."   # e.g. "--features bench_support"

set -euo pipefail

# Find workspace root by walking upward until Cargo.toml is found.
find_workspace_root() {
  local d="$1"
  for _ in {1..6}; do
    if [[ -f "$d/Cargo.toml" ]]; then
      echo "$d"
      return 0
    fi
    d="$(dirname "$d")"
  done
  return 1
}

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
start_dir="$here"
repo="$(find_workspace_root "$start_dir" || true)"
if [[ -z "${repo:-}" ]]; then
  echo "Could not find workspace Cargo.toml by walking up from: $start_dir"
  echo "Hint: run this script from within the workspace, or set REPO_ROOT env var."
  exit 1
fi
cd "$repo"

if ! command -v cargo-public-api >/dev/null 2>&1; then
  echo "cargo-public-api not found. install it first:"
  echo "cargo install cargo-public-api"
  exit 1
fi

outdir="target/public-api"
mkdir -p "$outdir"

features="${CARGO_FEATURES:-}"

case "${1:-}" in
  save)
    name="${2:-ryker-$(date +%Y%m%d-%H%M%S)}"
    out="$outdir/$name.txt"
    cargo public-api -p ryker $features --simplified \
      --omit blanket-impls --omit auto-trait-impls > "$out"
    echo "Saved: $out"
    ;;
  diff)
    old="$outdir/${2:?old snapshot name}.txt"
    new="$outdir/${3:?new snapshot name}.txt"
    if [[ ! -f "$old" || ! -f "$new" ]]; then
      echo "Missing files: $old or $new"
      exit 2
    fi
    diff -u "$old" "$new" || true
    ;;
  now)
    cargo public-api -p ryker $features --simplified \
      --omit blanket-impls --omit auto-trait-impls
    ;;
  *)
    echo "Usage:"
    echo "  $0 save <name>"
    echo "  $0 diff <old> <new>"
    echo "  $0 now"
    exit 64
    ;;
esac
