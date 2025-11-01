#!/usr/bin/env bash
set -euo pipefail

# Colors
G="\033[0;32m"; Y="\033[1;33m"; R="\033[0;31m"; Z="\033[0m"

echo -e "${Y}fmt + clippy + build…${Z}"
cargo fmt -p omnigate
cargo clippy -p omnigate --no-deps -- -D warnings
cargo build -p omnigate

# Defaults (API=5305, ADMIN=9605)
API_ADDR="${API_ADDR:-127.0.0.1:5305}"
ADMIN_ADDR="${ADMIN_ADDR:-127.0.0.1:9605}"
LOG="/tmp/omnigate.log"

# REQUIRED: config path (can be overridden)
CONFIG_PATH="${OMNIGATE_CONFIG:-crates/omnigate/configs/omnigate.toml}"
if [ ! -f "${CONFIG_PATH}" ]; then
  echo -e "${R}❌ Config file not found at: ${CONFIG_PATH}${Z}"
  exit 1
fi
echo -e "${Y}using config: ${CONFIG_PATH}${Z}"

# Optional dev-readiness override
export OMNIGATE_DEV_READY="${OMNIGATE_DEV_READY:-}"

echo -e "${Y}starting omnigate at ${API_ADDR} (logs: ${LOG})…${Z}"
BIN="target/debug/omnigate"
# Pass --config so the readiness 'config' gate can flip
"${BIN}" --config "${CONFIG_PATH}" > "${LOG}" 2>&1 &
PID=$!

cleanup() {
  echo -e "${Y}stopping omnigate (pid=${PID})…${Z}"
  kill "${PID}" >/dev/null 2>&1 || true
  wait "${PID}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Wait for /healthz on the API port
echo "waiting for /healthz…"
set +e
for i in {1..100}; do
  curl -sf "http://${API_ADDR}/healthz" >/dev/null && break
  sleep 0.1
done
set -e

echo "-- /healthz (API)"
curl -sf "http://${API_ADDR}/healthz"
echo

# Metrics are on the ADMIN plane
echo "-- /metrics (ADMIN)"
if ! curl -sf "http://${ADMIN_ADDR}/metrics" | head -n 20; then
  echo -e "${R}❌ /metrics not available on ${ADMIN_ADDR}. Check ${LOG}.${Z}"
  exit 1
fi

# Root / should 404
echo "-- / (API, expect 404)"
CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://${API_ADDR}/")
if [ "${CODE}" = "404" ]; then
  echo -e "✅ 404 ok"
else
  echo -e "${R}❌ expected 404, got ${CODE}${Z}"
  exit 1
fi

check_ready() {
  local path="$1"
  local status body
  status=$(curl -s -o /dev/null -w "%{http_code}" "http://${API_ADDR}${path}")
  body=$(curl -s "http://${API_ADDR}${path}" || true)
  echo "readyz status (${path} on API): ${status}"
  if [ "${status}" = "200" ]; then
    echo -e "${G}✅ ready (body: ${body})${Z}"
    return 0
  else
    echo -e "${Y}ℹ️ not ready (status ${status}, body: ${body})${Z}"
    # Show last 20 log lines to explain the gate
    echo -e "${Y}--- tail ${LOG} ---${Z}"
    tail -n 20 "${LOG}" || true
    echo -e "${Y}-------------------${Z}"
    return 1
  fi
}

echo "-- /readyz (API; truthful readiness)"
check_ready "/readyz" || true

echo "-- /ops/readyz (API alias)"
check_ready "/ops/readyz" || true

echo -e "${G}✅ smoke ok${Z}"
