#!/usr/bin/env bash
# RO:WHAT — WOW demo runner: omnigate + svc-gateway + static site files (no cheating).
# RO:WHY  — One command: bring up a real app-plane static site through gateway.
# RO:INV  — Do NOT embed HTML in core; serve from examples/site-demo/site/.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

say() { printf '[demo-site] %s\n' "$*"; }
die() { say "ERROR: $*"; exit 1; }

need_cmd() { command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"; }

need_cmd cargo
need_cmd curl
need_cmd lsof

GATEWAY_BIND="${DEMO_SITE_GATEWAY_BIND:-127.0.0.1:5304}"
OMNIGATE_BIND="${DEMO_SITE_OMNIGATE_BIND:-127.0.0.1:5305}"
OMNIGATE_METRICS="${DEMO_SITE_OMNIGATE_METRICS:-127.0.0.1:9605}"

SITE_DIR="${DEMO_SITE_DIR:-${ROOT_DIR}/examples/site-demo/site}"

kill_port() {
  local port="$1"
  local pids
  pids="$(lsof -ti "tcp:${port}" 2>/dev/null || true)"
  if [[ -n "${pids}" ]]; then
    say "Killing existing listener(s) on tcp:${port} -> ${pids}"
    kill -9 ${pids} 2>/dev/null || true
  fi
}

port_of() { echo "$1" | awk -F: '{print $NF}'; }

kill_port "$(port_of "${GATEWAY_BIND}")"
kill_port "$(port_of "${OMNIGATE_BIND}")"
kill_port "$(port_of "${OMNIGATE_METRICS}")"

if [[ ! -d "${SITE_DIR}" ]]; then
  die "Site dir missing: ${SITE_DIR} (expected examples/site-demo/site/)"
fi

if [[ ! -f "${SITE_DIR}/index.html" ]]; then
  die "Missing ${SITE_DIR}/index.html (create it; demo refuses to cheat)"
fi

say "Building omnigate + svc-gateway..."
cargo build -p omnigate -p svc-gateway

say "Starting omnigate with auto-restart loop..."
(
  cd "${ROOT_DIR}"
  while true; do
    say "omnigate starting (bind=${OMNIGATE_BIND}, metrics=${OMNIGATE_METRICS})"
    OMNIGATE_BIND="${OMNIGATE_BIND}" \
    OMNIGATE_METRICS_ADDR="${OMNIGATE_METRICS}" \
    OMNIGATE_APP_STATIC_DIR="${SITE_DIR}" \
    "${ROOT_DIR}/target/debug/omnigate" || true
    say "omnigate exited; restarting in 0.75s..."
    sleep 0.75
  done
) &
OMNI_PID=$!

say "Starting svc-gateway (bind=${GATEWAY_BIND}, upstream=http://${OMNIGATE_BIND})..."
(
  cd "${ROOT_DIR}"
  SVC_GATEWAY_BIND_ADDR="${GATEWAY_BIND}" \
  SVC_GATEWAY_OMNIGATE_BASE_URL="http://${OMNIGATE_BIND}" \
  "${ROOT_DIR}/target/debug/svc-gateway"
) &
GW_PID=$!

cleanup() {
  say "Shutting down..."
  kill "${GW_PID}" 2>/dev/null || true
  kill "${OMNI_PID}" 2>/dev/null || true
  wait 2>/dev/null || true
  say "Done."
}
trap cleanup EXIT INT TERM

sleep 0.4

say ""
say "WOW demo is up."
say "Site:           http://${GATEWAY_BIND}/app/site"
say "Reload:         curl -sS -X POST http://${GATEWAY_BIND}/app/site/reload -H 'content-type: application/json' -d '{}'"
say "Gateway:        http://${GATEWAY_BIND}/healthz   http://${GATEWAY_BIND}/readyz   http://${GATEWAY_BIND}/metrics"
say "Omnigate ops:   http://${OMNIGATE_BIND}/healthz  http://${OMNIGATE_BIND}/readyz"
say "Omnigate ops:   http://${OMNIGATE_BIND}/ops/healthz  http://${OMNIGATE_BIND}/ops/readyz  http://${OMNIGATE_BIND}/ops/metrics"
say "Ctrl-C to stop."
say ""

wait
