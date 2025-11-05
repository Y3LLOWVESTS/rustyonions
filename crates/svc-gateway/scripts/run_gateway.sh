#!/usr/bin/env bash
set -euo pipefail

# Minimal runner that builds, starts svc-gateway, waits for /healthz,
# prints first metrics lines, and writes the PID to ./target/gateway.pid.

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
CRATE_DIR="$ROOT/crates/svc-gateway"
PID_FILE="$ROOT/target/gateway.pid"
LOG_DIR="$ROOT/target/run-logs"
LOG_FILE="$LOG_DIR/gateway.log"

mkdir -p "$LOG_DIR"

echo "fmt + clippy + build…"
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway

echo "killing old svc-gateway (if any)…"
if [[ -f "$PID_FILE" ]]; then
  OLD_PID="$(cat "$PID_FILE" || true)"
  if [[ -n "${OLD_PID:-}" ]] && ps -p "$OLD_PID" >/dev/null 2>&1; then
    kill "$OLD_PID" || true
    sleep 0.2
  fi
  rm -f "$PID_FILE"
fi
pkill -f 'svc-gateway' 2>/dev/null || true

echo "starting svc-gateway…"
# Honor caller-provided env like SVC_GATEWAY_DEV_ROUTES etc.
RUST_LOG="${RUST_LOG:-info,svc_gateway=debug}" \
cargo run -p svc-gateway >"$LOG_FILE" 2>&1 &

GWPID=$!
echo "$GWPID" > "$PID_FILE"
echo "PID: $GWPID (logs: $LOG_FILE)"

# Short grace before first probe (avoid noisy connection-refused)
sleep 1

# Wait until /healthz answers (quiet probes)
printf "waiting for /healthz… "
for i in {1..60}; do
  if curl -fsS --max-time 0.2 http://127.0.0.1:5304/healthz >/dev/null 2>&1; then
    echo "ready."
    break
  fi
  sleep 0.2
  if ! ps -p "$GWPID" >/dev/null 2>&1; then
    echo "gateway exited early; last 60 lines:"
    tail -n 60 "$LOG_FILE" || true
    exit 1
  fi
done

echo "healthz:"
curl -is http://127.0.0.1:5304/healthz | sed -n '1,12p'

echo "readyz:"
curl -is http://127.0.0.1:5304/readyz  | sed -n '1,12p' || true

echo
echo "metrics (first lines):"
curl -s http://127.0.0.1:5304/metrics | head -n 12 || true

echo "Done. Gateway is running (PID $(cat "$PID_FILE")). To stop: crates/svc-gateway/scripts/stop_gateway.sh"
