#!/usr/bin/env bash
set -euo pipefail

: "${BIND_ADDR:=127.0.0.1:5304}"
: "${HEALTH_PATH:=/healthz}"
: "${READY_PATH:=/readyz}"
: "${DEV_RL_PATH:=/dev/rl}"
: "${DEV_ECHO_PATH:=/dev/echo}"

# Load gen knobs
: "${DUR:=30s}"
: "${THREADS:=4}"
: "${CONNS:=128}"
: "${QPS:=0}"           # 0 = unlimited for hey; ignored by wrk

# Gateway knobs (explicit to make runs reproducible)
: "${SVC_GATEWAY_DEV_ROUTES:=${SVC_GATEWAY_DEV:-1}}"
: "${SVC_GATEWAY_READY_TIMEOUT_MS:=1000}"
: "${SVC_GATEWAY_READY_MAX_INFLIGHT:=1024}"
: "${SVC_GATEWAY_RL_RPS:=3}"
: "${SVC_GATEWAY_RL_BURST:=5}"
: "${SVC_GATEWAY_RL_TARPIT_MS:=0}"    # 0 = no tarpit (keeps /dev/rl bench fast)
: "${SVC_GATEWAY_MAX_BODY_BYTES:=1048576}" # 1 MiB
: "${SVC_GATEWAY_DEV_READY:=1}"            # force ready during health/ready bench
: "${SVC_GATEWAY_READY_SLEEP_MS:=0}"

LOG_DIR="${LOG_DIR:-./target/bench-logs}"
mkdir -p "$LOG_DIR"

echo "killing any existing svc-gateway…"
pkill -f svc-gateway || true
sleep 0.2

echo "starting gateway…"
GLOG="$LOG_DIR/gateway.log"
BIND_ADDR="$BIND_ADDR" \
SVC_GATEWAY_DEV_ROUTES="$SVC_GATEWAY_DEV_ROUTES" \
SVC_GATEWAY_READY_TIMEOUT_MS="$SVC_GATEWAY_READY_TIMEOUT_MS" \
SVC_GATEWAY_READY_MAX_INFLIGHT="$SVC_GATEWAY_READY_MAX_INFLIGHT" \
SVC_GATEWAY_RL_RPS="$SVC_GATEWAY_RL_RPS" \
SVC_GATEWAY_RL_BURST="$SVC_GATEWAY_RL_BURST" \
SVC_GATEWAY_RL_TARPIT_MS="$SVC_GATEWAY_RL_TARPIT_MS" \
SVC_GATEWAY_MAX_BODY_BYTES="$SVC_GATEWAY_MAX_BODY_BYTES" \
SVC_GATEWAY_DEV_READY="$SVC_GATEWAY_DEV_READY" \
SVC_GATEWAY_READY_SLEEP_MS="$SVC_GATEWAY_READY_SLEEP_MS" \
RUST_LOG="${RUST_LOG:-info,svc_gateway=debug}" \
cargo run -p svc-gateway >"$GLOG" 2>&1 &

PID=$!
trap 'kill "$PID" 2>/dev/null || true' EXIT
echo "PID: $PID (logs -> $GLOG)"

# wait for health
att=0
until [ $att -ge 100 ]; do
  code="$(curl -s -o /dev/null -w "%{http_code}" "http://${BIND_ADDR}${HEALTH_PATH}" || true)"
  [ "$code" = "200" ] && break
  att=$((att+1)); sleep 0.1
done
[ "$code" = "200" ] || { echo "gateway not ready"; tail -n 120 "$GLOG" || true; exit 1; }

have_hey=0; have_wrk=0
command -v hey >/dev/null && have_hey=1
command -v wrk >/dev/null && have_wrk=1
[ "$have_hey" = 0 ] && [ "$have_wrk" = 0 ] && { echo "install hey or wrk"; exit 1; }

bench_hey() {
  local name="$1" url="$2"
  local out="$LOG_DIR/${name}_hey.txt"
  local q=()
  [ "$QPS" != "0" ] && q=(-q "$QPS")
  echo "== hey: $name -> $url"
  hey -z "$DUR" -c "$CONNS" "${q[@]}" "$url" | tee "$out"
}

bench_wrk() {
  local name="$1" url="$2"
  local out="$LOG_DIR/${name}_wrk.txt"
  echo "== wrk: $name -> $url"
  wrk -t"$THREADS" -c"$CONNS" -d"$DUR" "$url" | tee "$out"
}

bench() {
  local name="$1" path="$2"
  local url="http://${BIND_ADDR}${path}"
  if [ "$have_hey" = 1 ]; then bench_hey "$name" "$url"; else bench_wrk "$name" "$url"; fi
}

echo
echo "===== 1) Fast path: ${HEALTH_PATH} ====="
bench "healthz" "$HEALTH_PATH"

echo
echo "===== 2) Guarded path: ${READY_PATH} (no sleep, forced ready) ====="
bench "readyz" "$READY_PATH"

echo
echo "===== 3) Rejection hot path: ${DEV_RL_PATH} (will produce 429s) ====="
bench "dev_rl" "$DEV_RL_PATH"

echo
echo "===== 4) Rejection hot path: ${DEV_ECHO_PATH} (413 via body cap) ====="
bigfile="$(mktemp)"; head -c $((SVC_GATEWAY_MAX_BODY_BYTES+1024)) </dev/zero | tr '\0' 'A' > "$bigfile"
for i in $(seq 1 10); do
  curl -s -o /dev/null -w "%{http_code}\n" -X POST --data-binary @"$bigfile" "http://${BIND_ADDR}${DEV_ECHO_PATH}"
done | tee "$LOG_DIR/dev_echo_413.txt"
rm -f "$bigfile"

echo
echo "===== 5) Metrics snapshot ====="
curl -s "http://${BIND_ADDR}/metrics" \
 | grep -E 'gateway_http_requests_total|gateway_request_latency_seconds|gateway_rejections_total' \
 | sed -n '1,200p' > "$LOG_DIR/metrics.txt"
sed -n '1,200p' "$LOG_DIR/metrics.txt"

echo "Done. To stop: kill $PID"
