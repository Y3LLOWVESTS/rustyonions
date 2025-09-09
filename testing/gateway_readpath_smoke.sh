#!/usr/bin/env bash
# testing/gateway_readpath_smoke.sh — end-to-end stack:
# pack → svc-index (UDS) → svc-storage (UDS) → svc-overlay (UDS) → gateway (HTTP) → GET
#
# Usage:
#   RON_INDEX_DB=/tmp/ron.index ./testing/gateway_readpath_smoke.sh
#
# Optional env:
#   ROOT=. OUT_DIR=.onions TLD=text BIND=127.0.0.1:9080
set -euo pipefail

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
[[ -x "$GW" ]] || need_build=true
[[ -x "$IDX" ]] || need_build=true
[[ -x "$STO" ]] || need_build=true
[[ -x "$OVL" ]] || need_build=true

if $need_build; then
  echo "[*] Building tldctl + services + gateway (debug)"
  cargo build -q -p tldctl -p svc-index -p svc-storage -p svc-overlay -p gateway
fi

[[ -x "$TLDCTL" ]] || { echo "tldctl not found at $TLDCTL"; exit 1; }
[[ -x "$GW" ]] || { echo "gateway not found at $GW"; exit 1; }
[[ -x "$IDX" ]] || { echo "svc-index not found at $IDX"; exit 1; }
[[ -x "$STO" ]] || { echo "svc-storage not found at $STO"; exit 1; }
[[ -x "$OVL" ]] || { echo "svc-overlay not found at $OVL"; exit 1; }

echo "[*] Using index db: $RON_INDEX_DB"
mkdir -p "$(dirname "$RON_INDEX_DB")" "$OUT_DIR"

# Create input and pack
TMP_DIR="$(mktemp -d -t ron_readpath.XXXXXX)"
INPUT="$TMP_DIR/payload.bin"
printf "hello rusty onions\n" > "$INPUT"

echo "[*] Packing a sample bundle (tld=$TLD)"
PACK_OUT="$(RON_INDEX_DB="$RON_INDEX_DB" "$TLDCTL" pack   --tld "$TLD"   --input "$INPUT"   --index-db "$RON_INDEX_DB"   --store-root "$OUT_DIR"   | tee "$TMP_DIR/pack.out")" || { echo "tldctl pack failed"; exit 1; }
ADDR="$(printf "%s\n" "$PACK_OUT" | grep -Eo 'b3:[0-9a-f]{8,}\.[A-Za-z0-9._-]+' | tail -n1)"
[[ -n "$ADDR" ]] || { echo "Could not parse address from tldctl output"; exit 1; }
ADDR_NOPREFIX="${ADDR#b3:}"
echo "packed: $ADDR_NOPREFIX"

# Socket paths for services
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

echo "[*] Starting svc-index @ $IDX_SOCK"
(RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG=info "$IDX" >"$IDX_LOG" 2>&1) & IDX_PID=$!

echo "[*] Starting svc-storage @ $STO_SOCK"
(RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$STO" >"$STO_LOG" 2>&1) & STO_PID=$!

echo "[*] Starting svc-overlay @ $OVL_SOCK (-> index=$IDX_SOCK, storage=$STO_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$OVL" >"$OVL_LOG" 2>&1) & OVL_PID=$!

sleep 1

echo "[*] Starting gateway on $BIND (RON_OVERLAY_SOCK=$OVL_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RUST_LOG=info "$GW" --bind "$BIND" >"$GW_LOG" 2>&1) & GW_PID=$!

URL="http://$BIND/o/$ADDR_NOPREFIX/Manifest.toml"
echo "[*] Waiting for gateway to serve: $URL"
for i in $(seq 1 60); do
  if curl -fsS "$URL" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

echo "[*] GET the manifest"
if ! curl -fsS "$URL" | sed -n '1,50p'; then
  echo
  echo "[!] Request failed. Logs below:"
  echo "---- gateway ----"; tail -n +1 "$GW_LOG" | sed -n '1,120p'
  echo "---- overlay ----"; tail -n +1 "$OVL_LOG" | sed -n '1,120p'
  echo "---- storage ----"; tail -n +1 "$STO_LOG" | sed -n '1,120p'
  echo "---- index   ----"; tail -n +1 "$IDX_LOG" | sed -n '1,120p'
  exit 1
fi

echo "[*] OK (see logs in $LOG_DIR)"
