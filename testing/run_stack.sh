#!/usr/bin/env bash
set -euo pipefail
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
# EXAMPLE TEST: HOLD=1 PACK_FIRST=1 PACK_TEXT="hello via PACK_FIRST" \
# RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9082 \ 
# testing/run_stack.sh
# THEN RUN http_cache_smoke.sh (Read in comments how to run)

# Env:
#   RON_INDEX_DB (default /tmp/ron.index; override to share with other tools)
#   OUT_DIR      (default .onions)
#   BIND=127.0.0.1:9080 (can change, script will auto-bump if busy)
#   RUN_DIR=./.tmp/stack
#   HOLD=1 to keep running until Ctrl-C
#   PACK_FIRST=1 to pre-pack text or a file (PACK_TLD=text, PACK_TEXT="...", or PACK_FILE=/path)

ROOT="${ROOT:-.}"
RUN_DIR="${RUN_DIR:-./.tmp/stack}"
RON_INDEX_DB="${RON_INDEX_DB:-/tmp/ron.index}"
OUT_DIR="${OUT_DIR:-.onions}"
BIND="${BIND:-127.0.0.1:9080}"

TLDCTL="${TLDCTL:-$ROOT/target/debug/tldctl}"
GW="${GW:-$ROOT/target/debug/gateway}"
IDX="${IDX:-$ROOT/target/debug/svc-index}"
STO="${STO:-$ROOT/target/debug/svc-storage}"
OVL="${OVL:-$ROOT/target/debug/svc-overlay}"

mkdir -p "$RUN_DIR"/{sock,log}

IDX_SOCK="$RUN_DIR/sock/index.sock"
STO_SOCK="$RUN_DIR/sock/storage.sock"
OVL_SOCK="$RUN_DIR/sock/overlay.sock"

IDX_LOG="$RUN_DIR/log/index.log"
STO_LOG="$RUN_DIR/log/storage.log"
OVL_LOG="$RUN_DIR/log/overlay.log"
GW_LOG="$RUN_DIR/log/gateway.log"

echo "[*] RUN_DIR=$RUN_DIR"
echo "[*] RON_INDEX_DB=$RON_INDEX_DB"
echo "[*] OUT_DIR=$OUT_DIR"
echo "[*] BIND=$BIND"

source testing/lib/ready.sh

# ---- Optional pre-pack (before services to avoid DB lock) ----
PRE_URL=""
if [[ "${PACK_FIRST:-0}" = "1" ]]; then
  PACK_TLD="${PACK_TLD:-text}"
  if [[ -n "${PACK_FILE:-}" ]]; then
    INPUT="$PACK_FILE"
  else
    INPUT="$(mktemp)"
    : "${PACK_TEXT:=hello via PACK_FIRST}"
    printf "%s\n" "$PACK_TEXT" > "$INPUT"
  fi
  echo "[*] Pre-pack: tld=$PACK_TLD input=$INPUT"
  PACK_OUT="$(RON_INDEX_DB="$RON_INDEX_DB" "$TLDCTL" pack \
      --tld "$PACK_TLD" --input "$INPUT" \
      --index-db "$RON_INDEX_DB" --store-root "$OUT_DIR")"
  ADDR="$(printf "%s\n" "$PACK_OUT" | grep -Eo 'b3:[0-9a-f]{8,}\.[A-Za-z0-9._-]+' | tail -n1)"
  [[ -n "$ADDR" ]] || { echo "[!] tldctl pack produced no address"; exit 1; }
  echo "[*] Packed address: $ADDR"
  ADDR_NOPREFIX="${ADDR#b3:}"
  # Choose object file by suffix if you like; for text we use payload.bin
  PRE_URL="http://${BIND}/o/${ADDR_NOPREFIX}/payload.bin"
fi

# ---- Start services (UDS) ----
echo "[*] Starting svc-index @ $IDX_SOCK"
(RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG=info "$IDX" >"$IDX_LOG" 2>&1) & IDX_PID=$!

echo "[*] Starting svc-storage @ $STO_SOCK"
(RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$STO" >"$STO_LOG" 2>&1) & STO_PID=$!

echo "[*] Starting svc-overlay @ $OVL_SOCK (index=$IDX_SOCK, storage=$STO_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$OVL" >"$OVL_LOG" 2>&1) & OVL_PID=$!

# Wait for sockets
wait_udsocket "$IDX_SOCK" 15 || { echo "[!] index sock not ready"; exit 1; }
wait_udsocket "$STO_SOCK" 15 || { echo "[!] storage sock not ready"; exit 1; }
wait_udsocket "$OVL_SOCK" 15 || { echo "[!] overlay sock not ready"; exit 1; }

# ---- Bind picker (avoid "address in use") ----
pick_bind() {
  local host="${1%:*}" port="${1##*:}"
  for ((p=port; p<port+20; p++)); do
    if ! (nc -z "$host" "$p" >/dev/null 2>&1); then
      echo "${host}:$p"; return 0
    fi
  done
  return 1
}
NEW_BIND="$(pick_bind "$BIND")" || { echo "[!] no free port near $BIND"; exit 1; }
if [[ "$NEW_BIND" != "$BIND" ]]; then
  echo "[!] $BIND busy; switching to $NEW_BIND"
  BIND="$NEW_BIND"
fi

echo "[*] Starting gateway on $BIND"
(RON_OVERLAY_SOCK="$OVL_SOCK" RUST_LOG=info "$GW" --bind "$BIND" >"$GW_LOG" 2>&1) & GW_PID=$!

trap 'echo "[*] Cleanup"; kill "$GW_PID" "$OVL_PID" "$STO_PID" "$IDX_PID" 2>/dev/null || true' EXIT

wait_http_ok "http://$BIND/healthz" 20 || {
  echo "[!] gateway /healthz not ready"
  echo "---- gateway ----"; tail -n +1 "$GW_LOG" | sed -n '1,120p'; exit 1;
}

if [[ -n "$PRE_URL" ]]; then
  echo
  echo "[*] Pre-packed URL:"
  echo "    $PRE_URL"
  echo
fi

if [[ "${HOLD:-0}" = "1" ]]; then
  echo "[*] Stack is up. Press Ctrl-C to stop."
  while true; do sleep 86400; done # allow-sleep
else
  echo "[*] Stack is up."
fi
