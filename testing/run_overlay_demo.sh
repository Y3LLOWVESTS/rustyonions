#!/usr/bin/env bash
# Tiny manual demo: starts a server, PUTs a file, GETs it back.
set -euo pipefail

CFG="${1:-config.sample.toml}"
LOG=".demo_tcp.$(date +%s)-$$.log"

echo "[demo] building ronode…"
cargo build -p node

echo "[demo] starting server in background (cfg: $CFG)…"
RUST_LOG=info cargo run -p node -- --config "$CFG" serve --transport tcp >"$LOG" 2>&1 &
PID=$!
trap 'kill "$PID" >/dev/null 2>&1 || true' EXIT

# Wait for listener line
if ! (timeout 5 grep -q "listening on 127.0.0.1:1777" <(tail -n +1 -f "$LOG")); then
  echo "[demo] server did not start; recent log:"
  tail -n 80 "$LOG" || true
  exit 1
fi

echo "[demo] putting README.md…"
HASH=$(cargo run -p node -- put README.md | tail -n1)
echo "[demo] hash: $HASH"

OUT="/tmp/roundtrip.out"
echo "[demo] getting to $OUT…"
cargo run -p node -- get "$HASH" "$OUT"
diff -q README.md "$OUT" && echo "[demo] roundtrip OK ✅"

echo "[demo] metrics (if running):"
curl -s http://127.0.0.1:2888/metrics.json || true
