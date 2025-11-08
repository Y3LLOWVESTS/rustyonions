#!/usr/bin/env bash
# Smoke test for svc-passport (portable, macOS-safe, self-tracing)
# - Auto-spawns server if not healthy; reuses existing
# - Discovers actual bound URL from logs; falls back to host:port if absent
# - Checks aligned with current router: /healthz, /metrics, issue/verify/verify_batch
# - TRACE=1 enables bash -x and error context dump
# - All logs go to stderr so stdout is reserved for function returns (e.g., URL)

set -euo pipefail

# -------------------- knobs --------------------
PASSPORT_HOST="${PASSPORT_HOST:-127.0.0.1}"
PASSPORT_PORT="${PASSPORT_PORT:-5307}"             # fallback only; discovery preferred
STARTUP_TIMEOUT_SECS="${STARTUP_TIMEOUT_SECS:-25}"
DISABLE_AUTO_SPAWN="${DISABLE_AUTO_SPAWN:-0}"
LOGFILE="${LOGFILE:-/tmp/svc-passport.log}"
RUST_LOG="${RUST_LOG:-info}"
TRACE="${TRACE:-0}"
# ------------------------------------------------

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # -> crates/svc-passport
CFG_FILE_DEFAULT="$ROOT/config/default.toml"

SERVER_PID=""
: > "$LOGFILE"

# ---- tracing helpers ----
if [[ "$TRACE" == "1" ]]; then
  set -x
fi
on_err() {
  local code=$?
  echo "=== ERROR: script aborted (exit=$code) at line ${BASH_LINENO[0]} in ${BASH_SOURCE[1]-main} ===" >&2
  echo "--- Last 60 lines of ${LOGFILE} ---" >&2
  tail -n 60 "$LOGFILE" 2>/dev/null || true
  exit "$code"
}
trap on_err ERR

# log helpers -> stderr (so stdout can carry function return values)
log()  { printf "\033[1;34m[INFO]\033[0m %s\n" "$*" >&2; }
ok()   { printf "\033[1;32m[ OK ]\033[0m %s\n" "$*" >&2; }
warn() { printf "\033[1;33m[WARN]\033[0m %s\n" "$*" >&2; }
err()  { printf "\033[1;31m[FAIL]\033[0m %s\n" "$*" >&2; }

need() { command -v "$1" >/dev/null 2>&1 || { err "Missing tool: $1"; exit 1; }; }
need curl
need jq
need awk
need grep
need sed
command -v lsof >/dev/null 2>&1 || warn "lsof not found; port freeing limited"

