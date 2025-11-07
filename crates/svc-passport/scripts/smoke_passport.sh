#!/usr/bin/env bash
# Smoke test for svc-passport (portable & robust)
# - Auto-spawns server if not healthy; reuses if already up or port is bound
# - Checks: /healthz, /v1/keys, issue, verify, verify_batch, rotate, attest, /metrics (optional)
# - Negative checks:
#     * tampered envelope → accept 200:false OR 400 (invalid)
#     * over-limit → expect 413 only if we spawned the server (cap set)

set -euo pipefail

PASSPORT_HOST="${PASSPORT_HOST:-127.0.0.1}"
PASSPORT_PORT="${PASSPORT_PORT:-5307}"
PASSPORT_URL="${PASSPORT_URL:-http://${PASSPORT_HOST}:${PASSPORT_PORT}}"
STARTUP_TIMEOUT_SECS="${STARTUP_TIMEOUT_SECS:-20}"
DISABLE_AUTO_SPAWN="${DISABLE_AUTO_SPAWN:-0}"

SERVER_PID=""
LOGFILE="/tmp/svc-passport.log"
: > "${LOGFILE}"

log()  { printf "\033[1;34m[INFO]\033[0m %s\n" "$*"; }
ok()   { printf "\033[1;32m[ OK ]\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m[WARN]\033[0m %s\n" "$*"; }
err()  { printf "\033[1;31m[FAIL]\033[0m %s\n" "$*"; }

need_tool() { command -v "$1" >/dev/null 2>&1 || { err "Missing required tool: $1"; exit 1; }; }

