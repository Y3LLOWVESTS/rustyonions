#!/usr/bin/env bash
set -euo pipefail

SOCK="${RON_INDEX_SOCK:-/tmp/ron/svc-index.sock}"
DB_DIR="$(mktemp -d -t ron_index.XXXXXX)"
DB="$DB_DIR/db"
LOG="/tmp/ron_stack.log"

cleanup() {
  echo "[*] Cleanup"
  [[ -n "${IDX_PID:-}" ]] && kill "$IDX_PID" 2>/dev/null || true
  [[ -n "${GW_PID:-}"  ]] && kill "$GW_PID"  2>/dev/null || true
  rm -f "$SOCK"
  rm -rf "$DB_DIR"
}
trap cleanup EXIT

echo "[+] build"
cargo build -q -p svc-index -p gateway -p ronctl

echo "[+] start svc-index"
: > "$LOG"
(RON_INDEX_SOCK="$SOCK" RON_INDEX_DB="$DB" RUST_LOG=info cargo run -q -p svc-index >> "$LOG" 2>&1) & IDX_PID=$!

echo -n "[+] wait for index sock "
for i in {1..600}; do [[ -S "$SOCK" ]] && { echo; break; }; printf "."; sleep 0.1; done
[[ -S "$SOCK" ]] || { echo; tail -n +1 "$LOG"; exit 1; }

echo "[+] ping index"
cargo run -q -p ronctl -- ping

echo "[+] start gateway (reads RON_INDEX_SOCK)"
(RON_INDEX_SOCK="$SOCK" cargo run -q -p gateway >> "$LOG" 2>&1) & GW_PID=$!

echo "[i] add a mapping and try your usual curl:"
# example:
# ADDR="b3:<your-address>.text"
# DIR="<bundle-dir>"
# cargo run -q -p ronctl -- put "$ADDR" "$DIR"
# curl -i http://127.0.0.1:54087/o/$ADDR/payload.bin

echo "[i] tailing logs (Ctrl+C to stop)"
tail -f "$LOG"
