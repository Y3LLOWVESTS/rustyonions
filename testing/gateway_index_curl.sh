#!/usr/bin/env bash
# testing/gateway_index_curl.sh
# E2E: svc-index + gateway + ronctl + curl (fixed port; no log scraping)
set -euo pipefail

ADDR="${ADDR:-b3:d6c737c0e427638310a70961bdfde218f7186e4cad459beddb543dd817189571.text}"
BUNDLE="${BUNDLE:-$(pwd)/testing/sample_bundle}"   # needs Manifest.toml + payload.bin
SOCK="${SOCK:-/tmp/ron/svc-index.sock}"
PORT="${PORT:-49999}"

DB_DIR="$(mktemp -d)"
DB="$DB_DIR/db"

mkdir -p testing/_out
LOG="${LOG:-testing/_out/gw_index_$(date +%s).log}"

if [ ! -f "$BUNDLE/Manifest.toml" ] || [ ! -f "$BUNDLE/payload.bin" ]; then
  echo "[!] Expected bundle at $BUNDLE with Manifest.toml and payload.bin"
  echo "    Quick scaffold:"
  echo "      mkdir -p $BUNDLE"
  echo "      printf '[payment]\\nrequired=false\\n' > $BUNDLE/Manifest.toml"
  echo "      echo hello > $BUNDLE/payload.bin"
  exit 1
fi

echo "[+] build"
cargo build -q

echo "[+] start svc-index"
RON_INDEX_SOCK="$SOCK" RON_INDEX_DB="$DB" RUST_LOG=svc_index=debug \
  cargo run -q -p svc-index >>"$LOG" 2>&1 &
PID_INDEX=$!

# wait for index socket
for i in {1..200}; do
  [ -S "$SOCK" ] && break
  sleep 0.05
done
if [ ! -S "$SOCK" ]; then
  echo "[!] index socket not found at $SOCK"
  tail -n 200 "$LOG" || true
  kill $PID_INDEX || true
  exit 1
fi

echo "[+] ping svc-index"
RON_INDEX_SOCK="$SOCK" cargo run -q -p ronctl -- ping

echo "[+] put mapping: $ADDR -> $BUNDLE"
RON_INDEX_SOCK="$SOCK" cargo run -q -p ronctl -- put "$ADDR" "$BUNDLE"

echo "[+] resolve mapping (should print the bundle dir)"
RON_INDEX_SOCK="$SOCK" cargo run -q -p ronctl -- resolve "$ADDR"

echo "[+] start gateway on 127.0.0.1:$PORT"
RON_INDEX_SOCK="$SOCK" RUST_LOG=gateway=debug \
  cargo run -q -p gateway -- --bind "127.0.0.1:$PORT" >>"$LOG" 2>&1 &
PID_GATEWAY=$!

# wait for the port to be open (netcat or curl HEAD)
for i in {1..200}; do
  if nc -z 127.0.0.1 "$PORT" 2>/dev/null; then break; fi
  sleep 0.05
done
if ! nc -z 127.0.0.1 "$PORT" 2>/dev/null; then
  echo "[!] gateway did not open 127.0.0.1:$PORT"
  tail -n 200 "$LOG" || true
  kill $PID_GATEWAY $PID_INDEX 2>/dev/null || true
  rm -rf "$DB_DIR" 2>/dev/null || true
  exit 1
fi

URL="http://127.0.0.1:$PORT/o/$ADDR/payload.bin"
echo "[+] curl $URL"
set +e
HTTP_OUT=$(curl -isS "$URL")
RC=$?
set -e
echo "$HTTP_OUT"

echo
echo "[*] logs at $LOG"
echo "===== last 200 lines ====="
tail -n 200 "$LOG" || true

echo "[*] stop processes"
kill $PID_GATEWAY $PID_INDEX 2>/dev/null || true
wait $PID_GATEWAY 2>/dev/null || true
wait $PID_INDEX 2>/dev/null || true

rm -rf "$DB_DIR" 2>/dev/null || true

exit $RC
