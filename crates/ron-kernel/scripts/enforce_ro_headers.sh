#!/usr/bin/env bash
set -euo pipefail

# Enforce presence of "RO:" header lines near the top of every Rust source file.
# Skips files under target/ and any generated code paths.

fail() { echo "RO header missing in: $1" >&2; exit 1; }

git ls-files 'crates/ron-kernel/**/*.rs' \
  | grep -vE '^target/' \
  | while read -r file; do
      head -n 25 "$file" | grep -q 'RO:' || fail "$file"
    done

echo "RO header check passed."
