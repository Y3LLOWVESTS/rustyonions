#!/usr/bin/env bash
set -euo pipefail

echo "[*] rustc version:"; rustc -V
echo "[*] cargo version:"; cargo -V

if ! command -v rg >/dev/null 2>&1; then
  echo "ripgrep (rg) is required"; exit 1
fi

echo "[*] Running Clippy with strict lints…"
cargo clippy --all-targets --all-features \
  -D warnings \
  -D clippy::await_holding_lock \
  -D clippy::await_holding_refcell_ref \
  -D clippy::mutex_atomic \
  -D clippy::unwrap_used \
  -D clippy::expect_used

echo "[*] Checking for banned patterns…"
if rg -n "sha-?256|sha256:" -S .; then
  echo "Found SHA-256 remnants; migrate to BLAKE3 and update docs."; exit 1
fi
rg -n "b3:" -S specs || { echo "Expected 'b3:' address reference in specs/"; exit 1; }
rg -n "max_frame\s*=\s*1\s*MiB" -S specs || { echo "Expected OAP/1 max_frame = 1 MiB in specs/"; exit 1; }

SLEEP_HITS="$(rg -n '\bsleep\s+\d+(\.\d+)?' testing || true)"
if [ -n "$SLEEP_HITS" ]; then
  BAD="$(printf '%s\n' "$SLEEP_HITS" | grep -v 'allow-sleep' || true)"
  if [ -n "$BAD" ]; then
    echo "Arbitrary sleeps found in testing/ (mark rare intentional ones with 'allow-sleep'):"
    printf '%s\n' "$BAD"
    exit 1
  fi
fi

if rg -n "static mut|lazy_static!" -S . >/dev/null; then
  echo "Global mutable state detected; replace with Arc/locks or OnceCell/OnceLock."; exit 1
fi

echo "[ok] CI invariants passed."
