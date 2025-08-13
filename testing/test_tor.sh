#!/usr/bin/env bash
set -euo pipefail

BASE_CFG="${1:-config.sample.toml}"
HS_KEY="${2:-.data/hs_ed25519_key}"

RUN_ID="$(date +%s)-$$"
TMP_DIR=".testrun.tor.$RUN_ID"
TMP_CFG="$TMP_DIR/config.testrun.toml"
LOG=".tor_server.$RUN_ID.log"

mkdir -p "$TMP_DIR" .data

# Create a temp config with a unique data_dir
awk -v dd="$TMP_DIR/data" '
  BEGIN{printed=0}
  /^data_dir[[:space:]]*=/ {
    print "data_dir = \"" dd "\""; printed=1; next
  }
  {print}
  END{if (!printed) print "data_dir = \"" dd "\""}
' "$BASE_CFG" > "$TMP_CFG"

echo "[tor] Using temp config: $TMP_CFG"
echo "[tor] Logs: $LOG"

cleanup() {
  echo
  echo "[tor] stopping server…"
  kill $SRV_PID 2>/dev/null || true
  wait $SRV_PID 2>/dev/null || true
  echo "[tor] cleaning temp dir $TMP_DIR"
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Start Tor HS server in background; write logs to file
RUST_LOG=info \
cargo run -p node -- --config "$TMP_CFG" serve --transport tor --hs-key-file "$HS_KEY" \
  > "$LOG" 2>&1 &
SRV_PID=$!

# Wait until onion appears in logs (up to ~90s)
echo -n "[tor] waiting for onion address"
ONION=""
for _ in $(seq 1 90); do
  sleep 1
  echo -n "."
  if grep -Eo '[a-z2-7]{56}\.onion:[0-9]+' "$LOG" >/dev/null 2>&1; then
    ONION="$(grep -Eo '[a-z2-7]{56}\.onion:[0-9]+' "$LOG" | head -n1)"
    break
  fi
done
echo
if [[ -z "$ONION" ]]; then
  echo "[tor] ERROR: could not detect onion address in logs"
  echo "[tor] last 80 lines of $LOG:"
  tail -n 80 "$LOG" || true
  exit 1
fi
echo "[tor] onion: $ONION"

# Put/Get round-trip over Tor
echo "tor-borne treasure" > /tmp/ro_msg_tor.txt
HASH=$(cargo run -p node -- --config "$TMP_CFG" put --transport tor --to "$ONION" /tmp/ro_msg_tor.txt | tail -n1)
echo "[tor] put hash: $HASH"
cargo run -p node -- --config "$TMP_CFG" get --transport tor --to "$ONION" "$HASH" /tmp/ro_msg_tor_out.txt
diff -u /tmp/ro_msg_tor.txt /tmp/ro_msg_tor_out.txt && echo "[tor] PUT/GET OK ✅"

# Show latest useful lines from server
echo "[tor] recent stats/logs:"
tail -n 80 "$LOG" | grep -E 'stats/|listening|overlay|onion' || tail -n 80 "$LOG" || true
