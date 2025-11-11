#!/usr/bin/env bash
# smoke_registry.sh — one-shot smoke test for svc-registry
# Modes:
#   (default)   — assumes service already running (two-terminal workflow)
#   --spawn     — kills port holders, spawns svc-registry in background, runs checks, then cleans up

set -euo pipefail

ADMIN_ADDR="${ADMIN_ADDR:-127.0.0.1:9909}"
API_ADDR="${API_ADDR:-127.0.0.1:9444}"
RUST_LOG="${RUST_LOG:-info}"
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
    ((i++)) || true
    if (( i >= retries )); then
      return 1
    fi
    sleep 1
  done
  ok "$url"
  return 0
}

# ── Metric helpers ──────────────────────────────────────────────────────────────
sum_head_requests() {
  # Sum requests_total counter values for route="/registry/head" across all methods/statuses
  curl -s "http://${ADMIN_ADDR}/metrics" \
  | awk -F' ' '/^requests_total\{.*route="\/registry\/head"/ { v=$NF; gsub(/\r/,"",v); s+=v } END { if (s=="") s=0; print s+0 }'
}

# Globals for --spawn
LOG_FILE="/tmp/svc-registry.log"
SVC_PID=""

spawn_service() {
  kill_port_holders "${ADMIN_ADDR##*:}"
  kill_port_holders "${API_ADDR##*:}"

  info "Spawning svc-registry (cargo run -p svc-registry)"
  : > "$LOG_FILE"
  if command -v setsid >/dev/null 2>&1; then
    RUST_LOG="$RUST_LOG" setsid bash -c 'cargo run -p svc-registry' > "$LOG_FILE" 2>&1 &
  else
    RUST_LOG="$RUST_LOG" bash -c 'cargo run -p svc-registry' > "$LOG_FILE" 2>&1 &
  fi
  SVC_PID="$!"
  info "svc-registry spawned (pid=${SVC_PID}); logs at $LOG_FILE"

  # Robust wait: “listening” log or /healthz 200
  local waited=0
  local max_wait=90
  while (( waited < max_wait )); do
    if grep -q '"message":"listening"' "$LOG_FILE" || grep -q 'listening' "$LOG_FILE"; then
      break
    fi
    if curl -sSf -o /dev/null "http://${ADMIN_ADDR}/healthz"; then
      break
    fi
    sleep 1
    ((waited++)) || true
  done

  if ! curl -sSf -o /dev/null "http://${ADMIN_ADDR}/healthz"; then
    if ! wait_for_healthz "http://${ADMIN_ADDR}/healthz" 10; then
      echo "----- last 120 log lines -----"
      tail -n 120 "$LOG_FILE" || true
      die "svc-registry failed to come up"
    fi
  fi
}

cleanup_spawn() {
  if [[ -n "${SVC_PID:-}" ]]; then
    info "Stopping svc-registry (pid=${SVC_PID})"
    kill -TERM "${SVC_PID}" 2>/dev/null || true
    sleep 0.5
    kill -9 "${SVC_PID}" 2>/dev/null || true
  fi
}

trap '[[ $SPAWN -eq 1 ]] && cleanup_spawn' EXIT

# If requested, spawn the service
if [[ $SPAWN -eq 1 ]]; then
  spawn_service
fi

# ────────────────────────────────────────────────────────────────────────────────
# Admin plane checks
# ────────────────────────────────────────────────────────────────────────────────
step "Admin plane checks"
wait_for_healthz "http://${ADMIN_ADDR}/healthz" 60

# Ready (truthful)
curl -is "http://${ADMIN_ADDR}/readyz" | sed -n '1,5p'
curl -s "http://${ADMIN_ADDR}/readyz" | jq -c . || die "jq required"

# Version
curl -s "http://${ADMIN_ADDR}/version" | jq -c . || die "jq required"

# Metrics (peek)
step "Metrics head (first 25 lines)"
curl -s "http://${ADMIN_ADDR}/metrics" | head -n 25

# ────────────────────────────────────────────────────────────────────────────────
# API: read head, drive some requests to move metrics, then commit
# ────────────────────────────────────────────────────────────────────────────────
step "API: /registry/head (capture current head)"
HEAD_JSON="$(curl -s "http://${API_ADDR}/registry/head")"
echo "HEAD: ${HEAD_JSON}"
HEAD_VER="$(echo "$HEAD_JSON" | jq -r '.version')"

step "Drive a few requests, then assert metrics moved"
before_val="$(sum_head_requests)"
# Hit /registry/head 5x
for _ in 1 2 3 4 5; do curl -s "http://${API_ADDR}/registry/head" >/dev/null; done
sleep 0.2
after_val="$(sum_head_requests)"
echo "[INFO] requests_total value before=${before_val} after=${after_val}"
delta=$(( ${after_val%.*} - ${before_val%.*} ))
if (( delta >= 5 )); then
  echo "✅ metrics advanced by >= 5 (delta=${delta})"
else
  die "requests_total did not advance as expected (delta=${delta})"
fi

# ────────────────────────────────────────────────────────────────────────────────
# SSE: start stream reader, commit, verify bump and event
# ────────────────────────────────────────────────────────────────────────────────
step "SSE smoke: start stream reader (capture a few lines incl. commits)"
SSE_LOG="$(mktemp -t svc-registry-sse.XXXXXX)"
curl -sN "http://${API_ADDR}/registry/stream" > "${SSE_LOG}" 2>&1 &
SSE_PID="$!"
sleep 0.3

step "POST /registry/commit and verify version bump"
RAND="$(LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 16 || echo XxXxXxXxXxXxXxXx)"
RESP="$(curl -s -X POST "http://${API_ADDR}/registry/commit" \
  -H 'content-type: application/json' \
  --data "{\"payload_b3\":\"b3:${RAND}\"}")"
echo "COMMIT RESP: ${RESP}"
NEW_VER="$(echo "$RESP" | jq -r '.version')"
if [[ "$NEW_VER" != "null" ]] && (( NEW_VER == HEAD_VER + 1 )); then
  echo "✅ head.version bumped by +1"
else
  die "commit did not bump version (+1)"
fi

step "Verify /registry/head reflects the new version"
HEAD2="$(curl -s "http://${API_ADDR}/registry/head")"
echo "HEAD NOW: ${HEAD2}"
NEW_HEAD_VER="$(echo "$HEAD2" | jq -r '.version')"
if (( NEW_HEAD_VER == NEW_VER )); then
  echo "✅ /registry/head matches committed version"
else
  die "/registry/head mismatch"
fi

step "Wait briefly and check SSE log for a commit event"
sleep 0.5
kill -TERM "${SSE_PID}" 2>/dev/null || true
sleep 0.2
kill -KILL "${SSE_PID}" 2>/dev/null || true

if grep -q '^event: commit' "${SSE_LOG}"; then
  ok "SSE commit event observed"
else
  echo "----- SSE LOG (tail) -----"
  tail -n 120 "${SSE_LOG}" || true
  die "SSE commit event not observed"
fi

step "SSE heartbeat (optional quick check)"
grep -m1 '^event: heartbeat' "${SSE_LOG}" >/dev/null 2>&1 && echo "event: heartbeat" || true

echo "✅ svc-registry smoke OK"
