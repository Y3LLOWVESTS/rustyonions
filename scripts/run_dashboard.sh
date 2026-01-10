#!/usr/bin/env bash
# RO:WHAT — Truthful dev runner for svc-admin + (macronode/micronode) + SPA.
# RO:WHY  — One command to bring up the real stack, cleanly, with diagnostics.
# RO:NOTE — svc-admin health/ready live on the metrics listener (SVC_ADMIN_METRICS_ADDR), not the UI/API port.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

say() { printf '[run-dashboard] %s\n' "$*"; }
die() { say "ERROR: $*"; exit 1; }

FLAG_MACRO="false"
FLAG_MICRO="false"
FLAG_UI="true"
FLAG_NO_KILL="false"

usage() {
  cat <<'EOF'
Usage:
  scripts/run_dashboard.sh [-macro] [-micro] [--no-ui] [--no-kill] [-h]

Flags:
  -macro     Start macronode
  -micro     Start micronode
  --no-ui    Do not start the Vite UI dev server
  --no-kill  Do not kill existing listeners on dev ports
  -h         Help

Defaults:
  If neither -macro nor -micro is provided, defaults to -macro.

Env overrides:
  MACRONODE_HTTP_ADDR=127.0.0.1:8080
  MICRONODE_HTTP_ADDR=127.0.0.1:5310

  SVC_ADMIN_BIND_ADDR=127.0.0.1:5300        (UI/API)
  SVC_ADMIN_METRICS_ADDR=127.0.0.1:5311     (healthz/readyz/metrics)

  DEV_WAIT_TIMEOUT_SECS=60
  DEV_WAIT_STEP_SECS=0.25
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -macro) FLAG_MACRO="true"; shift ;;
    -micro) FLAG_MICRO="true"; shift ;;
    --no-ui) FLAG_UI="false"; shift ;;
    --no-kill) FLAG_NO_KILL="true"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "Unknown arg: $1 (use -h)" ;;
  esac
done

if [[ "${FLAG_MACRO}" == "false" && "${FLAG_MICRO}" == "false" ]]; then
  FLAG_MACRO="true"
fi

MACRONODE_HTTP_ADDR="${MACRONODE_HTTP_ADDR:-127.0.0.1:8080}"
MICRONODE_HTTP_ADDR="${MICRONODE_HTTP_ADDR:-127.0.0.1:5310}"

SVC_ADMIN_BIND_ADDR="${SVC_ADMIN_BIND_ADDR:-127.0.0.1:5300}"
SVC_ADMIN_METRICS_ADDR="${SVC_ADMIN_METRICS_ADDR:-127.0.0.1:5311}"

SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND="${SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND:-true}"

DEV_WAIT_TIMEOUT_SECS="${DEV_WAIT_TIMEOUT_SECS:-60}"
DEV_WAIT_STEP_SECS="${DEV_WAIT_STEP_SECS:-0.25}"

MACRO_PID=""
MICRO_PID=""
ADMIN_PID=""
UI_PID=""

cleanup() {
  say "Shutting down..."
  for pid in "$UI_PID" "$ADMIN_PID" "$MICRO_PID" "$MACRO_PID"; do
    if [[ -n "${pid}" ]]; then
      kill "${pid}" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
  say "Done."
}
trap cleanup EXIT INT TERM

