#!/usr/bin/env bash
set -euo pipefail

# Portable BLAKE3 hashing: prefer 'b3sum' if available, else fallback to 'shasum -a 256' as placeholder.
# Replace fallback with a real BLAKE3 tool in your env/CI.

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VEC_DIR="$DIR/testdata/vectors"
OUT="$DIR/testdata/signatures/vectors.b3.txt"

if command -v b3sum >/dev/null 2>&1; then
  (cd "$VEC_DIR" && b3sum names_ascii.json names_unicode_mixed.json tldmap_minimal.json tldmap_minimal.cbor) > "$OUT"
  echo "algo=BLAKE3-256" >> "$OUT"
  echo "OK wrote $OUT (BLAKE3)"
else
  (cd "$VEC_DIR" && shasum -a 256 names_ascii.json names_unicode_mixed.json tldmap_minimal.json tldmap_minimal.cbor) > "$OUT"
  echo "algo=SHA-256 (TEMPORARY FALLBACK — replace with BLAKE3 in CI)" >> "$OUT"
  echo "Wrote $OUT (SHA-256 fallback)"
fi
