#!/usr/bin/env bash
set -euo pipefail

: "${BIND_ADDR:=127.0.0.1:5304}"
: "${PAR:=20}"                 # concurrent requests
: "${REQ_TIMEOUT:=2}"          # seconds per HTTP request max
: "${CONNECT_TIMEOUT:=1}"

# Force dev routes on regardless of shell env
export SVC_GATEWAY_DEV_ROUTES=1
export SVC_GATEWAY_DEV=1

: "${SVC_GATEWAY_RL_RPS:=3}"
: "${SVC_GATEWAY_RL_BURST:=5}"
: "${SVC_GATEWAY_RL_TARPIT_MS:=0}"
: "${RUST_LOG:=info,svc_gateway=debug}"

echo "killing old gateway (if any)…"
pkill -f svc-gateway || true
sleep 0.2

echo "start gateway with RL rps=${SVC_GATEWAY_RL_RPS} burst=${SVC_GATEWAY_RL_BURST} tarpit=${SVC_GATEWAY_RL_TARPIT_MS}ms..."
log="$(mktemp -t svc-gateway-rl.XXXXXX.log)"
BIND_ADDR="$BIND_ADDR" \
SVC_GATEWAY_RL_RPS="$SVC_GATEWAY_RL_RPS" \
SVC_GATEWAY_RL_BURST="$SVC_GATEWAY_RL_BURST" \
SVC_GATEWAY_RL_TARPIT_MS="$SVC_GATEWAY_RL_TARPIT_MS" \
RUST_LOG="$RUST_LOG" \
cargo run -p svc-gateway >"$log" 2>&1 &

PID=$!
echo "PID: $PID (logs: $log)"

# Wait for health
att=0; code=""
while [ $att -lt 100 ]; do
  if ! kill -0 "$PID" 2>/dev/null; then tail -n 200 "$log" || true; exit 1; fi
  code="$(curl -s -o /dev/null -w "%{http_code}" "http://${BIND_ADDR}/healthz" || true)"
  [ "$code" = "200" ] && break
  att=$((att+1)); sleep 0.1
done
[ "$code" = "200" ] || { echo "gateway not up"; tail -n 200 "$log" || true; exit 1; }

# Global watchdog so this script can never hang
WATCHDOG_SECS=$((REQ_TIMEOUT + 8))
( sleep "$WATCHDOG_SECS"; if ps -p "$PID" >/dev/null 2>&1; then
    echo "Watchdog: requests still running after ${WATCHDOG_SECS}s; dumping logs…"
    tail -n 200 "$log" || true
  fi ) & WD=$!

echo "bursting /dev/rl… (${PAR} concurrent hits)"
tmp="$(mktemp -t rl_hits.XXXXXX)"

# IMPORTANT: temporarily relax -e/pipefail for this pipeline so any single timeout
# doesn’t kill the whole script. We restore strict mode immediately after.
set +e
( yes 1 | head -n "$PAR" | xargs -P "$PAR" -I{} \
  sh -c '
    code=$(curl -sS -o /dev/null \
      --connect-timeout '"$CONNECT_TIMEOUT"' \
      --max-time '"$REQ_TIMEOUT"' \
      -w "%{http_code}" "http://'"$BIND_ADDR"'/dev/rl" 2>/dev/null || echo 000);
    echo "$code"
  ' ) >>"$tmp"
xargs_rc=$?
set -e

# Kill watchdog (we finished requests batch, even if some errored)
kill "$WD" >/dev/null 2>&1 || true

if [ $xargs_rc -ne 0 ]; then
  echo "note: some requests failed (xargs rc=$xargs_rc); see tallies and logs below."
fi

# Tally
ok=$(grep -c '^200$' "$tmp" || true)
rl=$(grep -c '^429$' "$tmp" || true)
ot=$(grep -c '^000$' "$tmp" || true)
echo "OK: $ok  RL(429): $rl  ERR/TO: $ot"

echo "metrics:"
curl -s "http://${BIND_ADDR}/metrics" | grep -E 'gateway_rejections_total\{reason="rate_limit"\}' || true

echo "Done. To stop: kill $PID"
