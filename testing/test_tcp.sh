#!/usr/bin/env bash
set -euo pipefail

# --- config / args ---
BASE_CFG="${1:-config.sample.toml}"
RUN_ID="$(date +%s)-$$"
TMP_DIR=".testrun.tcp.$RUN_ID"
TMP_CFG="$TMP_DIR/config.testrun.toml"
LOG=".tcp_server.$RUN_ID.log"

OVERLAY_ADDR_DEFAULT="127.0.0.1:1777"   # matches default in config
METRICS_ADDR_DEFAULT="127.0.0.1:2888"   # matches default in config

mkdir -p "$TMP_DIR"

# --- derive a unique data_dir in a temp config (preserves your behavior) ---
awk -v dd="$TMP_DIR/data" '
  BEGIN{printed=0}
  /^data_dir[[:space:]]*=/ { print "data_dir = \"" dd "\""; printed=1; next }
  { print }
  END{ if (!printed) print "data_dir = \"" dd "\"" }
' "$BASE_CFG" > "$TMP_CFG"

echo "[tcp] Using temp config: $TMP_CFG"
echo "[tcp] Logs: $LOG"

# --- helpers ---
is_listening() {
  local port="$1"
  # Prefer lsof (macOS), else netcat, else bash /dev/tcp
  if command -v lsof >/dev/null 2>&1; then
    lsof -n -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1
  elif command -v nc >/dev/null 2>&1; then
    nc -z localhost "$port" >/dev/null 2>&1
  else
    (echo > "/dev/tcp/127.0.0.1/$port") >/dev/null 2>&1
  fi
}


wait_for_port() {
  local port="$1"
  for _ in {1..50}; do
    if is_listening "$port"; then return 0; fi
    sleep 0.1
  done
  return 1
}

cleanup() {
  echo
  echo "[tcp] stopping server…"
  if [[ -n "${SRV_PID:-}" ]]; then
    kill "$SRV_PID" 2>/dev/null || true
    wait "$SRV_PID" 2>/dev/null || true
  fi
  echo "[tcp] cleaning temp dir $TMP_DIR"
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# --- start server in background (same as yours, but wait for port) ---
RUST_LOG=info \
cargo run -p node -- --config "$TMP_CFG" serve --transport tcp \
  > "$LOG" 2>&1 &
SRV_PID=$!

# Extract ports from config if present, otherwise use defaults
OVERLAY_PORT="$(grep -E '^\s*overlay_addr\s*=' "$TMP_CFG" | sed -E 's/.*:([0-9]+).*/\1/' || true)"
METRICS_PORT="$(grep -E '^\s*dev_inbox_addr\s*=' "$TMP_CFG" | sed -E 's/.*:([0-9]+).*/\1/' || true)"
OVERLAY_PORT="${OVERLAY_PORT:-${OVERLAY_ADDR_DEFAULT##*:}}"
METRICS_PORT="${METRICS_PORT:-${METRICS_ADDR_DEFAULT##*:}}"

if ! wait_for_port "$OVERLAY_PORT"; then
  echo "❌ Server did not start listening on :$OVERLAY_PORT in time."
  echo "---- server log ----"
  tail -n +1 "$LOG" || true
  echo "--------------------"
  exit 1
fi
echo "✅ Server listening on :$OVERLAY_PORT"

# --- Put/Get round-trip (hash on stdout, stats on stderr) ---
IN="$(mktemp -t ro_msg)"
OUT="$(mktemp -t ro_out)"
echo "framed and flawless" > "$IN"

echo "[tcp] PUT: $IN"
HASH="$(cargo run -p node -- --config "$TMP_CFG" put --transport tcp "$IN" 2>/dev/null | tail -n1)"
if [[ -z "$HASH" ]]; then
  echo "❌ No hash returned from PUT. Aborting."
  echo "---- server log (tail) ----"
  tail -n 80 "$LOG" || true
  echo "---------------------------"
  exit 1
fi
echo "[tcp] put hash: $HASH"

echo "[tcp] GET: $HASH -> $OUT"
cargo run -p node -- --config "$TMP_CFG" get --transport tcp "$HASH" "$OUT" >/dev/null

if diff -u "$IN" "$OUT" >/dev/null; then
  echo "[tcp] ✅ PUT/GET OK"
else
  echo "[tcp] ❌ PUT/GET mismatch!"
  exit 1
fi

# --- Show recent stats/logs (same filter as yours) ---
echo "[tcp] recent stats/logs:"
tail -n 50 "$LOG" | grep -E 'stats/|listening|overlay' || tail -n 50 "$LOG" || true

# --- Optional: ping metrics endpoint if available ---
if command -v curl >/dev/null 2>&1; then
  echo "[tcp] metrics (if enabled):"
  curl -s "http://127.0.0.1:${METRICS_PORT}/metrics.json" || echo "(metrics not reachable)"
fi

echo "[tcp] 🎉 Done."
