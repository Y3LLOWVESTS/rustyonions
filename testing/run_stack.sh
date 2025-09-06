#!/usr/bin/env bash
# testing/run_stack.sh — start full RustyOnions stack and keep it running until Ctrl-C
# Services: svc-index (UDS) → svc-storage (UDS) → svc-overlay (UDS) → gateway (HTTP)
# Env you can override (defaults in parentheses):
#   RON_INDEX_DB (/tmp/ron.index)   # sled DB path for svc-index + pack
#   OUT_DIR (.onions)               # bundle store root used by tldctl + storage
#   BIND (127.0.0.1:9080)           # gateway HTTP bind
#   RUST_LOG (info)                 # rust log level
#
# Usage:
#   chmod +x testing/run_stack.sh
#   RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 testing/run_stack.sh
#
# Notes:
# - Prints log directory on shutdown.
# - Exposes env sockets: RON_INDEX_SOCK, RON_STORAGE_SOCK, RON_OVERLAY_SOCK.
# - With the updated routes.rs, /healthz and /readyz will be available.

### MANUAL TESTING (WITHOUT THIS SCRIPT)
# ------------------------------------------------------------------------------
# RustyOnions Stack — QUICKSTART, ENV, and TROUBLESHOOTING
#
# WHY PACK BEFORE STARTING THE STACK?
# - svc-index uses sled and holds an EXCLUSIVE file lock on RON_INDEX_DB (e.g. /tmp/ron.index/db).
# - If the stack is running, a concurrent `tldctl pack` will fail with:
#     "could not acquire lock ... Resource temporarily unavailable (EWOULDBLOCK)"
# - Therefore: PACK OBJECTS *while the stack is stopped*, then start the stack.
#
# ENV VARS (override as needed):
#   RON_INDEX_DB   # sled DB root for index service and `tldctl` (default: /tmp/ron.index)
#   OUT_DIR        # bundle store root used by `tldctl` and storage service (default: .onions)
#   BIND           # gateway HTTP bind address (default: 127.0.0.1:9080)
#   RUST_LOG       # Rust log level (default: info)
#   RON_QUOTA_RPS  # (optional) requests/sec per tenant for 429 throttling (float)
#   RON_QUOTA_BURST# (optional) token bucket burst size (float)
#
# EXAMPLE SESSION (copy to a terminal, not here):
#   # 1) Pack while the stack is STOPPED (creates an address: b3:<hex>.text)
#   printf 'hello rusty onions\n' > /tmp/payload.bin
#   RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions \
#   target/debug/tldctl pack --tld text --input /tmp/payload.bin \
#     --index-db /tmp/ron.index --store-root .onions
#
#   # 2) Start the stack (stays running until Ctrl-C)
#   RON_QUOTA_RPS=1 RON_QUOTA_BURST=2 \
#   RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 \
#   testing/run_stack.sh
#
#   # 3) In another terminal, fetch the manifest (replace <hex> with your value)
#   URL="http://127.0.0.1:9080/o/<hex>.text/Manifest.toml"
#   curl -sS "$URL"
#
# HEALTH / READY PROBES (with updated routes.rs):
#   curl -sS http://127.0.0.1:9080/healthz
#   curl -sS http://127.0.0.1:9080/readyz
#
# TROUBLESHOOTING:
# - "could not acquire lock on '/tmp/ron.index/db' (EWOULDBLOCK)": svc-index is running.
#   Stop the stack (Ctrl-C), pack with `tldctl`, then restart.
#
# - Multiple old services still running? Inspect:
#     pgrep -fl svc-index
#     pgrep -fl svc-storage
#     pgrep -fl svc-overlay
#     pgrep -fl gateway
#
# - Which process holds the DB lock?
#     lsof /tmp/ron.index/db
#
# - macOS netcat often lacks `-U` for Unix sockets. To test a UDS without `nc -U`, use Python:
#     # paste this into a terminal (edit the socket path):
#     python3 - <<'PY'
#     import socket; p="</absolute/path/to/svc-overlay.sock>"
#     s=socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
#     s.settimeout(0.5)
#     try: s.connect(p); print("OK: connected", p)
#     except Exception as e: print("FAIL:", e)
#     finally: s.close()
#     PY
#
# - Verify quotas reached the gateway:
#     ps eww -p "$(pgrep -n gateway)" | grep -E 'RON_QUOTA_(RPS|BURST)'
#
# NOTES:
# - Addresses are BLAKE3-256: canonical form `b3:<hex>.<tld>`; services verify full digest.
# - OAP/1 `max_frame = 1 MiB`; storage streaming chunk size (e.g., 64 KiB) is an implementation detail.
# ------------------------------------------------------------------------------


