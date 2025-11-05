#!/usr/bin/env bash
# Deterministic sanity for rate_limit: expect 200 then 429s.
set -euo pipefail

# -------- Knobs (override via env) --------
: "${BIND_ADDR:=127.0.0.1:5304}"
: "${SVC_GATEWAY_DEV_ROUTES:=1}"   # enable /dev/*
: "${SVC_GATEWAY_RL_RPS:=1}"       # 1 token/sec
: "${SVC_GATEWAY_RL_BURST:=1}"     # burst = 1 token
: "${SVC_GATEWAY_RL_TARPIT_MS:=0}" # no tarpit to keep it snappy
: "${RUST_LOG:=info,svc_gateway=debug}"
: "${WAIT_MS:=10000}"              # up to 10s for /healthz

log="$(mktemp -t svc-gateway-sanity.XXXXXX.log)"
pids=()

cleanup() {
  set +e
  if [ "${#pids[@]}" -gt 0 ]; then
    for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done
  fi
}
trap cleanup EXIT

echo "killing any prior gateway…"
pkill -f svc-gateway || true
sleep 0.2

echo "priming build…"
cargo build -p svc-gateway >/dev/null

echo "starting gateway (RPS=$SVC_GATEWAY_RL_RPS BURST=$SVC_GATEWAY_RL_BURST TARPIT=${SVC_GATEWAY_RL_TARPIT_MS}ms)…"
BIND_ADDR="$BIND_ADDR" \
SVC_GATEWAY_DEV_ROUTES="$SVC_GATEWAY_DEV_ROUTES" \
SVC_GATEWAY_RL_RPS="$SVC_GATEWAY_RL_RPS" \
SVC_GATEWAY_RL_BURST="$SVC_GATEWAY_RL_BURST" \
SVC_GATEWAY_RL_TARPIT_MS="$SVC_GATEWAY_RL_TARPIT_MS" \
RUST_LOG="$RUST_LOG" \
cargo run -p svc-gateway >"$log" 2>&1 &

PID=$!
pids+=("$PID")
echo "PID: $PID (logs: $log)"

# -------- Wait for /healthz (portable: attempts * 100ms) --------
attempts=$(( (WAIT_MS + 99) / 100 ))  # ceil(WAIT_MS/100)
code=""
for _ in $(seq 1 "$attempts"); do
  if ! kill -0 "$PID" 2>/dev/null; then
    echo "gateway exited early"; tail -n 120 "$log" || true; exit 1
  fi
  code="$(curl -s -o /dev/null -w '%{http_code}' "http://${BIND_ADDR}/healthz" || true)"
  [ "$code" = "200" ] && break
  sleep 0.1
done
[ "$code" = "200" ] || { echo "gateway not healthy in ${WAIT_MS}ms"; tail -n 200 "$log" || true; exit 1; }
echo "gateway healthy."

# -------- Deterministic 3-hit probe --------
hit() { curl -s -o /dev/null -w '%{http_code}' "http://${BIND_ADDR}/dev/rl" || echo "ERR"; }

echo "probing /dev/rl (3 rapid hits; expect 200 then 429/429)…"
c1="$(hit)"; echo "hit 1 -> $c1"
c2="$(hit)"; echo "hit 2 -> $c2"
c3="$(hit)"; echo "hit 3 -> $c3"

# -------- Validate expectation --------
errs=0; [ "$c1" = "ERR" ] && errs=$((errs+1)); [ "$c2" = "ERR" ] && errs=$((errs+1)); [ "$c3" = "ERR" ] && errs=$((errs+1))

if [ "$c1" != "200" ] || [ $errs -gt 0 ]; then
  echo "FAIL: expected hit1=200 and no curl errors; saw: $c1 $c2 $c3 (ERRS=$errs)"
  echo "--- tail logs ($log) ---"; tail -n 200 "$log" || true
  echo "--- metrics (snippet) ---"
  curl -s "http://${BIND_ADDR}/metrics" | grep -E 'gateway_rejections_total\{reason="rate_limit"\}|gateway_http_requests_total\{.*dev_rl' || true
  exit 2
fi

count_429=0
[ "$c2" = "429" ] && count_429=$((count_429+1))
[ "$c3" = "429" ] && count_429=$((count_429+1))

if [ $count_429 -lt 1 ]; then
  echo "FAIL: expected at least one 429 under RPS=1,BURST=1 but saw: $c1 $c2 $c3"
  echo "--- tail logs ---"; tail -n 200 "$log" || true
  echo "--- metrics (snippet) ---"
  curl -s "http://${BIND_ADDR}/metrics" | grep -E 'gateway_rejections_total\{reason="rate_limit"\}|gateway_http_requests_total\{.*dev_rl' || true
  exit 3
fi

echo "OK: observed $count_429 x 429 (as expected)."
echo "metrics (rate_limit counters):"
curl -s "http://${BIND_ADDR}/metrics" | grep -E 'gateway_rejections_total\{reason="rate_limit"\}|gateway_http_requests_total\{.*dev_rl' || true

echo "Done. To stop: kill $PID (or Ctrl-C to trigger trap)."
