### GROK HELPED WITH THIS 

#!/usr/bin/env bash
set -euo pipefail

# --- config (overridable via env) ---
BIN="${BIN:-svc-storage}"
ADDR="${ADDR:-127.0.0.1:5303}"          # address the server should bind to
WAIT_SECS="${WAIT_SECS:-20}"            # max seconds to wait for server up
LOG_FILE="${LOG_FILE:-/tmp/${BIN}.log}" # server stdout/stderr
CARGO_FEATURES="${CARGO_FEATURES:-}"    # e.g. "--features metrics"

say()  { printf '%s\n' "$*"; }
fail() { printf '❌ %s\n' "$*" >&2; exit 1; }

# --- find repo root by walking up until we see a workspace Cargo.toml ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
find_repo_root() {
  local d="${SCRIPT_DIR}"
  while :; do
    if [ -f "${d}/Cargo.toml" ] && grep -q '^\[workspace\]' "${d}/Cargo.toml"; then
      printf '%s\n' "${d}"
      return 0
    fi
    local parent
    parent="$(dirname "${d}")"
    [ "${parent}" = "${d}" ] && break
    d="${parent}"
  done
  # Fallback: use current working dir (should still work if we're inside the repo)
  printf '%s\n' "$(pwd)"
}
REPO_ROOT="$(find_repo_root)"
cd "${REPO_ROOT}"

# --- build ---
say "building ${BIN}…"
cargo build -p "${BIN}" >/dev/null

# --- start server ---
: > "${LOG_FILE}"
say "starting ${BIN} at ${ADDR}… (logs: ${LOG_FILE})"
RUST_LOG="${RUST_LOG:-debug}" \
ADDR="${ADDR}" \
cargo run -p "${BIN}" ${CARGO_FEATURES} >"${LOG_FILE}" 2>&1 &
SERVER_PID=$!

cleanup() {
  if kill -0 "${SERVER_PID}" 2>/dev/null; then
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# --- wait for HTTP to be responsive ---
say "waiting for http://${ADDR}…"
UP=0
end_at=$(( $(date +%s) + WAIT_SECS ))
while (( $(date +%s) < end_at )); do
  # Prefer /healthz if present; otherwise any HTTP status on / indicates the server is up.
  HC="$(curl -s --fail --show-error -o /dev/null -w '%{http_code}' "http://${ADDR}/healthz" || true)"
  if [[ "${HC}" == "200" ]]; then
    say "✅ ${BIN} is up (GET /healthz -> 200)"
    UP=1; break
  fi
  ROOTC="$(curl -s --fail --show-error -o /dev/null -w '%{http_code}' "http://${ADDR}/" || true)"
  if [[ "${ROOTC}" =~ ^[1-5][0-9]{2}$ ]]; then
    say "✅ ${BIN} is up (GET / -> ${ROOTC})"
    UP=1; break
  fi
  if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
    fail "${BIN} failed to start (process exited)
---- server log ----
$(tail -n +1 "${LOG_FILE}")"
  fi
  sleep 0.2
done

if [[ "${UP}" != "1" ]]; then
  fail "timed out waiting for ${BIN}
---- server log ----
$(tail -n 200 "${LOG_FILE}")"
fi

# --- tests ---
PASS=0; FAIL=0; SKIP=0
_step() { printf "\n-- %s --\n" "$*"; }

# 1) POST object (use POST, not PUT)
_step "POST object"
CID=$(printf 'hello world' | curl -sS --fail --show-error -X POST --data-binary @- "http://${ADDR}/o" | jq -r .cid)
if [[ -n "${CID:-}" && "${CID}" == b3:* ]]; then
  say "✅ POST returned cid=${CID}"; ((PASS++))
else
  say "❌ POST failed (cid='${CID:-}')" ; ((FAIL++))
fi

# 2) HEAD object (expect 200, length=11, etag present)
_step "HEAD object"
H="$(curl -sSI --fail --show-error "http://${ADDR}/o/${CID}")" || true
echo "${H}" | grep -q "^HTTP/.* 200"       && { say "✅ HEAD 200"; ((PASS++)); } || { say "❌ HEAD not 200"; ((FAIL++)); }
echo "${H}" | grep -iq "^Content-Length: 11" && { say "✅ HEAD Content-Length=11"; ((PASS++)); } || { say "❌ HEAD missing/incorrect Content-Length"; ((FAIL++)); }
echo "${H}" | grep -iq "^ETag:"            && { say "✅ HEAD ETag present"; ((PASS++)); } || { say "❌ HEAD missing ETag"; ((FAIL++)); }

# 3) GET full
_step "GET full"
BODY="$(curl -s --fail --show-error "http://${ADDR}/o/${CID}")"
[[ "${BODY}" == "hello world" ]] && { say "✅ GET full body matches"; ((PASS++)); } || { say "❌ GET full body mismatch: '${BODY}'"; ((FAIL++)); }

# 4) Range GET
_step "Range GET bytes=0-4"
RANGE_BODY="$(curl -s --fail --show-error -H 'Range: bytes=0-4' -i "http://${ADDR}/o/${CID}")"
echo "${RANGE_BODY}" | head -n1 | grep -q "206" && { say "✅ Range GET 206"; ((PASS++)); } || { say "❌ Range GET not 206"; ((FAIL++)); }
echo "${RANGE_BODY}" | tail -n1 | grep -q "^hello$" && { say "✅ Range body matches 'hello'"; ((PASS++)); } || { say "❌ Range body mismatch: '$(echo "${RANGE_BODY}" | tail -n1)'"; ((FAIL++)); }

# 5) GET unknown -> 404 (use valid-formatted but fake CID)
FAKE_CID="b3:$(printf '%064s' '' | tr ' ' '0')"  # b3:000...0 (64 zeros)
_step "GET unknown"
UNKNOWN_CODE="$(curl -s --fail --show-error -o /dev/null -w '%{http_code}' "http://${ADDR}/o/${FAKE_CID}" || true)"
[[ "${UNKNOWN_CODE}" == "404" ]] && { say "✅ GET unknown cid -> 404"; ((PASS++)); } || { say "❌ GET unknown returned ${UNKNOWN_CODE}"; ((FAIL++)); }

# 6) metrics (optional)
_step "metrics"
METRIC_CODE="$(curl -s --fail --show-error -o /dev/null -w '%{http_code}' "http://${ADDR}/metrics" || true)"
if [[ "${METRIC_CODE}" == "200" ]]; then
  say "✅ /metrics OK"; ((PASS++))
else
  say "⏭️  /metrics not mounted (code=${METRIC_CODE}); skipping"; ((SKIP++))
fi

# --- summary ---
echo "---- summary ----"
echo "PASS=${PASS} FAIL=${FAIL} SKIP=${SKIP}"
if (( FAIL == 0 )); then
  echo "all good ✅"
else
  echo "some checks failed ❌"
  echo "---- server log (tail) ----"
  tail -n 200 "${LOG_FILE}" || true
  exit 1
fi