#!/usr/bin/env bash
set -euo pipefail

# Config knobs (override when calling the script)
: "${BIND_ADDR:=127.0.0.1:5304}"
: "${SVC_GATEWAY_DEV_READY:=1}"
: "${SVC_GATEWAY_READY_TIMEOUT_MS:=200}"
: "${RUST_LOG:=info,svc_gateway=debug}"

echo "fmt + clippy + build…"
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway

echo "killing old svc-gateway (if any)…"
pkill -f svc-gateway || true

echo "starting svc-gateway…"
BIND_ADDR="$BIND_ADDR" \
SVC_GATEWAY_DEV_READY="$SVC_GATEWAY_DEV_READY" \
SVC_GATEWAY_READY_TIMEOUT_MS="$SVC_GATEWAY_READY_TIMEOUT_MS" \
RUST_LOG="$RUST_LOG" \
cargo run -p svc-gateway >/dev/null 2>&1 &

PID=$!
echo "PID: $PID"

# --- wait until /healthz returns 200 (up to ~10s) ---
echo -n "waiting for /healthz… "
att=0
until [ $att -ge 100 ]; do
  code="$(curl -s -o /dev/null -w "%{http_code}" "http://${BIND_ADDR}/healthz" || true)"
  if [ "$code" = "200" ]; then
    echo "ready."
    break
  fi
  att=$((att+1))
  sleep 0.1
done
if [ "$code" != "200" ]; then
  echo "timeout waiting for gateway to start (last code: ${code:-none})"
  echo "tip: check 'cargo run' logs without redirect if debugging startup."
  # Don't kill the process here; it may still be starting. Exit non-zero for CI clarity.
  exit 1
fi

echo "healthz:"
curl -si "http://${BIND_ADDR}/healthz" | head -n 5

echo "readyz:"
curl -si "http://${BIND_ADDR}/readyz" | head -n 5

echo "metrics (first lines):"
curl -s "http://${BIND_ADDR}/metrics" | head -n 10

echo "Done. Gateway is running (PID $PID). To stop: kill $PID"
