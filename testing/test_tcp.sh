#!/usr/bin/env bash
set -euo pipefail

BASE_CFG="${1:-config.sample.toml}"
RUN_ID="$(date +%s)-$$"
TMP_DIR=".testrun.tcp.$RUN_ID"
TMP_CFG="$TMP_DIR/config.testrun.toml"
LOG=".tcp_server.$RUN_ID.log"

mkdir -p "$TMP_DIR"

# Create a temp config with a unique data_dir
awk -v dd="$TMP_DIR/data" '
  BEGIN{printed=0}
  /^data_dir[[:space:]]*=/ {
    print "data_dir = \"" dd "\""; printed=1; next
  }
  {print}
  END{if (!printed) print "data_dir = \"" dd "\""}
' "$BASE_CFG" > "$TMP_CFG"

echo "[tcp] Using temp config: $TMP_CFG"
echo "[tcp] Logs: $LOG"

cleanup() {
  echo
  echo "[tcp] stopping server…"
  kill $SRV_PID 2>/dev/null || true
  wait $SRV_PID 2>/dev/null || true
  echo "[tcp] cleaning temp dir $TMP_DIR"
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Start server (TCP) in background, log to file
RUST_LOG=info \
cargo run -p node -- --config "$TMP_CFG" serve --transport tcp \
  > "$LOG" 2>&1 &
SRV_PID=$!

# Give server a moment
sleep 1

# Put/Get round-trip
echo "framed and flawless" > /tmp/ro_msg.txt
HASH=$(cargo run -p node -- --config "$TMP_CFG" put --transport tcp /tmp/ro_msg.txt | tail -n1)
echo "[tcp] put hash: $HASH"
cargo run -p node -- --config "$TMP_CFG" get --transport tcp "$HASH" /tmp/ro_msg_out.txt
diff -u /tmp/ro_msg.txt /tmp/ro_msg_out.txt && echo "[tcp] PUT/GET OK ✅"

# Show latest useful lines from server
echo "[tcp] recent stats/logs:"
tail -n 50 "$LOG" | grep -E 'stats/|listening|overlay' || tail -n 50 "$LOG" || true