cleanup() {
  if [[ -n "${SERVER_PID}" ]]; then
    log "Stopping spawned svc-passport (pid=${SERVER_PID})"
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# -------------------- config --------------------
prepare_config_env() {
  # Respect caller-supplied config if present
  if [[ -n "${PASSPORT_CONFIG_FILE:-}" || -n "${PASSPORT_CONFIG:-}" ]]; then
    return
  fi
  if [[ -f "$CFG_FILE_DEFAULT" ]]; then
    export PASSPORT_CONFIG_FILE="$CFG_FILE_DEFAULT"
    return
  fi
  err "No config provided and default missing: $CFG_FILE_DEFAULT"
  exit 1
}

# -------------------- ports --------------------
port_bound() {
  local port="$1"
  if command -v lsof >/dev/null 2>&1; then
    lsof -iTCP:"$port" -sTCP:LISTEN -n -P >/dev/null 2>&1
  else
    (exec 3<>/dev/tcp/"$PASSPORT_HOST"/"$port") >/dev/null 2>&1 || return 1
    exec 3>&- || true
    return 0
  fi
}

free_fixed_ports_if_needed() {
  # Best-effort only; defensive for macOS (no xargs -r)
  for PORT in "$PASSPORT_PORT" 5308; do
    if port_bound "$PORT"; then
      warn "Port $PORT busy. Killing holder(s)…"
      if command -v lsof >/dev/null 2>&1; then
        PIDS="$(lsof -tiTCP:"$PORT" -sTCP:LISTEN -n -P 2>/dev/null || true)"
        if [[ -n "${PIDS:-}" ]]; then
          # shellcheck disable=SC2086
          kill -9 $PIDS 2>/dev/null || true
        fi
      fi
      sleep 0.2
    fi
  done
}

# -------------------- URL discovery --------------------
discover_url_from_logs() {
  # Expect startup to log e.g. "svc-passport: listening on http://127.0.0.1:5307"
  # Extract the last http://IP:PORT seen
  local found
  found="$(grep -Eo 'http://[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+' "$LOGFILE" | tail -n1 || true)"
  printf "%s" "${found}"
}
fallback_url() { echo "http://${PASSPORT_HOST}:${PASSPORT_PORT}"; }

is_healthy() {
  local base="$1"
  [[ -z "$base" ]] && return 1
  curl -sSf "$base/healthz" >/dev/null 2>&1
}

# -------------------- spawn & wait --------------------
spawn() {
  log "Spawning svc-passport (cargo run -p svc-passport)"
  : > "$LOGFILE"

  # If PASSPORT_CONFIG exists but is empty, unset it so file mode is used.
  if [[ "${PASSPORT_CONFIG+x}" == "x" && -z "${PASSPORT_CONFIG:-}" ]]; then
    unset PASSPORT_CONFIG
  fi

  # Build an env block that only includes non-empty config vars
  (
    export RUST_LOG="$RUST_LOG"
    if [[ -n "${PASSPORT_CONFIG_FILE:-}" ]]; then
      export PASSPORT_CONFIG_FILE
    fi
    if [[ -n "${PASSPORT_CONFIG:-}" ]]; then
      export PASSPORT_CONFIG
    fi
    cargo run -p svc-passport --quiet
  ) >"$LOGFILE" 2>&1 &
  SERVER_PID=$!
  log "svc-passport spawned (pid=${SERVER_PID}); logs at ${LOGFILE}"
}

maybe_spawn() {
  if [[ "${DISABLE_AUTO_SPAWN}" == "1" ]]; then
    warn "Auto-spawn disabled; expecting service to be up."
    return 1
  fi
  local try_url; try_url="$(fallback_url)"
  if is_healthy "$try_url"; then
    log "Target already healthy at ${try_url}; not spawning."
    return 1
  fi
  if port_bound "${PASSPORT_PORT}"; then
    log "Port ${PASSPORT_PORT} bound; reusing existing process (will wait for health)."
    return 1
  fi
  spawn
  return 0
}

wait_for_up() {
  log "Waiting for service…"
  local url=""
  local deadline=$((SECONDS + STARTUP_TIMEOUT_SECS))
  while [[ $SECONDS -lt $deadline ]]; do
    url="$(discover_url_from_logs || true)"
    if [[ -z "$url" ]]; then
      url="$(fallback_url)"
    fi
    if is_healthy "$url"; then
      ok "Service is up at ${url}"
      # IMPORTANT: stdout returns ONLY the URL
      echo "$url"
      return 0
    fi
    if [[ -n "$SERVER_PID" ]] && ! kill -0 "$SERVER_PID" 2>/dev/null; then
      err "Process exited before becoming healthy. Last 200 log lines:"
      tail -n 200 "$LOGFILE" >&2 || true
      exit 1
    fi
    sleep 0.2
  done
  err "Timed out waiting for /healthz"
  warn "Last 200 lines of ${LOGFILE}:"
  tail -n 200 "$LOGFILE" >&2 || true
  exit 1
}

# -------------------- HTTP helpers --------------------
curl_save() {
  local url="$1" method="$2" path="$3" body="${4:-}" out="${5:-/dev/null}"
  if [[ -n "$body" ]]; then
    curl -sS -X "$method" -H 'content-type: application/json' -d "$body" -o "$out" -w '%{http_code}' "${url}${path}"
  else
    curl -sS -X "$method" -o "$out" -w '%{http_code}' "${url}${path}"
  fi
}

run_checks() {
  local url="$1" spawned="$2"
  local code failures=0 tmp body ENV ENV_FILE

  # Basics (aligned with current router)
  code=$(curl -s -o /dev/null -w '%{http_code}' "${url}/healthz") && [[ "$code" == "200" ]] \
    && ok "/healthz" || { err "/healthz (code=$code)"; failures=$((failures+1)); }

  if curl -sSf "${url}/metrics" >/dev/null 2>&1; then ok "/metrics"; else warn "/metrics (not present)"; fi

  # Issue
  ENV_FILE="$(mktemp)"
  code=$(curl_save "$url" POST "/v1/passport/issue" '{"hello":"world"}' "$ENV_FILE")
  [[ "$code" == "200" ]] && ok "issue → envelope" || { err "issue (code=$code)"; failures=$((failures+1)); }
  ENV="$(cat "$ENV_FILE")"; rm -f "$ENV_FILE"

  # Verify single
  tmp="$(mktemp)"
  code=$(curl_save "$url" POST "/v1/passport/verify" "$ENV" "$tmp"); body="$(cat "$tmp")"; rm -f "$tmp"
  [[ "$code" == "200" ]] && jq -e '. == true' >/dev/null 2>&1 <<<"$body" \
    && ok "verify (single)" || { err "verify single (code=$code body=$body)"; failures=$((failures+1)); }

  # Verify batch
  tmp="$(mktemp)"
  code=$(curl_save "$url" POST "/v1/passport/verify_batch" "[$ENV,$ENV]" "$tmp"); body="$(cat "$tmp")"; rm -f "$tmp"
  [[ "$code" == "200" ]] && jq -e '. == [true,true]' >/dev/null 2>&1 <<<"$body" \
    && ok "verify (batch)" || { err "verify batch (code=$code body=$body)"; failures=$((failures+1)); }

  # Negative: tamper → 200:false or 400
  local ENV_TAMPER code_tamper BODY_TAMPER body_tamper_file
  ENV_TAMPER="$(echo "$ENV" | jq '.msg_b64="e30"')" || ENV_TAMPER=''
  if [[ -n "$ENV_TAMPER" ]]; then
    body_tamper_file="$(mktemp)"
    code_tamper=$(curl_save "$url" POST "/v1/passport/verify" "$ENV_TAMPER" "$body_tamper_file")
    BODY_TAMPER="$(cat "$body_tamper_file" || true)"; rm -f "$body_tamper_file"
    if [[ "$code_tamper" == "200" ]] && echo "$BODY_TAMPER" | jq -e '. == false' >/dev/null 2>&1; then
      ok "negative: tampered → 200 false"
    elif [[ "$code_tamper" == "400" ]] ; then
      ok "negative: tampered → 400"
    else
      err "negative: tampered expected 200:false or 400; got ${code_tamper:-<none>}"
      failures=$((failures+1))
    fi
  fi

  (( failures == 0 )) && { ok "All checks passed"; return 0; }
  err "Smoke FAILED (${failures} failing check(s))"; return 1
}

# -------------------- main --------------------
{ set +e; echo "--- TRACE: env ---" >&2; env | grep -E '^PASSPORT_|^RUST_LOG' >&2 || true; echo "--------------" >&2; set -e; }

prepare_config_env
free_fixed_ports_if_needed

spawned=0
if maybe_spawn; then spawned=1; fi

log "--- REACHED: before wait_for_up (spawned=${spawned}) ---"

URL="$(wait_for_up)"
run_checks "$URL" "$spawned"
