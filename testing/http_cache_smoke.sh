#!/usr/bin/env bash
# testing/http_cache_smoke.sh — cache/headers smoke against a running gateway
# Behavior:
# - If gateway at $BIND is up AND we have an address (env or .tmp/stack/last_addr.txt), we reuse it (no pack).
# - If gateway is down, we self-pack one object and then start a minimal stack (and clean it up).
#
# Env:
#   BIND=127.0.0.1:PORT            # default 127.0.0.1:9080
#   RON_INDEX_DB=/tmp/ron.index    # sled DB path
#   OUT_DIR=.onions                # store root
#   ADDR=b3:<hex>.<ext>            # optional (with b3:)
#   ADDR_NOPREFIX=<hex>.<ext>      # optional (without b3:)
#
# Requires: curl; optionally rg (fallback to grep -Ei).
#
#
# 
:' Test Example (Run with run_stack.sh and grab the output and replace HASH_FILE_OUTPUT AND PORT):

ADDR_NOPREFIX=<HASH_FILE_OUTPUT>.text \
BIND=127.0.0.1:<PORT> \
RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions \
testing/http_cache_smoke.sh

'
#
#





set -euo pipefail

# ---- tiny readiness helpers ----
_http_code() { curl -s -o /dev/null -w "%{http_code}" "$1" || echo "000"; }
pause() { sleep 0.2; }  # allow-sleep (tight polling)
wait_http_status() { local url="$1" want="$2" t="${3:-20}"; local end=$((SECONDS+t)); while [ $SECONDS -lt $end ]; do [ "$(_http_code "$url")" = "$want" ] && return 0; pause; done; return 1; }

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1"; exit 1; }; }
need curl

BIND="${BIND:-127.0.0.1:9080}"
BASE="http://$BIND"
RON_INDEX_DB="${RON_INDEX_DB:-/tmp/ron.index}"
OUT_DIR="${OUT_DIR:-.onions}"

# ---- resolve address to test (prefer env, then last_addr.txt) ----
ADDR_NOPREFIX="${ADDR_NOPREFIX:-}"
if [ -z "$ADDR_NOPREFIX" ]; then
  if [ -n "${ADDR:-}" ]; then
    ADDR_NOPREFIX="${ADDR#b3:}"
  elif [ -f "$ROOT/.tmp/stack/last_addr.txt" ]; then
    RAW="$(cat "$ROOT/.tmp/stack/last_addr.txt" | tr -d '\r\n' || true)"
    [ -n "$RAW" ] && ADDR_NOPREFIX="${RAW#b3:}"
  fi
fi

# Pick object file from suffix (allow override via OBJ_FILE)
EXT="${ADDR_NOPREFIX##*.}"
OBJ_FILE="${OBJ_FILE:-}"
if [ -z "$OBJ_FILE" ]; then
  case "$EXT" in
    post|dir) OBJ_FILE="Manifest.toml" ;;
    text|bin|txt) OBJ_FILE="payload.bin" ;;
    *)        OBJ_FILE="Manifest.toml" ;;
  esac
fi

# ---- check gateway up ----
GW_UP=0
if wait_http_status "$BASE/healthz" 200 1; then GW_UP=1; fi

# If gateway is up but we still don't have an address, ask for one explicitly.
if [ "$GW_UP" -eq 1 ] && [ -z "$ADDR_NOPREFIX" ]; then
  echo "[!] Gateway is running at $BIND, but no address found."
  echo "    Set ADDR or ADDR_NOPREFIX (e.g., from run_stack’s Pre-packed URL), then re-run."
  exit 1
fi