need_cmd() { command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"; }

kill_port() {
  local port="$1"
  local pids
  pids="$(lsof -ti "tcp:${port}" 2>/dev/null || true)"
  if [[ -n "${pids}" ]]; then
    say "Killing existing listener(s) on tcp:${port} -> ${pids}"
    # shellcheck disable=SC2086
    kill -9 ${pids} 2>/dev/null || true
  fi
}

port_of() { echo "$1" | awk -F: '{print $NF}'; }

list_listeners() {
  local label="$1"
  shift
  say "Listeners (${label}):"
  for hp in "$@"; do
    local p
    p="$(port_of "${hp}")"
    local out
    out="$(lsof -nP -iTCP:"${p}" -sTCP:LISTEN 2>/dev/null || true)"
    if [[ -n "${out}" ]]; then
      echo "${out}" | sed 's/^/[listen] /'
    else
      echo "[listen] tcp:${p} (none)"
    fi
  done
}

http_probe() {
  local url="$1"
  local tmp
  tmp="$(mktemp)"
  local code
  code="$(curl -sS -m 2 -o "${tmp}" -w "%{http_code}" "${url}" 2>/dev/null || echo "000")"
  local head
  head="$(head -c 220 "${tmp}" 2>/dev/null || true)"
  rm -f "${tmp}" || true
  say "PROBE ${url} -> HTTP ${code} | body: ${head}"
}

truth_probe_node() {
  local name="$1"
  local addr="$2"
  say "Truth probes (${name}):"
  http_probe "http://${addr}/api/v1/status" || true
  http_probe "http://${addr}/api/v1/system/summary" || true
  http_probe "http://${addr}/api/v1/storage/summary" || true
  http_probe "http://${addr}/metrics" || true
}

wait_for_ok_pid() {
  local name="$1"
  local pid="$2"
  local addr="$3"   # host:port
  local url_ready="http://${addr}/readyz"
  local url_health="http://${addr}/healthz"

  say "Waiting for ${name}: ${url_ready} (timeout=${DEV_WAIT_TIMEOUT_SECS}s)"

  local max_tries
  max_tries="$(python3 - <<PY
import math
timeout=float("${DEV_WAIT_TIMEOUT_SECS}")
step=float("${DEV_WAIT_STEP_SECS}")
print(max(1, int(math.ceil(timeout/step))))
PY
)"

  for _ in $(seq 1 "${max_tries}"); do
    if ! kill -0 "${pid}" >/dev/null 2>&1; then
      die "${name} exited early (pid=${pid})."
    fi

    if curl -sf "${url_ready}" >/dev/null 2>&1; then
      say "${name} is ready (${url_ready} OK)."
      return 0
    fi

    sleep "${DEV_WAIT_STEP_SECS}"
  done

  say "${name} not ready on /readyz; trying ${url_health}..."
  for _ in $(seq 1 10); do
    if ! kill -0 "${pid}" >/dev/null 2>&1; then
      die "${name} exited early (pid=${pid})."
    fi
    if curl -sf "${url_health}" >/dev/null 2>&1; then
      say "${name} is responding on /healthz (treating as ready for dev)."
      return 0
    fi
    sleep "${DEV_WAIT_STEP_SECS}"
  done

  say "Diagnostics for ${name}:"
  http_probe "${url_ready}" || true
  http_probe "${url_health}" || true
  list_listeners "${name} readiness port" "${addr}" || true

  die "${name} did not become ready within ${DEV_WAIT_TIMEOUT_SECS}s."
}

cd "${ROOT_DIR}"

need_cmd cargo
need_cmd curl
need_cmd python3
need_cmd lsof
if [[ "${FLAG_UI}" == "true" ]]; then need_cmd npm; fi

say "Repo root: ${ROOT_DIR}"
say "Flags: macro=${FLAG_MACRO} micro=${FLAG_MICRO} ui=${FLAG_UI} no-kill=${FLAG_NO_KILL}"
say "macronode http: ${MACRONODE_HTTP_ADDR}"
say "micronode http: ${MICRONODE_HTTP_ADDR}"
say "svc-admin bind: ${SVC_ADMIN_BIND_ADDR} metrics: ${SVC_ADMIN_METRICS_ADDR}"
say "SPA dev:        http://localhost:5173"
echo

if [[ "${FLAG_NO_KILL}" != "true" ]]; then
  kill_port 5173
  kill_port "$(port_of "${MACRONODE_HTTP_ADDR}")"
  kill_port 8090
  kill_port "$(port_of "${MICRONODE_HTTP_ADDR}")"
  kill_port "$(port_of "${SVC_ADMIN_BIND_ADDR}")"
  kill_port "$(port_of "${SVC_ADMIN_METRICS_ADDR}")"
  for p in 5301 5302 5303 5304 5305; do
    kill_port "${p}"
  done
fi

list_listeners "before build" "${MACRONODE_HTTP_ADDR}" "${MICRONODE_HTTP_ADDR}" "${SVC_ADMIN_BIND_ADDR}" "${SVC_ADMIN_METRICS_ADDR}"
echo

say "Building binaries (one-time, foreground)..."
cargo build -p macronode
cargo build -p micronode
cargo build -p svc-admin --bin svc-admin

