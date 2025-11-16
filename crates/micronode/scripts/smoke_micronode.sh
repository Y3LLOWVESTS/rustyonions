#!/usr/bin/env bash
# smoke_micronode.sh — one-shot smoke test for Micronode
# Modes:
#   (default)   — assumes micronode already running on ADDR (two-terminal workflow)
#   --spawn     — kills port holder on 5310, spawns micronode in background, runs checks, then cleans up
#
# Env knobs:
#   ADDR                 — admin/API addr (default 127.0.0.1:5310)
#   RUST_LOG             — log level when spawning (default info,micronode=debug)
#   MICRONODE_DEV_ROUTES — when spawning, enable dev routes (default 1)

set -euo pipefail

ADDR="${ADDR:-127.0.0.1:5310}"
RUST_LOG="${RUST_LOG:-info,micronode=debug}"

SPAWN=0
if [[ "${1:-}" == "--spawn" ]]; then
  SPAWN=1
fi

# Utilities (macOS-friendly)
die() { echo "[ERR] $*" >&2; exit 1; }
info() { echo "[INFO] $*"; }
ok() { echo "[OK] $*"; }
step() { echo "[STEP] $*"; }

kill_port_holders() {
  local port="$1"
  local pids
  if pids=$(lsof -ti tcp:"$port" 2>/dev/null); then
    if [[ -n "$pids" ]]; then
      info "Port $port busy. Killing holder(s)…"
      # shellcheck disable=SC2086
      kill -9 $pids || true
      sleep 0.2
    fi
  fi
}

wait_for_healthz() {
  local url="$1"
  local retries="${2:-60}" # ~60s max
  local i=0
  info "Waiting for $url ..."
  until curl -sSf -o /dev/null "$url"; do
    i=$((i+1)) || true
    if [[ "$i" -ge "$retries" ]]; then
      die "Timed out waiting for $url"
    fi
    sleep 1
  done
  ok "Healthy: $url"
}

MICRO_PID=""
LOG_FILE=""

cleanup() {
  if [[ -n "$MICRO_PID" ]]; then
    info "Killing micronode (pid=$MICRO_PID)…"
    kill "$MICRO_PID" 2>/dev/null || true
  fi
  if [[ -n "$LOG_FILE" && -f "$LOG_FILE" ]]; then
    info "Micronode logs were captured in: $LOG_FILE"
  fi
}
trap cleanup EXIT

if [[ "$SPAWN" -eq 1 ]]; then
  info "Spawn mode: will start micronode on ${ADDR}"
  # Assume ADDR is host:port; we only care about the port for kill_port_holders
  PORT="${ADDR##*:}"
  kill_port_holders "$PORT"

  LOG_FILE="$(mktemp -t micronode-smoke-XXXX.log)"
  info "Spawning micronode (logs -> $LOG_FILE)…"

  MICRONODE_DEV_ROUTES="${MICRONODE_DEV_ROUTES:-1}" \
  RUST_LOG="$RUST_LOG" \
    cargo run -p micronode >"$LOG_FILE" 2>&1 &

  MICRO_PID=$!
  sleep 0.5
fi

BASE_URL="http://${ADDR}"

step "Admin plane checks"
wait_for_healthz "${BASE_URL}/healthz"

info "GET /metrics (head)"
curl -sSf "${BASE_URL}/metrics" | head -n 20 >/tmp/micronode_metrics_head.$$ || die "/metrics not reachable"
ok "/metrics reachable"

# Optional: assert our http metrics family is present
if curl -sSf "${BASE_URL}/metrics" | grep -q "micronode_http_requests_total"; then
  ok "micronode_http_requests_total present in /metrics"
else
  info "micronode_http_requests_total not seen yet (may appear after more traffic)"
fi

step "Readiness (/readyz)"
curl -sSf "${BASE_URL}/readyz" | jq . || die "/readyz failed"
ok "/readyz returned JSON"

step "Version (/version)"
curl -sSf "${BASE_URL}/version" | jq . || die "/version failed"
ok "/version returned JSON"

step "KV roundtrip via /v1/kv/{bucket}/{key}"
BUCKET="${BUCKET:-smoke}"
KEY="${KEY:-k}"
VALUE="${VALUE:-hello-micronode}"

PUT_CODE=$(curl -sS -o /dev/null -w "%{http_code}" \
  -X PUT "${BASE_URL}/v1/kv/${BUCKET}/${KEY}" \
  -H 'content-type: application/octet-stream' \
  --data-binary "${VALUE}" || true)

echo "[INFO] PUT status: ${PUT_CODE}"
if [[ "${PUT_CODE}" != "201" && "${PUT_CODE}" != "204" ]]; then
  die "Expected 201/204 from PUT, got ${PUT_CODE}"
fi
ok "PUT /v1/kv/${BUCKET}/${KEY} → ${PUT_CODE}"

GET_BODY=$(curl -sS "${BASE_URL}/v1/kv/${BUCKET}/${KEY}" || true)
echo "[INFO] GET body: ${GET_BODY}"
if [[ "${GET_BODY}" != "${VALUE}" ]]; then
  die "Expected GET body '${VALUE}', got '${GET_BODY}'"
fi
ok "GET /v1/kv/${BUCKET}/${KEY} roundtrip OK"

DEL_CODE=$(curl -sS -o /dev/null -w "%{http_code}" \
  -X DELETE "${BASE_URL}/v1/kv/${BUCKET}/${KEY}" || true)

echo "[INFO] DELETE status: ${DEL_CODE}"
if [[ "${DEL_CODE}" != "204" ]]; then
  die "Expected 204 from DELETE, got ${DEL_CODE}"
fi
ok "DELETE /v1/kv/${BUCKET}/${KEY} → ${DEL_CODE}"

echo "✅ micronode smoke OK"