# If gateway is down, self-pack one object and start a tiny stack (and clean it on exit).
if [ "$GW_UP" -eq 0 ]; then
  echo "[*] Gateway not detected at $BIND — packing and starting a local stack"
  TLDCTL="${TLDCTL:-$ROOT/target/debug/tldctl}"
  IDX="${IDX:-$ROOT/target/debug/svc-index}"
  STO="${STO:-$ROOT/target/debug/svc-storage}"
  OVL="${OVL:-$ROOT/target/debug/svc-overlay}"
  GW="${GW:-$ROOT/target/debug/gateway}"

  # Build missing bits
  [[ -x "$TLDCTL" ]] || cargo build -q -p tldctl
  [[ -x "$IDX"    ]] || cargo build -q -p svc-index
  [[ -x "$STO"    ]] || cargo build -q -p svc-storage
  [[ -x "$OVL"    ]] || cargo build -q -p svc-overlay
  [[ -x "$GW"     ]] || cargo build -q -p gateway

  TMP_DIR="$(mktemp -d -t ron_httpcache.XXXXXX)"
  trap '[[ -n "${GW_PID:-}"  ]] && kill "$GW_PID"  2>/dev/null || true; [[ -n "${OVL_PID:-}" ]] && kill "$OVL_PID" 2>/dev/null || true; [[ -n "${STO_PID:-}" ]] && kill "$STO_PID" 2>/dev/null || true; [[ -n "${IDX_PID:-}" ]] && kill "$IDX_PID" 2>/dev/null || true; rm -rf "$TMP_DIR"' EXIT

  mkdir -p "$(dirname "$RON_INDEX_DB")" "$OUT_DIR" "$TMP_DIR/run" "$TMP_DIR/logs"
  IDX_SOCK="$TMP_DIR/run/index.sock"; STO_SOCK="$TMP_DIR/run/storage.sock"; OVL_SOCK="$TMP_DIR/run/overlay.sock"
  IDX_LOG="$TMP_DIR/logs/index.log";  STO_LOG="$TMP_DIR/logs/storage.log";  OVL_LOG="$TMP_DIR/logs/overlay.log"; GW_LOG="$TMP_DIR/logs/gateway.log"

  # Pre-pack (no lock conflict yet)
  INPUT="$(mktemp)"; echo "hello cache world" > "$INPUT"
  PACK_OUT="$("$TLDCTL" pack --tld text --input "$INPUT" --index-db "$RON_INDEX_DB" --store-root "$OUT_DIR")"
  ADDR="$(printf "%s\n" "$PACK_OUT" | grep -Eo 'b3:[0-9a-f]{8,}\.[A-Za-z0-9._-]+' | tail -n1)"
  [ -n "$ADDR" ] || { echo "[!] Could not parse address from tldctl output"; echo "$PACK_OUT"; exit 1; }
  ADDR_NOPREFIX="${ADDR#b3:}"
  EXT="${ADDR_NOPREFIX##*.}"; [ "$EXT" = "text" ] && OBJ_FILE="payload.bin"

  # Start services
  (RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG=info "$IDX" >"$IDX_LOG" 2>&1) & IDX_PID=$!
  (RON_STORAGE_SOCK="$STO_SOCK"                      RUST_LOG=info "$STO" >"$STO_LOG" 2>&1) & STO_PID=$!
  (RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$OVL" >"$OVL_LOG" 2>&1) & OVL_PID=$!

  # Wait for overlay socket
  end=$((SECONDS+20)); while [ $SECONDS -lt $end ]; do [ -S "$OVL_SOCK" ] && break; pause; done
  [ -S "$OVL_SOCK" ] || { echo "[!] overlay UDS not ready"; sed -n '1,120p' "$OVL_LOG" || true; exit 1; }

  # Start gateway
  (RON_OVERLAY_SOCK="$OVL_SOCK" RUST_LOG=info "$GW" --bind "$BIND" >"$GW_LOG" 2>&1) & GW_PID=$!

  wait_http_status "$BASE/healthz" 200 20 || { echo "[!] gateway /healthz not ready"; sed -n '1,120p' "$GW_LOG" || true; exit 1; }
fi

URL="$BASE/o/$ADDR_NOPREFIX/$OBJ_FILE"
echo "[*] Using address: b3:$ADDR_NOPREFIX"
echo "[*] URL: $URL"

# ---- 1) HEAD: ETag + caching headers (case-insensitive) ----
HEADERS="$(mktemp)"
curl -s -D "$HEADERS" -o /dev/null -I "$URL" || { echo "[!] HEAD failed for $URL"; exit 1; }
grep -qi '^etag:'          "$HEADERS" || { echo "[!] Missing ETag"; exit 1; }
grep -qi '^cache-control:' "$HEADERS" || { echo "[!] Missing Cache-Control"; exit 1; }
grep -qi '^vary:'          "$HEADERS" || { echo "[!] Missing Vary"; exit 1; }

ETAG_VAL="$(awk 'tolower($1) ~ /^etag:$/ {sub(/\r/,""); $1=""; sub(/^ /,""); print; exit}' "$HEADERS")"
printf "[*] ETag: %s\n" "$ETAG_VAL"
echo "$ETAG_VAL" | grep -Eq '^"b3:[0-9a-f]+"' || { echo "[!] Bad ETag format (expect \"b3:<hex>\")"; exit 1; }

# ---- 2) If-None-Match -> 304 ----
printf "[*] If-None-Match probe… "
curl -isS -H "If-None-Match: $ETAG_VAL" "$URL" | head -n1 | grep -q ' 304 ' && echo "OK" || { echo "expected 304"; exit 1; }

# ---- 3) Precompressed encodings (best-effort printout) ----
echo "[*] Accept-Encoding: br"
curl -sSI -H 'Accept-Encoding: br' "$URL" | grep -Ei '^(content-encoding|vary|content-length):' || true
echo "[*] Accept-Encoding: zstd"
curl -sSI -H 'Accept-Encoding: zstd' "$URL" | grep -Ei '^(content-encoding|vary|content-length):' || true

# ---- 4) Byte range ----
echo "[*] Range bytes=0-3 (expect 206)"
curl -isS -H 'Range: bytes=0-3' "$URL" | grep -Ei '^(HTTP/1\.1 206|content-range|content-length):' || { echo "[!] Range check failed"; exit 1; }

echo "[OK] http_cache_smoke passed"
