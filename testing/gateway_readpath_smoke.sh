#!/usr/bin/env bash
# testing/gateway_readpath_smoke.sh — end-to-end stack:
# pack → svc-index (UDS) → svc-storage (UDS) → svc-overlay (UDS) → gateway (HTTP) → GET
#
# Usage:
#   RON_INDEX_DB=/tmp/ron.index ./testing/gateway_readpath_smoke.sh
#
# Run:
# chmod +x testing/lib/ready.sh                                                                    
# chmod +x testing/gateway_readpath_smoke.sh
#
# Example Test:
# RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 testing/gateway_readpath_smoke.sh
#
# Optional env:
#   ROOT=. OUT_DIR=.onions TLD=text BIND=127.0.0.1:9080 OBJ_FILE=<override>
# 
# "Hello Rusty Onions" = Successful test

set -euo pipefail

# ---- readiness helpers (source if present) ----
if [ -f testing/lib/ready.sh ]; then
  # shellcheck source=/dev/null
  source testing/lib/ready.sh
else
  # minimal fallbacks if ready.sh isn't available
  _http_code() { curl -s -o /dev/null -w "%{http_code}" "$1" || echo "000"; }
  wait_http_status() {
    local url="$1"; local want="$2"; local timeout="${3:-30}"; local end=$((SECONDS+timeout))
    while [ $SECONDS -lt $end ]; do [ "$(_http_code "$url")" = "$want" ] && return 0; sleep 0.2; done
    return 1
  }
  wait_udsocket() { local p="$1"; local t="${2:-20}"; local end=$((SECONDS+t)); while [ $SECONDS -lt $end ]; do [ -S "$p" ] && return 0; sleep 0.2; done; return 1; }
fi

ROOT="${ROOT:-.}"
RON_INDEX_DB="${RON_INDEX_DB:-$ROOT/.tmp/index}"
OUT_DIR="${OUT_DIR:-$ROOT/.onions}"
TLD="${TLD:-text}"
BIND="${BIND:-127.0.0.1:9080}"

TLDCTL="${TLDCTL:-$ROOT/target/debug/tldctl}"
GW="${GW:-$ROOT/target/debug/gateway}"
IDX="${IDX:-$ROOT/target/debug/svc-index}"
STO="${STO:-$ROOT/target/debug/svc-storage}"
OVL="${OVL:-$ROOT/target/debug/svc-overlay}"

need_build=false
[[ -x "$TLDCTL" ]] || need_build=true
[[ -x "$GW"     ]] || need_build=true
[[ -x "$IDX"    ]] || need_build=true
[[ -x "$STO"    ]] || need_build=true
[[ -x "$OVL"    ]] || need_build=true

if $need_build; then
  echo "[*] Building tldctl + services + gateway (debug)"
  cargo build -q -p tldctl -p svc-index -p svc-storage -p svc-overlay -p gateway
fi

[[ -x "$TLDCTL" ]] || { echo "tldctl not found at $TLDCTL"; exit 1; }
[[ -x "$GW"     ]] || { echo "gateway not found at $GW"; exit 1; }
[[ -x "$IDX"    ]] || { echo "svc-index not found at $IDX"; exit 1; }
[[ -x "$STO"    ]] || { echo "svc-storage not found at $STO"; exit 1; }
[[ -x "$OVL"    ]] || { echo "svc-overlay not found at $OVL"; exit 1; }

echo "[*] Using index db: $RON_INDEX_DB"
mkdir -p "$(dirname "$RON_INDEX_DB")" "$OUT_DIR"

# ---- pack sample ----
TMP_DIR="$(mktemp -d -t ron_readpath.XXXXXX)"
INPUT="$TMP_DIR/payload.bin"
printf "hello rusty onions\n" > "$INPUT"

echo "[*] Packing a sample bundle (tld=$TLD)"
PACK_OUT="$(RON_INDEX_DB="$RON_INDEX_DB" "$TLDCTL" pack \
  --tld "$TLD" \
  --input "$INPUT" \
  --index-db "$RON_INDEX_DB" \
  --store-root "$OUT_DIR" | tee "$TMP_DIR/pack.out")" || { echo "tldctl pack failed"; exit 1; }
ADDR="$(printf "%s\n" "$PACK_OUT" | grep -Eo 'b3:[0-9a-f]{8,}\.[A-Za-z0-9._-]+' | tail -n1)"
[[ -n "$ADDR" ]] || { echo "Could not parse address from tldctl output"; exit 1; }
ADDR_NOPREFIX="${ADDR#b3:}"
echo "packed: $ADDR_NOPREFIX"

