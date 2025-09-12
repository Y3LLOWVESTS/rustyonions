#!/usr/bin/env bash
set -euo pipefail

# 60-second smoke test for index → gateway path
# - Packs a file with tldctl (unique temp index/store to avoid Sled locks)
# - Starts gateway (now via svc-overlay) and parses its bind URL from stdout
# - Fetches Manifest.toml and payload.bin
# - Cleans up the gateway + any temp services this script started
#
# Example test: cargo build -q -p tldctl -p svc-index -p svc-storage -p svc-overlay -p gateway
# chmod +x testing/smoke_index.sh
# DEBUG_BIND=1 testing/smoke_index.sh
#
# Prefer shared helpers if present; otherwise keep tiny local fallbacks.
if [ -f testing/lib/ready.sh ]; then
  # shellcheck source=/dev/null
  source testing/lib/ready.sh
else
  wait_udsocket() { # path [timeout]
    local p="$1" t="${2:-15}" end=$((SECONDS+t))
    while [ $SECONDS -lt $end ]; do [ -S "$p" ] && return 0; sleep 0.2; done  # allow-sleep
    return 1
  }
fi

# ---- tiny, portable helpers for this script ----
strip_ansi_and_extract_hostport() {
  # Reads a log file and prints the first HOST:PORT found in either form (after stripping ANSI):
  #   1) "listening on http://HOST:PORT"
  #   2) "gateway listening local=HOST:PORT"
  # Uses awk with \033 escapes to drop color codes.
  local file="$1"
  awk '
    { line=$0; gsub(/\033\[[0-9;]*[A-Za-z]/,"",line); }
    match(line, /http:\/\/[0-9.]+:[0-9]+/) { s=substr(line, RSTART, RLENGTH); sub(/^http:\/\//,"",s); print s; exit }
    match(line, /local=[0-9.]+:[0-9]+/)    { s=substr(line, RSTART, RLENGTH); sub(/^local=/,   "",s); print s; exit }
  ' "$file" 2>/dev/null || true
}

get_hostport_from_lsof() {
  # macOS: AND filters with -a so we only see LISTEN sockets for *this* PID.
  command -v lsof >/dev/null 2>&1 || return 1
  local pid="$1"
  lsof -nP -a -p "$pid" -iTCP -sTCP:LISTEN 2>/dev/null | awk 'NR>1 {print $9; exit}' | sed 's/ (LISTEN)$//' || true
}

wait_for_bind() {
  # Polls log (ANSI-stripped) and then lsof up to timeout; prints HOST:PORT on success.
  local file="$1" pid="$2" timeout="${3:-20}"
  local end=$((SECONDS+timeout))
  while [ $SECONDS -lt $end ]; do
    local hp
    hp="$(strip_ansi_and_extract_hostport "$file")"
    if [ -n "${hp:-}" ]; then printf '%s\n' "$hp"; return 0; fi
    hp="$(get_hostport_from_lsof "$pid" || true)"
    if [ -n "${hp:-}" ]; then printf '%s\n' "$hp"; return 0; fi
    sleep 0.2  # allow-sleep
  done
  return 1
}

INPUT="${1:-README.md}"
if [[ ! -f "$INPUT" ]]; then
  echo "[smoke] ERROR: Input file '$INPUT' not found" >&2
  exit 1
fi

IDX="$(mktemp -d)"
STORE="$(mktemp -d)"
LOG="$(mktemp)"
TMP_RUN="$(mktemp -d -t smoke_index_run.XXXXXX)"  # sockets/logs if we need to start services

cleanup() {
  # Kill anything this script started (present variables only)
  [[ -n "${GW_PID:-}"   ]] && kill "$GW_PID"   2>/dev/null || true
  [[ -n "${OVL_PID:-}"  ]] && kill "$OVL_PID"  2>/dev/null || true
  [[ -n "${STO_PID:-}"  ]] && kill "$STO_PID"  2>/dev/null || true
  [[ -n "${IDX_PID:-}"  ]] && kill "$IDX_PID"  2>/dev/null || true
  # Wait to avoid zombies
  [[ -n "${GW_PID:-}"   ]] && wait "$GW_PID"   2>/dev/null || true
  [[ -n "${OVL_PID:-}"  ]] && wait "$OVL_PID"  2>/dev/null || true
  [[ -n "${STO_PID:-}"  ]] && wait "$STO_PID"  2>/dev/null || true
  [[ -n "${IDX_PID:-}"  ]] && wait "$IDX_PID"  2>/dev/null || true
  rm -f "$LOG"
  # Leave IDX/STORE for post-mortem; uncomment to auto-remove:
  # rm -rf "$IDX" "$STORE"
  rm -rf "$TMP_RUN"
}
trap cleanup EXIT

echo "[smoke] Index dir:  $IDX"
echo "[smoke] Store dir:  $STORE"

ADDR="$(target/debug/tldctl pack \
  --tld text \
  --input "$INPUT" \
  --index-db "$IDX" \
  --store-root "$STORE")"
echo "[smoke] Packed address: $ADDR"

# Use address without the b3: prefix in URLs
ADDR_NOPREFIX="${ADDR#b3:}"

# ---- ensure an overlay is available (start a tiny local stack iff needed) ----
OVL_SOCK_USE="${RON_OVERLAY_SOCK:-}"
if [ -z "$OVL_SOCK_USE" ]; then
  echo "[smoke] Starting minimal services (index/storage/overlay) for the gateway"

  mkdir -p "$TMP_RUN"/{sock,log}
  IDX_SOCK="$TMP_RUN/sock/index.sock"
  STO_SOCK="$TMP_RUN/sock/storage.sock"
  OVL_SOCK="$TMP_RUN/sock/overlay.sock"

  IDX_LOG="$TMP_RUN/log/index.log"
  STO_LOG="$TMP_RUN/log/storage.log"
  OVL_LOG="$TMP_RUN/log/overlay.log"

  # Binaries
  IDX_BIN="${IDX_BIN:-target/debug/svc-index}"
  STO_BIN="${STO_BIN:-target/debug/svc-storage}"
  OVL_BIN="${OVL_BIN:-target/debug/svc-overlay}"

  # Start services
  ( RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$IDX" RUST_LOG=info "$IDX_BIN" >"$IDX_LOG" 2>&1 ) & IDX_PID=$!
  ( RON_STORAGE_SOCK="$STO_SOCK"              RUST_LOG=info "$STO_BIN" >"$STO_LOG" 2>&1 ) & STO_PID=$!
  ( RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG=info "$OVL_BIN" >"$OVL_LOG" 2>&1 ) & OVL_PID=$!

  # Wait for overlay UDS
  wait_udsocket "$OVL_SOCK" 20 || { echo "[smoke] ERROR: overlay UDS not ready"; sed -n '1,200p' "$OVL_LOG" || true; exit 1; }

  OVL_SOCK_USE="$OVL_SOCK"
fi

echo "[smoke] Starting gateway..."
# Capture stdout/stderr to a log so we can parse the bind URL
RON_OVERLAY_SOCK="$OVL_SOCK_USE" target/debug/gateway >"$LOG" 2>&1 &
GW_PID=$!

[ "${DEBUG_BIND:-0}" = "1" ] && echo "[smoke] Gateway log: $LOG"

# Wait for gateway bind (log first, lsof fallback), robust to ANSI codes
if ! HOSTPORT="$(wait_for_bind "$LOG" "$GW_PID" 20)"; then
  echo "[smoke] ERROR: Could not detect gateway bind address from logs or lsof:"
  sed -n '1,200p' "$LOG" >&2 || true
  if command -v lsof >/dev/null 2>&1; then
    echo "[smoke] lsof snapshot:" >&2
    lsof -nP -a -p "$GW_PID" -iTCP -sTCP:LISTEN >&2 || true
  fi
  exit 1
fi

BASE_URL="http://$HOSTPORT"
echo "[smoke] Gateway: $BASE_URL"

# Verify Manifest.toml
echo "[smoke] Fetching manifest..."
curl -sSf "$BASE_URL/o/$ADDR_NOPREFIX/Manifest.toml" | head -n 10

# Verify payload.bin bytes flow
echo "[smoke] Fetching payload (first 64 bytes)…"
curl -sSf "$BASE_URL/o/$ADDR_NOPREFIX/payload.bin" | head -c 64 | hexdump -C

echo "[smoke] ✅ smoke test completed"
