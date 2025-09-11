#!/usr/bin/env bash
# testing/run_stack.sh — start full RustyOnions stack and keep it running until Ctrl-C
# Services: svc-index (UDS) → svc-storage (UDS) → svc-overlay (UDS) → gateway (HTTP)
#
# Env (override as needed):
#   ROOT=.                 # repo root
#   RON_INDEX_DB=/tmp/ron.index
#   OUT_DIR=.onions
#   BIND=127.0.0.1:9082
#   RUST_LOG=info
#
# Optional pack-before-start:
#   PACK_FIRST=1           # if set to 1, pack before starting services (avoids sled lock)
#   PACK_TLD=text          # tld for pre-pack (default: text)
#   PACK_INPUT=/path/file  # file to pack; if empty and PACK_TEXT set, a temp file is created
#   PACK_TEXT="hello..."   # inline content to pack if PACK_INPUT unset
#
# After startup, we print the last packed URL (if any) for convenience.
# 
# EXAMPLE TEST: RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9082 testing/run_stack.sh
# THEN RUN http_cache_smoke.sh (Read in comments how to run)

set -euo pipefail
source testing/lib/ready.sh

ROOT="${ROOT:-.}"
cd "$ROOT"

RON_INDEX_DB="${RON_INDEX_DB:-/tmp/ron.index}"
OUT_DIR="${OUT_DIR:-.onions}"
BIND="${BIND:-127.0.0.1:9082}"
RUST_LOG="${RUST_LOG:-info}"

PACK_FIRST="${PACK_FIRST:-0}"
PACK_TLD="${PACK_TLD:-text}"
PACK_INPUT="${PACK_INPUT:-}"
PACK_TEXT="${PACK_TEXT:-}"

IDX="${IDX:-$ROOT/target/debug/svc-index}"
STO="${STO:-$ROOT/target/debug/svc-storage}"
OVL="${OVL:-$ROOT/target/debug/svc-overlay}"
GW="${GW:-$ROOT/target/debug/gateway}"
TLDCTL="${TLDCTL:-$ROOT/target/debug/tldctl}"

RUN_DIR="$ROOT/.tmp/stack"
LOG_DIR="$RUN_DIR/logs"
SOCK_DIR="$RUN_DIR/sock"
mkdir -p "$LOG_DIR" "$SOCK_DIR" "$(dirname "$RON_INDEX_DB")" "$OUT_DIR"

IDX_SOCK="$SOCK_DIR/index.sock"
STO_SOCK="$SOCK_DIR/storage.sock"
OVL_SOCK="$SOCK_DIR/overlay.sock"

GW_LOG="$LOG_DIR/gateway.log"
IDX_LOG="$LOG_DIR/index.log"
STO_LOG="$LOG_DIR/storage.log"
OVL_LOG="$LOG_DIR/overlay.log"

LAST_ADDR_FILE="$RUN_DIR/last_addr.txt"

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1"; exit 1; }; }
need curl

echo "[*] RUN_DIR=$RUN_DIR"
echo "[*] RON_INDEX_DB=$RON_INDEX_DB"
echo "[*] OUT_DIR=$OUT_DIR"
echo "[*] BIND=$BIND"

cleanup() {
  echo
  echo "[*] Shutting down…"
  [[ -n "${GW_PID:-}"  ]] && kill "$GW_PID"  2>/dev/null || true
  [[ -n "${OVL_PID:-}" ]] && kill "$OVL_PID" 2>/dev/null || true
  [[ -n "${STO_PID:-}" ]] && kill "$STO_PID" 2>/dev/null || true
  [[ -n "${IDX_PID:-}" ]] && kill "$IDX_PID" 2>/dev/null || true
  echo "[*] Logs: $LOG_DIR"
}
trap cleanup INT TERM

# ---- optional pre-pack (before services grab the sled lock) ----
do_pack() {
  local tld="${1:-text}"
  local input="${2:-}"
  local text="${3:-}"
  local tmp=""
  if [ -z "$input" ]; then
    if [ -n "$text" ]; then
      tmp="$(mktemp)"; printf "%s\n" "$text" > "$tmp"; input="$tmp"
    else
      tmp="$(mktemp)"; printf "hello from PACK_FIRST\n" > "$tmp"; input="$tmp"
    fi
  fi
  echo "[*] Pre-pack: tld=$tld input=$input"
  if ! [[ -x "$TLDCTL" ]]; then
    echo "[*] building tldctl…" ; cargo build -q -p tldctl
  fi
  PACK_OUT="$("$TLDCTL" pack --tld "$tld" --input "$input" --index-db "$RON_INDEX_DB" --store-root "$OUT_DIR")"
  ADDR="$(printf "%s\n" "$PACK_OUT" | grep -Eo 'b3:[0-9a-f]{8,}\.[A-Za-z0-9._-]+' | tail -n1 || true)"
  if [ -n "$ADDR" ]; then
    echo "$ADDR" > "$LAST_ADDR_FILE"
    echo "[*] Packed address: $ADDR"
  else
    echo "[!] Pre-pack produced no address (check tldctl output):"
    echo "$PACK_OUT"
  fi
  [ -n "$tmp" ] && rm -f "$tmp"
}

if [ "$PACK_FIRST" = "1" ]; then
  do_pack "$PACK_TLD" "$PACK_INPUT" "$PACK_TEXT"
fi

echo "[*] Starting svc-index @ $IDX_SOCK"
(RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG="$RUST_LOG" "$IDX" >"$IDX_LOG" 2>&1) & IDX_PID=$!

echo "[*] Starting svc-storage @ $STO_SOCK"
(RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$STO" >"$STO_LOG" 2>&1) & STO_PID=$!

echo "[*] Starting svc-overlay @ $OVL_SOCK (index=$IDX_SOCK, storage=$STO_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$OVL" >"$OVL_LOG" 2>&1) & OVL_PID=$!

# Wait for overlay UDS to exist
wait_udsocket "$OVL_SOCK" 20 || { echo "[!] overlay UDS not ready: $OVL_SOCK"; exit 1; }

# If requested port is busy, suggest another
HOST="${BIND%%:*}"; PORT="${BIND##*:}"
if command -v nc >/dev/null 2>&1; then
  if nc -z "$HOST" "$PORT" >/dev/null 2>&1; then
    echo "[!] Port $PORT appears busy. You can rerun with BIND=${HOST}:$((PORT+1))"
  fi
fi

echo "[*] Starting gateway on $BIND"
(RON_OVERLAY_SOCK="$OVL_SOCK" RUST_LOG="$RUST_LOG" "$GW" --bind "$BIND" >"$GW_LOG" 2>&1) & GW_PID=$!

# Wait for HTTP readiness
wait_http_ok "http://$BIND/healthz" 30 || { echo "[!] gateway not ready"; exit 1; }

# If we pre-packed, show a ready-to-copy URL
if [ -f "$LAST_ADDR_FILE" ]; then
  ADDR="$(cat "$LAST_ADDR_FILE")"
  ADDR_NOPREFIX="${ADDR#b3:}"
  EXT="${ADDR_NOPREFIX##*.}"
  case "$EXT" in
    post|dir) FILE="Manifest.toml" ;;
    *)        FILE="payload.bin" ;;
  esac
  echo
  echo "[*] Pre-packed URL:"
  echo "    http://$BIND/o/$ADDR_NOPREFIX/$FILE"
fi

echo
echo "[*] Stack is up. Press Ctrl-C to stop."
wait