cleanup() {
  if [[ -n "${SERVER_PID}" ]]; then
    log "Stopping spawned svc-passport (pid=${SERVER_PID})"
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

need_tool curl
need_tool jq

is_healthy() { curl -sSf "${PASSPORT_URL}/healthz" >/dev/null 2>&1; }

port_bound() {
  if command -v lsof >/dev/null 2>&1; then
    lsof -iTCP:"${PASSPORT_PORT}" -sTCP:LISTEN >/dev/null 2>&1
  else
    (echo >/dev/tcp/"${PASSPORT_HOST}"/"${PASSPORT_PORT}") >/dev/null 2>&1 || return 1
    return 0
  fi
}

spawn() {
  log "Spawning svc-passport (cargo run -p svc-passport)"
  : > "${LOGFILE}"
  PASSPORT_MAX_MSG_BYTES="${PASSPORT_MAX_MSG_BYTES:-2048}" \
  RUST_LOG="${RUST_LOG:-info}" \
  cargo run -p svc-passport --quiet >"${LOGFILE}" 2>&1 &
  SERVER_PID=$!
  log "svc-passport spawned (pid=${SERVER_PID}); logs at ${LOGFILE}"
}

maybe_spawn() {
  if [[ "${DISABLE_AUTO_SPAWN}" == "1" ]]; then
    warn "Auto-spawn disabled; expecting service to be up at ${PASSPORT_URL}"
    return 1
  fi
  if is_healthy; then
    log "Target already healthy at ${PASSPORT_URL}; not spawning."
    return 1
  fi
  if port_bound; then
    log "Port ${PASSPORT_PORT} bound; reusing existing process (will wait for health)."
    return 1
  fi
  spawn
  return 0
}

wait_for_up() {
  local deadline=$((SECONDS + STARTUP_TIMEOUT_SECS))
  while true; do
    if is_healthy; then
      ok "Service is up at ${PASSPORT_URL}"
      return 0
    fi
    if [[ -n "${SERVER_PID}" ]] && ! kill -0 "${SERVER_PID}" 2>/dev/null; then
      err "svc-passport process exited early (pid=${SERVER_PID})"
      if [[ -s "${LOGFILE}" ]]; then
        warn "Last 120 lines of ${LOGFILE}:"
        tail -n 120 "${LOGFILE}" || true
      fi
      return 1
    fi
    if (( SECONDS > deadline )); then
      err "Timed out waiting for ${PASSPORT_URL}/healthz"
      if [[ -s "${LOGFILE}" ]]; then
        warn "Last 120 lines of ${LOGFILE}:"
        tail -n 120 "${LOGFILE}" || true
      fi
      return 1
    fi
    sleep 0.2
  done
}

# Save BODY to an explicit OUTFILE and echo HTTP CODE to stdout
# Usage: code=$(curl_save METHOD PATH BODY OUTFILE)
curl_save() {
  local method="$1" path="$2" body="${3:-}" outfile="$4" code
  if [[ -n "$body" ]]; then
    code=$(curl -sS -o "$outfile" -w "%{http_code}" -X "$method" \
           "${PASSPORT_URL}${path}" -H 'content-type: application/json' -d "$body" || true)
  else
    code=$(curl -sS -o "$outfile" -w "%{http_code}" -X "$method" \
           "${PASSPORT_URL}${path}" || true)
  fi
  printf "%s" "$code"
}

run_checks() {
  local failures=0 spawned="${1:-0}"
  log "Target: ${PASSPORT_URL}"

  # 1) Health
  if curl -sS -f "${PASSPORT_URL}/healthz" | jq -e '.ok == true' >/dev/null; then ok "/healthz"; else err "/healthz"; failures=$((failures+1)); fi

  # 2) Keys
  if curl -sS -f "${PASSPORT_URL}/v1/keys" | jq -e 'has("keys")' >/dev/null; then ok "/v1/keys"; else err "/v1/keys"; failures=$((failures+1)); fi

  # 3) Issue → Envelope
  local ENV_FILE ENV code
  ENV_FILE="$(mktemp)"; code=$(curl_save POST "/v1/passport/issue" '{"hello":"world"}' "$ENV_FILE"); ENV="$(cat "$ENV_FILE")"; rm -f "$ENV_FILE"
  if [[ "$code" == "200" ]] && echo "$ENV" | jq -e '.alg and .kid and .sig_b64 and .msg_b64' >/dev/null; then
    ok "issue → envelope"
  else
    err "issue → envelope (code=$code)"
    failures=$((failures+1))
  fi

  # 4) Verify (single)
  local code_verify body_tmp
  body_tmp="$(mktemp)"
  code_verify=$(curl_save POST "/v1/passport/verify" "$ENV" "$body_tmp"); rm -f "$body_tmp"
  if [[ "$code_verify" == "200" ]]; then ok "verify (single)"; else err "verify (single) (code=$code_verify)"; failures=$((failures+1)); fi

  # 5) Verify (batch)
  local BATCH code_batch body_tmp2
  BATCH="$(jq -n --argjson a "${ENV}" --argjson b "${ENV}" '[ $a, $b ]')"
  body_tmp2="$(mktemp)"
  code_batch=$(curl_save POST "/v1/passport/verify_batch" "$BATCH" "$body_tmp2"); rm -f "$body_tmp2"
  if [[ "$code_batch" == "200" ]]; then ok "verify (batch)"; else err "verify (batch) (code=$code_batch)"; failures=$((failures+1)); fi

  # 6) Rotate
  local code_rotate tmp
  tmp="$(mktemp)"
  code_rotate=$(curl_save POST "/admin/rotate" "" "$tmp"); rm -f "$tmp"
  if [[ "$code_rotate" == "200" ]]; then ok "admin/rotate"; else err "admin/rotate (code=$code_rotate)"; failures=$((failures+1)); fi

  # 7) Attest
  tmp="$(mktemp)"
  local code_attest; code_attest=$(curl_save GET "/admin/attest" "" "$tmp"); rm -f "$tmp"
  if [[ "$code_attest" == "200" ]]; then ok "admin/attest"; else err "admin/attest (code=$code_attest)"; failures=$((failures+1)); fi

  # 8) Optional: metrics
  if curl -sSf "${PASSPORT_URL}/metrics" >/dev/null 2>&1; then ok "/metrics (present)"; else warn "/metrics (not present)"; fi

  # 9) NEGATIVE: tamper → accept 200:false OR 400 (invalid)
  local ENV_TAMPER code_tamper body_tamper_file BODY_TAMPER
  ENV_TAMPER="$(echo "$ENV" | jq '.msg_b64="e30"')" # "{}"
  body_tamper_file="$(mktemp)"
  code_tamper=$(curl_save POST "/v1/passport/verify" "$ENV_TAMPER" "$body_tamper_file")
  BODY_TAMPER="$(cat "$body_tamper_file" || true)"; rm -f "$body_tamper_file"
  if [[ "$code_tamper" == "200" ]] && echo "$BODY_TAMPER" | jq -e '. == false' >/dev/null 2>&1; then
    ok "negative: tampered → 200 false"
  elif [[ "$code_tamper" == "400" ]]; then
    ok "negative: tampered → 400"
  else
    err "negative: tampered expected 200:false or 400; got ${code_tamper}"
    failures=$((failures+1))
  fi

  # 10) NEGATIVE: over-limit → 413 only if we spawned (cap set)
  if [[ "$spawned" == "1" ]]; then
    local BIG code_big big_tmp
    BIG=$(python - <<'PY' 2>/dev/null || true
s = "A" * 4096
print('{"pad":"%s"}' % s)
PY
)
    [[ -z "$BIG" ]] && BIG='{"pad":"'"$(head -c 4096 </dev/zero | tr "\0" "A")"'"}'
    big_tmp="$(mktemp)"
    code_big=$(curl_save POST "/v1/passport/issue" "$BIG" "$big_tmp"); rm -f "$big_tmp"
    if [[ "$code_big" == "413" ]]; then ok "negative: over-limit → 413"; else err "negative: over-limit expected 413 got ${code_big:-<none>}"; failures=$((failures+1)); fi
  else
    warn "skipping over-limit check (service not spawned, cap unknown)"
  fi

  (( failures == 0 )) && { ok "All checks passed"; return 0; }
  err "Smoke FAILED (${failures} failing check(s))"; return 1
}

spawned=0
if maybe_spawn; then spawned=1; fi
wait_for_up
run_checks "$spawned"
