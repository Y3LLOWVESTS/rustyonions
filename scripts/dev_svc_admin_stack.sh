#!/usr/bin/env bash
# RO:WHAT  — Dev helper to run macronode + svc-admin + svc-admin UI (Vite) together.
# RO:WHY   — One command to boot the whole admin stack, then just open the browser.
# RO:USES  — Assumes Rust toolchain, cargo, Node.js, and npm are installed.
# RO:NOTE  — Run from anywhere; script auto-detects project root.
# RO:DEVUX — Starts macronode first, waits for readiness, then starts svc-admin and the UI.
#            This avoids noisy sampler warnings + Cargo shared-lock contention.

set -euo pipefail

# Discover project root (one level above this script).
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

say() {
  printf '[dev-stack] %s\n' "$*"
}

# Default addresses (can be overridden with env if you really want).
MACRONODE_HTTP_ADDR="${RON_HTTP_ADDR:-127.0.0.1:8080}"
MACRONODE_METRICS_ADDR="${RON_METRICS_ADDR:-${MACRONODE_HTTP_ADDR}}"

SVC_ADMIN_HTTP_ADDR_DEFAULT="127.0.0.1:5300"
SVC_ADMIN_METRICS_ADDR_DEFAULT="127.0.0.1:5310"

SVC_ADMIN_HTTP_ADDR="${SVC_ADMIN_HTTP_ADDR:-${SVC_ADMIN_HTTP_ADDR_DEFAULT}}"
SVC_ADMIN_METRICS_ADDR="${SVC_ADMIN_METRICS_ADDR:-${SVC_ADMIN_METRICS_ADDR_DEFAULT}}"

# Dev-only: enable the app playground (default ON for this dev stack).
SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND="${SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND:-true}"

# Wait behavior (dev ergonomics).
DEV_WAIT_FOR_MACRONODE="${DEV_WAIT_FOR_MACRONODE:-true}"  # set false to skip waiting
DEV_WAIT_TIMEOUT_SECS="${DEV_WAIT_TIMEOUT_SECS:-30}"      # total wait time
DEV_WAIT_STEP_SECS="${DEV_WAIT_STEP_SECS:-0.25}"          # poll interval
DEV_WAIT_PATH="${DEV_WAIT_PATH:-/readyz}"                 # fallback to /healthz if needed

# PIDs for cleanup.
MACRONODE_PID=""
SVC_ADMIN_PID=""
UI_PID=""

cleanup() {
  say "Shutting down dev stack..."
  for pid in "$UI_PID" "$SVC_ADMIN_PID" "$MACRONODE_PID"; do
    if [[ -n "${pid}" ]]; then
      kill "${pid}" 2>/dev/null || true
    fi
  done

  # Wait for children to exit to avoid zombie processes.
  wait 2>/dev/null || true
  say "Done."
}

# Ensure cleanup runs on Ctrl-C and also on script exit/errors.
trap cleanup EXIT INT TERM

wait_for_http_ok() {
  local base="http://${MACRONODE_HTTP_ADDR}"
  local path="${DEV_WAIT_PATH}"
  local url="${base}${path}"

  say "Waiting for macronode to respond at: ${url} (timeout: ${DEV_WAIT_TIMEOUT_SECS}s)"
  local deadline
  deadline="$(python3 - <<'PY'
import time
print(time.time())
PY
)"
  # Convert timeout to a "max tries" using step size (avoid bc).
  local max_tries
  max_tries="$(python3 - <<PY