set -euo pipefail

ROOT="${ROOT:-.}"
RON_INDEX_DB="${RON_INDEX_DB:-/tmp/ron.index}"
OUT_DIR="${OUT_DIR:-.onions}"
BIND="${BIND:-127.0.0.1:9080}"
RUST_LOG="${RUST_LOG:-info}"

TLDCTL="${TLDCTL:-$ROOT/target/debug/tldctl}"
GW="${GW:-$ROOT/target/debug/gateway}"
IDX="${IDX:-$ROOT/target/debug/svc-index}"
STO="${STO:-$ROOT/target/debug/svc-storage}"
OVL="${OVL:-$ROOT/target/debug/svc-overlay}"

echo "[*] Building components (debug)"
cargo build -q -p tldctl -p svc-index -p svc-storage -p svc-overlay -p gateway

for bin in "$TLDCTL" "$GW" "$IDX" "$STO" "$OVL"; do
  [[ -x "$bin" ]] || { echo "missing binary: $bin"; exit 1; }
done

RUN_DIR="$(mktemp -d -t ron_stack.XXXXXX)"
LOG_DIR="$RUN_DIR/logs"
mkdir -p "$LOG_DIR"
IDX_SOCK="$RUN_DIR/svc-index.sock"
STO_SOCK="$RUN_DIR/svc-storage.sock"
OVL_SOCK="$RUN_DIR/svc-overlay.sock"

echo "[*] RON_INDEX_DB=$RON_INDEX_DB"
echo "[*] OUT_DIR=$OUT_DIR"
echo "[*] RUN_DIR=$RUN_DIR"
mkdir -p "$(dirname "$RON_INDEX_DB")" "$OUT_DIR"

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

echo "[*] Starting svc-index @ $IDX_SOCK"
(RON_INDEX_SOCK="$IDX_SOCK" RON_INDEX_DB="$RON_INDEX_DB" RUST_LOG="$RUST_LOG" "$IDX" >"$LOG_DIR/index.log" 2>&1) & IDX_PID=$!

echo "[*] Starting svc-storage @ $STO_SOCK"
(RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$STO" >"$LOG_DIR/storage.log" 2>&1) & STO_PID=$!

echo "[*] Starting svc-overlay @ $OVL_SOCK (index=$IDX_SOCK, storage=$STO_SOCK)"
(RON_OVERLAY_SOCK="$OVL_SOCK" RON_INDEX_SOCK="$IDX_SOCK" RON_STORAGE_SOCK="$STO_SOCK" RUST_LOG="$RUST_LOG" "$OVL" >"$LOG_DIR/overlay.log" 2>&1) & OVL_PID=$!

sleep 1

echo "[*] Starting gateway on $BIND"
(RON_OVERLAY_SOCK="$OVL_SOCK" RUST_LOG="$RUST_LOG" "$GW" --bind "$BIND" >"$LOG_DIR/gateway.log" 2>&1) & GW_PID=$!

echo "[*] Stack is up"
echo "[*] Try in another terminal:"
echo "    curl -sS http://$BIND/healthz"
echo "    curl -sS http://$BIND/readyz | jq ."
echo
echo "[*] Press Ctrl-C here to stop."
wait
