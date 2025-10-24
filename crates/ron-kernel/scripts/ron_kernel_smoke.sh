#!/usr/bin/env bash
# Verifies: build+tests, demo boots, config loads (seed after boot), /metrics|/healthz OK,
# readiness polled but non-fatal, amnesia=false -> 0 then amnesia=true -> 1 via watcher.

set -euo pipefail

curl_status() { curl -s -o /dev/null -w "%{http_code}" "$1"; }
fail() { echo "ERROR: $*" 1>&2; exit 1; }

echo "[1/9] Build & test"
cargo build -p ron-kernel
cargo test  -p ron-kernel

echo "[2/9] Start example (kernel_demo) on 127.0.0.1:0"
LOGFILE="$(mktemp)"
RON_AMNESIA=0 cargo run -p ron-kernel --example kernel_demo >"$LOGFILE" 2>&1 &
APP_PID=$!
trap 'kill -TERM $APP_PID >/dev/null 2>&1 || true; rm -f "$LOGFILE" /tmp/ron-kernel.toml' EXIT

echo "Waiting for server to print URLs… (log: $LOGFILE)"
for i in {1..120}; do
  grep -q "metrics:" "$LOGFILE" && break || sleep 0.2
done

ADDR="$(grep -m1 -E 'metrics:' "$LOGFILE" | sed -n 's#.*http://\([^/]*\)/metrics.*#\1#p')"
[ -n "${ADDR:-}" ] || fail "could not discover server address from logs"
echo "Discovered addr: $ADDR"

METRICS_URL="http://$ADDR/metrics"
HEALTHZ_URL="http://$ADDR/healthz"
READYZ_URL="http://$ADDR/readyz"

echo "[3/9] Seed config AFTER boot so watcher observes first load"
cat > /tmp/ron-kernel.toml <<'TOML'
version = 1
amnesia = false
TOML
sleep 0.8

echo "[4/9] Poll /readyz up to 10s (non-fatal if it stays 503)"
READY=0
for i in {1..100}; do
  code="$(curl_status "$READYZ_URL" || true)"
  [ "$code" = "200" ] && { READY=1; break; }
  [ "$i" -eq 1 ] && echo "initial /readyz code: ${code:-curl-failed}"
  sleep 0.1
done
if [ "$READY" -eq 1 ]; then
  echo "readyz OK (200)"
else
  echo "readyz still $code — continuing (non-fatal)"
fi

echo "[5/9] Curl /metrics and /healthz"
curl -fsS "$METRICS_URL" | head -n 5 >/dev/null
curl -fsS "$HEALTHZ_URL" >/dev/null
echo "Surfaces OK"

read_amnesia() { curl -fsS "$METRICS_URL" | awk '/^amnesia_mode[[:space:]]/{print $2; exit}'; }

echo "[6/9] Read baseline amnesia_mode"
BASE="$(read_amnesia || true)"
echo "amnesia baseline: ${BASE:-unset}"

echo "[7/9] Force amnesia=false → expect metric 0"
cat > /tmp/ron-kernel.toml <<'TOML'
version = 2
amnesia = false
TOML
sleep 0.8
A0="$(read_amnesia || true)"
echo "amnesia after false: ${A0:-unset}"
[ "${A0:-x}" = "0" ] || { echo "--- metrics sample ---"; curl -fsS "$METRICS_URL" | head -n 50; fail "expected amnesia_mode=0"; }

echo "[8/9] Force amnesia=true → expect metric 1"
cat > /tmp/ron-kernel.toml <<'TOML'
version = 3
amnesia = true
TOML
sleep 0.8
A1="$(read_amnesia || true)"
echo "amnesia after true:  ${A1:-unset}"
[ "${A1:-x}" = "1" ] || { echo "--- metrics sample ---"; curl -fsS "$METRICS_URL" | head -n 50; fail "expected amnesia_mode=1"; }

echo "[9/9] Done — killing example"
kill -TERM $APP_PID
wait $APP_PID || true
echo "Smoke passed ✅"
