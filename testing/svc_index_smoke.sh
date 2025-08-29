#!/usr/bin/env bash
set -euo pipefail

SOCK="${RON_INDEX_SOCK:-/tmp/ron/svc-index.sock}"

# Fresh, unique DB per run (avoids sled lock contention)
DB_DIR="$(mktemp -d -t ron_index.XXXXXX)"
DB="$DB_DIR/db"
LOG="/tmp/ron_svc_index.log"

ADDR="b3:d6c737c0e427638310a70961bdfde218f7186e4cad459beddb543dd817189571.text"

BUNDLE_DIR="$(mktemp -d -t ron_bundle.XXXXXX)"
echo "dummy" > "${BUNDLE_DIR}/payload.bin"
printf 'version = 1\naddr = "%s"\n' "$ADDR" > "${BUNDLE_DIR}/Manifest.toml"

mkdir -p "$(dirname "$SOCK")" "$(dirname "$DB")"

cleanup() {
  echo "[*] Stopping svc-index (${SVC_PID:-not-started})"
  if [[ -n "${SVC_PID:-}" ]]; then kill "$SVC_PID" 2>/dev/null || true; fi
  rm -f "$SOCK"
  rm -rf "$DB_DIR" "$BUNDLE_DIR"
}
trap cleanup EXIT

echo "[+] Prebuilding svc-index and ronctl (faster startup)…"
cargo build -q -p svc-index -p ronctl

echo "[+] Starting svc-index… (logs: $LOG)"
: > "$LOG"
(
  RON_INDEX_SOCK="$SOCK" \
  RON_INDEX_DB="$DB" \
  RUST_LOG=info \
  cargo run -q -p svc-index 2>&1 | tee -a "$LOG"
) &
SVC_PID=$!

# Wait for the socket (up to 60s)
echo -n "[+] Waiting for $SOCK "
for i in {1..600}; do
  [[ -S "$SOCK" ]] && { echo; break; }
  printf "."
  sleep 0.1
done
if [[ ! -S "$SOCK" ]]; then
  echo
  echo "[FAIL] svc-index socket did not appear"
  echo "------ svc-index logs ------"
  tail -n +1 "$LOG"
  exit 1
fi

echo "[+] ronctl ping"
cargo run -q -p ronctl -- ping

echo "[+] ronctl resolve (expect NOT FOUND pre-put)"
PRE=$(cargo run -q -p ronctl -- resolve "$ADDR" || true)
echo "    -> $PRE"

echo "[+] ronctl put (insert mapping)"
cargo run -q -p ronctl -- put "$ADDR" "$BUNDLE_DIR"

echo "[+] ronctl resolve (should equal bundle dir)"
RES=$(cargo run -q -p ronctl -- resolve "$ADDR")
echo "    -> $RES"

# Normalize paths using Python (portable on macOS)
py_realpath() {
  python3 - <<'PY' "$1"
import os, sys
print(os.path.realpath(sys.argv[1]))
PY
}

RES_NORM=$(py_realpath "$RES")
DIR_NORM=$(py_realpath "$BUNDLE_DIR")

if [[ "$RES_NORM" == "$DIR_NORM" ]]; then
  echo "[PASS] svc-index end-to-end OK"
  exit 0
else
  echo "[FAIL] resolved path != bundle dir"
  echo "       RES= $RES_NORM"
  echo "       DIR= $DIR_NORM"
  echo "------ svc-index logs ------"
  tail -n +50 "$LOG"
  exit 2
fi
