#!/usr/bin/env bash
set -euo pipefail

: "${BIND_ADDR:=127.0.0.1:5304}"
: "${SVC_GATEWAY_DEV_ROUTES:=1}"
: "${SVC_GATEWAY_RL_RPS:=3}"
: "${SVC_GATEWAY_RL_BURST:=5}"
: "${RUST_LOG:=info,svc_gateway=debug}"

echo "killing old gateway (if any)…"
pkill -f svc-gateway || true
sleep 0.2

echo "start gateway with RL rps=$SVC_GATEWAY_RL_RPS burst=$SVC_GATEWAY_RL_BURST..."
log="$(mktemp -t svc-gateway-rl.XXXXXX.log)"
BIND_ADDR="$BIND_ADDR" \
SVC_GATEWAY_DEV_ROUTES="$SVC_GATEWAY_DEV_ROUTES" \
SVC_GATEWAY_RL_RPS="$SVC_GATEWAY_RL_RPS" \
SVC_GATEWAY_RL_BURST="$SVC_GATEWAY_RL_BURST" \
RUST_LOG="$RUST_LOG" \
cargo run -p svc-gateway >"$log" 2>&1 &

PID=$!
echo "PID: $PID (logs: $log)"

# wait for up
att=0; code=""
while [ $att -lt 100 ]; do
  if ! kill -0 "$PID" 2>/dev/null; then tail -n 80 "$log" || true; exit 1; fi
  code="$(curl -s -o /dev/null -w "%{http_code}" "http://${BIND_ADDR}/healthz" || true)"
  [ "$code" = "200" ] && break
  att=$((att+1)); sleep 0.1
done
[ "$code" = "200" ] || { echo "gateway not up"; tail -n 80 "$log" || true; exit 1; }

echo "bursting /dev/rl…"
ok=0; rl=0
for i in $(seq 1 20); do
  c="$(curl -s -o /dev/null -w "%{http_code}" "http://${BIND_ADDR}/dev/rl")"
  if [ "$c" = "200" ]; then ok=$((ok+1)); else rl=$((rl+1)); fi
done
echo "OK: $ok  RL(429): $rl"

echo "metrics:"
curl -s "http://${BIND_ADDR}/metrics" | grep -E 'gateway_rejections_total\{reason="rate_limit"\}' || true

echo "Done. To stop: kill $PID"