# ---- sockets/logs ----
RUN_DIR="$TMP_DIR/run"
LOG_DIR="$TMP_DIR/logs"
mkdir -p "$RUN_DIR" "$LOG_DIR"

IDX_SOCK="$RUN_DIR/svc-index.sock"
STO_SOCK="$RUN_DIR/svc-storage.sock"
OVL_SOCK="$RUN_DIR/svc-overlay.sock"

IDX_LOG="$LOG_DIR/svc-index.log"
STO_LOG="$LOG_DIR/svc-storage.log"
OVL_LOG="$LOG_DIR/svc-overlay.log"
GW_LOG="$LOG_DIR/gateway.log"

cleanup() {
  echo "[*] Cleanup"
  [[ -n "${GW_PID:-}"  ]] && kill "$GW_PID"  2>/dev/null || true
  [[ -n "${OVL_PID:-}" ]] && kill "$OVL_PID" 2>/dev/null || true
  [[ -n "${STO_PID:-}" ]] && kill "$STO_PID" 2>/dev/null || true
  [[ -n "${IDX_PID:-}" ]] && kill "$IDX_PID" 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# ---- start services ----
echo "[*] Starting svc-index @ $IDX_SOCK"
(RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG=info "$IDX" >"$IDX_LOG" 2>&1) & IDX_PID=$!

echo "[*] Starting svc-storage @ $STO_SOCK"
(RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$STO" >"$STO_LOG" 2>&1) & STO_PID=$!

echo "[*] Starting svc-overlay @ $OVL_SOCK (-> index=$IDX_SOCK, storage=$STO_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$OVL" >"$OVL_LOG" 2>&1) & OVL_PID=$!

wait_udsocket "$OVL_SOCK" 20 || { echo "[!] overlay UDS not ready: $OVL_SOCK"; sed -n '1,200p' "$OVL_LOG" || true; exit 1; }

# ---- ensure port is free (auto-pick if busy) ----
HOST="${BIND%%:*}"
PORT="${BIND##*:}"
if command -v nc >/dev/null 2>&1; then
  if nc -z "$HOST" "$PORT" >/dev/null 2>&1; then
    for p in $(seq $((PORT+1)) $((PORT+50))); do
      if ! nc -z "$HOST" "$p" >/dev/null 2>&1; then
        echo "[!] Port $PORT in use; switching to $HOST:$p"
        BIND="$HOST:$p"
        break
      fi
    done
  fi
fi

echo "[*] Starting gateway on $BIND (RON_OVERLAY_SOCK=$OVL_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RUST_LOG=info "$GW" --bind "$BIND" >"$GW_LOG" 2>&1) & GW_PID=$!

BASE="http://$BIND"

# basic readiness
if ! wait_http_status "$BASE/healthz" 200 20; then
  echo "[!] gateway /healthz not ready (check bind/port)"; sed -n '1,200p' "$GW_LOG" || true; exit 1
fi

# pick object file based on suffix unless overridden
EXT="${ADDR_NOPREFIX##*.}"
OBJ_FILE="${OBJ_FILE:-}"
if [ -z "$OBJ_FILE" ]; then
  case "$EXT" in
    post|dir) OBJ_FILE="Manifest.toml" ;;
    text|bin) OBJ_FILE="payload.bin" ;;
    *)        OBJ_FILE="Manifest.toml" ;;
  esac
fi

URL="$BASE/o/$ADDR_NOPREFIX/$OBJ_FILE"
echo "[*] Waiting for gateway to serve: $URL"
if ! wait_http_status "$URL" 200 30; then
  ALT="$([ "$OBJ_FILE" = "Manifest.toml" ] && echo "payload.bin" || echo "Manifest.toml")"
  ALT_URL="$BASE/o/$ADDR_NOPREFIX/$ALT"
  echo "[!] $OBJ_FILE not found within timeout; trying $ALT_URL"
  if ! wait_http_status "$ALT_URL" 200 15; then
    echo "[!] Gateway did not serve the object. Recent logs:"
    echo "---- gateway ----"; sed -n '1,200p' "$GW_LOG" || true
    echo "---- overlay ----"; sed -n '1,200p' "$OVL_LOG" || true
    echo "---- storage ----"; sed -n '1,200p' "$STO_LOG" || true
    echo "---- index   ----"; sed -n '1,200p' "$IDX_LOG" || true
    exit 1
  else
    URL="$ALT_URL"
  fi
fi

echo "[*] GET: $URL"
curl -fsS "$URL" | sed -n '1,80p' || { echo "[!] GET failed"; sed -n '1,200p' "$GW_LOG" || true; exit 1; }

echo "[*] OK (see logs in $LOG_DIR)"
