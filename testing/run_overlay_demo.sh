#!/usr/bin/env bash
# Tiny manual demo: starts a server, PUTs a file, GETs it back.
set -euo pipefail

BIN="target/debug/ronode"
CFG="config.json"

if [[ ! -x "$BIN" ]]; then
  echo "Building ronode…"
  cargo build -p node
fi

echo "Starting server in background…"
$BIN serve --config "$CFG" &
PID=$!
trap "kill $PID >/dev/null 2>&1 || true" EXIT

sleep 0.5
echo "Putting README.md…"
HASH=$($BIN put --config "$CFG" ./README.md)
echo "Hash: $HASH"

echo "Getting to /tmp/roundtrip.out…"
$BIN get --config "$CFG" "$HASH" /tmp/roundtrip.out
diff -q README.md /tmp/roundtrip.out && echo "Roundtrip OK"
