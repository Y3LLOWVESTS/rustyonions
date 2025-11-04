#!/usr/bin/env bash
set -euo pipefail

: "${BIND_ADDR:=127.0.0.1:5304}"
: "${SVC_GATEWAY_DEV_ROUTES:=1}"          # enable /dev/echo
: "${SVC_GATEWAY_MAX_BODY_BYTES:=1024}"   # 1 KiB limit for the test
: "${RUST_LOG:=info,svc_gateway=debug}"

echo "killing old gateway (if any)…"
pkill -f svc-gateway || true
sleep 0.2

echo "start gateway with body cap..."
log="$(mktemp -t svc-gateway-test.XXXXXX.log)"
BIND_ADDR="$BIND_ADDR" \
SVC_GATEWAY_DEV_ROUTES="$SVC_GATEWAY_DEV_ROUTES" \
SVC_GATEWAY_MAX_BODY_BYTES="$SVC_GATEWAY_MAX_BODY_BYTES" \
RUST_LOG="$RUST_LOG" \
cargo run -p svc-gateway >"$log" 2>&1 &
PID=$!
echo "PID: $PID (logs: $log)"

# wait for /healthz up, but also detect early process death
att=0
code=""
while [ $att -lt 100 ]; do
  if ! kill -0 "$PID" 2>/dev/null; then
    echo "gateway process exited early. recent log:"
    tail -n 80 "$log" || true
    exit 1
  fi
  code="$(curl -s -o /dev/null -w "%{http_code}" "http://${BIND_ADDR}/healthz" || true)"
  [ "$code" = "200" ] && break
  att=$((att+1))
  sleep 0.1
done
if [ "$code" != "200" ]; then
  echo "gateway not up (last code: ${code:-none}). recent log:"
  tail -n 80 "$log" || true
  exit 1
fi

echo "== small ok =="
printf 'hi' | curl -s -i -X POST --data-binary @- "http://${BIND_ADDR}/dev/echo" | sed -n '1,10p'

echo "== too big =="
bigfile="$(mktemp)"
head -c 2048 </dev/zero | tr '\0' 'A' > "$bigfile"
curl -s -i -X POST --data-binary @"$bigfile" "http://${BIND_ADDR}/dev/echo" | sed -n '1,10p'
rm -f "$bigfile"

echo "== metrics =="
curl -s "http://${BIND_ADDR}/metrics" | grep -E 'gateway_rejections_total\{reason="body_cap"\}|gateway_ready_' | sed -n '1,50p'

echo "Done. To stop: kill $PID"