if [[ "${FLAG_MACRO}" == "true" ]]; then
  (
    cd "${ROOT_DIR}"
    say "Starting macronode..."
    RON_HTTP_ADDR="${MACRONODE_HTTP_ADDR}" \
    RON_METRICS_ADDR="${MACRONODE_HTTP_ADDR}" \
    MACRONODE_DEV_INSECURE=1 \
    "${ROOT_DIR}/target/debug/macronode"
  ) &
  MACRO_PID=$!
  say "macronode PID: ${MACRO_PID}"
  wait_for_ok_pid "macronode" "${MACRO_PID}" "${MACRONODE_HTTP_ADDR}"
  truth_probe_node "macronode" "${MACRONODE_HTTP_ADDR}"
fi

if [[ "${FLAG_MICRO}" == "true" ]]; then
  (
    cd "${ROOT_DIR}"
    say "Starting micronode..."
    MICRONODE_BIND="${MICRONODE_HTTP_ADDR}" \
    MICRONODE_METRICS_BIND="${MICRONODE_HTTP_ADDR}" \
    "${ROOT_DIR}/target/debug/micronode"
  ) &
  MICRO_PID=$!
  say "micronode PID: ${MICRO_PID}"
  wait_for_ok_pid "micronode" "${MICRO_PID}" "${MICRONODE_HTTP_ADDR}"
  truth_probe_node "micronode" "${MICRONODE_HTTP_ADDR}"
fi

(
  cd "${ROOT_DIR}"
  say "Starting svc-admin..."

  envs=(
    "SVC_ADMIN_BIND_ADDR=${SVC_ADMIN_BIND_ADDR}"
    "SVC_ADMIN_METRICS_ADDR=${SVC_ADMIN_METRICS_ADDR}"
    "SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=${SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND}"
    "SVC_ADMIN_NODES_CLEAR=1"
  )

  if [[ "${FLAG_MACRO}" == "true" ]]; then
    envs+=(
      "SVC_ADMIN_NODES__macronode__BASE_URL=http://${MACRONODE_HTTP_ADDR}"
      "SVC_ADMIN_NODES__macronode__DISPLAY_NAME=Macronode"
      "SVC_ADMIN_NODES__macronode__ENVIRONMENT=dev"
      "SVC_ADMIN_NODES__macronode__INSECURE_HTTP=true"
      "SVC_ADMIN_NODES__macronode__FORCED_PROFILE=macronode"
    )
  fi

  if [[ "${FLAG_MICRO}" == "true" ]]; then
    envs+=(
      "SVC_ADMIN_NODES__micronode__BASE_URL=http://${MICRONODE_HTTP_ADDR}"
      "SVC_ADMIN_NODES__micronode__DISPLAY_NAME=Micronode"
      "SVC_ADMIN_NODES__micronode__ENVIRONMENT=dev"
      "SVC_ADMIN_NODES__micronode__INSECURE_HTTP=true"
      "SVC_ADMIN_NODES__micronode__FORCED_PROFILE=micronode"
    )
  fi

  env "${envs[@]}" "${ROOT_DIR}/target/debug/svc-admin"
) &
ADMIN_PID=$!
say "svc-admin PID: ${ADMIN_PID}"

wait_for_ok_pid "svc-admin" "${ADMIN_PID}" "${SVC_ADMIN_METRICS_ADDR}"
http_probe "http://${SVC_ADMIN_METRICS_ADDR}/readyz" || true
http_probe "http://${SVC_ADMIN_METRICS_ADDR}/metrics" || true

if [[ "${FLAG_UI}" == "true" ]]; then
  (
    cd "${ROOT_DIR}/crates/svc-admin/ui"
    say "Starting svc-admin UI (npm run dev)..."
    npm run dev
  ) &
  UI_PID=$!
  say "UI dev server PID: ${UI_PID}"
fi

echo
say "Stack is up."
say "SPA:               http://localhost:5173"
say "svc-admin UI/API:   http://${SVC_ADMIN_BIND_ADDR}"
say "svc-admin health:   http://${SVC_ADMIN_METRICS_ADDR}/healthz"
say "svc-admin ready:    http://${SVC_ADMIN_METRICS_ADDR}/readyz"
if [[ "${FLAG_MACRO}" == "true" ]]; then say "macronode admin:    http://${MACRONODE_HTTP_ADDR}"; fi
if [[ "${FLAG_MICRO}" == "true" ]]; then say "micronode admin:    http://${MICRONODE_HTTP_ADDR}"; fi
say "Ctrl-C to stop."

wait