import math
timeout=float("${DEV_WAIT_TIMEOUT_SECS}")
step=float("${DEV_WAIT_STEP_SECS}")
print(max(1, int(math.ceil(timeout/step))))
PY
)"

  for i in $(seq 1 "${max_tries}"); do
    if curl -sf "${url}" >/dev/null 2>&1; then
      say "macronode is ready (${url} OK)."
      return 0
    fi
    sleep "${DEV_WAIT_STEP_SECS}"
  done

  # Fallback attempt to /healthz if /readyz isn't implemented or is gated.
  if [[ "${DEV_WAIT_PATH}" == "/readyz" ]]; then
    local url2="${base}/healthz"
    say "Still not ready on /readyz; trying ${url2}..."
    for i in $(seq 1 10); do
      if curl -sf "${url2}" >/dev/null 2>&1; then
        say "macronode is responding on /healthz (treating as ready for dev)."
        return 0
      fi
      sleep "${DEV_WAIT_STEP_SECS}"
    done
  fi

  say "WARNING: macronode did not become ready within ${DEV_WAIT_TIMEOUT_SECS}s."
  say "         Continuing anyway (svc-admin samplers may log initial connection-refused)."
  return 0
}

cd "${ROOT_DIR}"

say "Project root: ${ROOT_DIR}"
say "macronode admin HTTP: ${MACRONODE_HTTP_ADDR}"
say "svc-admin UI/API:    ${SVC_ADMIN_HTTP_ADDR}"
say "svc-admin metrics:   ${SVC_ADMIN_METRICS_ADDR}"
say "SPA dev server:      http://localhost:5173"
say "Playground flag:     SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=${SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND}"
say "Wait-for-macronode:  ${DEV_WAIT_FOR_MACRONODE} (${DEV_WAIT_PATH}, ${DEV_WAIT_TIMEOUT_SECS}s timeout)"
echo

# ---------------------------------------------------------------------------
# 1) Start macronode (admin plane + metrics on same port)
# ---------------------------------------------------------------------------
(
  cd "${ROOT_DIR}"
  say "Starting macronode..."
  RON_HTTP_ADDR="${MACRONODE_HTTP_ADDR}" \
  RON_METRICS_ADDR="${MACRONODE_METRICS_ADDR}" \
  MACRONODE_DEV_INSECURE=1 \
  cargo run -p macronode
) &
MACRONODE_PID=$!
say "macronode PID: ${MACRONODE_PID}"

# Wait until macronode is answering (prevents sampler spam + cargo lock contention).
if [[ "${DEV_WAIT_FOR_MACRONODE}" == "true" ]]; then
  wait_for_http_ok
fi

# ---------------------------------------------------------------------------
# 2) Start svc-admin (backend API + metrics listener)
# ---------------------------------------------------------------------------
(
  cd "${ROOT_DIR}"
  say "Starting svc-admin..."
  SVC_ADMIN_HTTP_ADDR="${SVC_ADMIN_HTTP_ADDR}" \
  SVC_ADMIN_METRICS_ADDR="${SVC_ADMIN_METRICS_ADDR}" \
  SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND="${SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND}" \
  SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL="http://${MACRONODE_HTTP_ADDR}" \
  SVC_ADMIN_NODES__EXAMPLE_NODE__METRICS_URL="http://${MACRONODE_HTTP_ADDR}/metrics" \
  cargo run -p svc-admin --bin svc-admin
) &
SVC_ADMIN_PID=$!
say "svc-admin PID: ${SVC_ADMIN_PID}"

# ---------------------------------------------------------------------------
# 3) Start svc-admin UI (Vite dev server)
# ---------------------------------------------------------------------------
(
  cd "${ROOT_DIR}/crates/svc-admin/ui"
  say "Starting svc-admin UI (npm run dev)..."

  # Optional: isolate npm cache to avoid contention with other runs.
  # Uncomment if you keep seeing "Blocking waiting for file lock on package cache".
  # export NPM_CONFIG_CACHE="${ROOT_DIR}/.cache/npm"

  npm run dev
) &
UI_PID=$!
say "UI dev server PID: ${UI_PID}"

echo
say "Dev stack is starting up..."
say "Open the SPA in your browser at: http://localhost:5173"
say "Playground page (SPA):          http://localhost:5173/playground"
say "svc-admin API is at:            http://${SVC_ADMIN_HTTP_ADDR}"
say "macronode admin plane is at:    http://${MACRONODE_HTTP_ADDR}"

# Block until children exit (Ctrl-C to stop).
wait
