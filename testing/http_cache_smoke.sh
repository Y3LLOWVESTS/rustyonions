#!/usr/bin/env bash
# testing/http_cache_smoke.sh — build → pack → start stack → run HTTP cache tests
# Requires: curl, ripgrep (rg) recommended (falls back to grep -E)
#
# Env (override as needed):
#   RON_INDEX_DB=/tmp/ron.index
#   OUT_DIR=.onions
#   BIND=127.0.0.1:9080
#   RUST_LOG=info
#   HOLD=1   # optional: keep stack running after tests until Ctrl-C

set -euo pipefail

ROOT="${ROOT:-.}"
RON_INDEX_DB="${RON_INDEX_DB:-/tmp/ron.index}"
OUT_DIR="${OUT_DIR:-.onions}"
BIND="${BIND:-127.0.0.1:9080}"
RUST_LOG="${RUST_LOG:-info}"
HOLD="${HOLD:-0}"

TLDCTL="${TLDCTL:-$ROOT/target/debug/tldctl}"
IDX="${IDX:-$ROOT/target/debug/svc-index}"
STO="${STO:-$ROOT/target/debug/svc-storage}"
OVL="${OVL:-$ROOT/target/debug/svc-overlay}"
GW="${GW:-$ROOT/target/debug/gateway}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing tool: $1"; exit 1; }; }
need curl

echo "[*] Building components (debug)"
cargo build -q -p tldctl -p svc-index -p svc-storage -p svc-overlay -p gateway

for bin in "$TLDCTL" "$IDX" "$STO" "$OVL" "$GW"; do
  [[ -x "$bin" ]] || { echo "missing binary: $bin"; exit 1; }
done

TMP_DIR="$(mktemp -d -t ron_http_cache.XXXXXX)"
RUN_DIR="$TMP_DIR/run"
LOG_DIR="$TMP_DIR/logs"
mkdir -p "$RUN_DIR" "$LOG_DIR" "$(dirname "$RON_INDEX_DB")" "$OUT_DIR"

IDX_SOCK="$RUN_DIR/svc-index.sock"
STO_SOCK="$RUN_DIR/svc-storage.sock"
OVL_SOCK="$RUN_DIR/svc-overlay.sock"

cleanup() {
  [[ "$HOLD" == "1" ]] && return
  echo
  echo "[*] Shutting down…"
  [[ -n "${GW_PID:-}"  ]] && kill "$GW_PID"  2>/dev/null || true
  [[ -n "${OVL_PID:-}" ]] && kill "$OVL_PID" 2>/dev/null || true
  [[ -n "${STO_PID:-}" ]] && kill "$STO_PID" 2>/dev/null || true
  [[ -n "${IDX_PID:-}" ]] && kill "$IDX_PID" 2>/dev/null || true
  echo "[*] Logs: $LOG_DIR"
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# ---- PACK (must happen before services to avoid sled lock) ----
INPUT="$TMP_DIR/payload.bin"
printf 'hello rusty onions\n' > "$INPUT"

echo "[*] Packing sample payload (tld=text)"
PACK_OUT="$(RON_INDEX_DB="$RON_INDEX_DB" "$TLDCTL" pack \
  --tld text \
  --input "$INPUT" \
  --index-db "$RON_INDEX_DB" \
  --store-root "$OUT_DIR")"
ADDR="$(printf "%s\n" "$PACK_OUT" | grep -Eo 'b3:[0-9a-f]{8,}\.[A-Za-z0-9._-]+$' | tail -n1)"
[[ -n "$ADDR" ]] || { echo "Could not parse address from tldctl output"; exit 1; }
ADDR_NOPREFIX="${ADDR#b3:}"
URL="http://$BIND/o/$ADDR_NOPREFIX/payload.bin"
echo "[*] Address: $ADDR"
echo "[*] URL:     $URL"

# ---- START SERVICES ----
echo "[*] Starting svc-index @ $IDX_SOCK"
(RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG="$RUST_LOG" "$IDX" >"$LOG_DIR/index.log" 2>&1) & IDX_PID=$!

echo "[*] Starting svc-storage @ $STO_SOCK"
(RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$STO" >"$LOG_DIR/storage.log" 2>&1) & STO_PID=$!

echo "[*] Starting svc-overlay @ $OVL_SOCK"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$OVL" >"$LOG_DIR/overlay.log" 2>&1) & OVL_PID=$!

sleep 1

echo "[*] Starting gateway on $BIND"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$GW" --bind "$BIND" >"$LOG_DIR/gateway.log" 2>&1) & GW_PID=$!

# ---- WAIT UNTIL READY ----
echo "[*] Waiting for gateway readiness"
for i in $(seq 1 60); do
  if curl -fsS "http://$BIND/readyz" >/dev/null 2>&1; then
    break
  fi
  sleep 0.3
done
curl -sS "http://$BIND/healthz" || true
curl -sS "http://$BIND/readyz" || true
echo

# ---- TEST HELPERS ----
filter_headers() {
  local pat="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -n -i "$pat" || true
  else
    grep -Ei "$pat" || true
  fi
}

# ---- TEST 1: HEAD ----
echo "==== [1] HEAD headers (key caching fields) ===="
curl -sSI "$URL" | filter_headers 'HTTP/1\.1|ETag|Cache-Control|Content-Length|Vary|Accept-Ranges|Content-Encoding'
echo

# ---- TEST 2: 304 with If-None-Match ----
echo "==== [2] Conditional 304 path (If-None-Match) ===="
HDRS="$(curl -sSI "$URL" | tr -d '\r')"
if command -v rg >/dev/null 2>&1; then
  ETAG=$(printf "%s\n" "$HDRS" | rg -o '"b3:[^"]+"' | tr -d '"')
else
  ETAG=$(printf "%s\n" "$HDRS" | awk 'BEGIN{IGNORECASE=1} /^ETag:/ {gsub("\r",""); gsub("\"",""); print $2; exit}')
fi
echo "[*] Using ETag: $ETAG"
curl -isS -H "If-None-Match: \"$ETAG\"" "$URL" | head -n1
echo

# ---- TEST 3: Precompressed encodings ----
echo "==== [3] Precompressed selection (br / zstd) ===="
echo "-- br --"
curl -sSI -H 'Accept-Encoding: br' "$URL" | filter_headers 'Content-Encoding|Vary|Content-Length'
echo "-- zstd --"
curl -sSI -H 'Accept-Encoding: zstd' "$URL" | filter_headers 'Content-Encoding|Vary|Content-Length'
echo

# ---- TEST 4: Byte ranges ----
echo "==== [4] Range request bytes=0-3 (expect 206) ===="
curl -isS -H 'Range: bytes=0-3' "$URL" | filter_headers 'HTTP/1\.1 206|Content-Range|Content-Length'
echo

if [[ "$HOLD" == "1" ]]; then
  echo "[*] HOLD=1 set — stack is running. Press Ctrl-C to exit."
  wait
fi
