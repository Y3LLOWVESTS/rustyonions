#!/usr/bin/env bash
# End-to-end: prebuild tests -> start services+gateway (gwsmoke) -> run http_read_path test fast.
# Readiness = (log OR tcp OR any http). OBJ_ADDR is parsed from gwsmoke's "post addr  :" line
# and normalized to b3:<hex>[.suffix]. We keep the stack alive via GWSMOKE_HOLD_SEC.

set -euo pipefail

ROOT="${ROOT:-.}"
OUT_DIR="${OUT_DIR:-.onions}"
HTTP_WAIT="${HTTP_WAIT:-25}"
RUST_LOG="${RUST_LOG:-info}"
ACCEPTS_HDR="${ACCEPTS:-br, zstd, gzip, identity}"
HOLD_SEC="${HOLD_SEC:-8}"   # exported as GWSMOKE_HOLD_SEC for gwsmoke

# ---- pick a free port unless BIND provided ----
pick_port() {
  for p in $(seq 9080 9099); do
    if ! nc -z 127.0.0.1 "$p" >/dev/null 2>&1; then echo "$p"; return 0; fi
  done
  return 1
}
PORT="${PORT:-$(pick_port || true)}"
[[ -n "${BIND:-}" ]] || BIND="127.0.0.1:${PORT:-9080}"
GATEWAY_URL="http://${BIND}"

TMP_DIR="$(mktemp -d -t ron_readpath.XXXXXX)"
SMOKE_LOG="${TMP_DIR}/gwsmoke.log"

cleanup() {
  if [[ -n "${SMOKE_PID:-}" ]] && ps -p "${SMOKE_PID}" >/dev/null 2>&1; then
    kill "${SMOKE_PID}" >/dev/null 2>&1 || true
    sleep 0.2 || true
    kill -9 "${SMOKE_PID}" >/dev/null 2>&1 || true
  fi
  echo "[i] Logs kept at: ${TMP_DIR}"
}
trap cleanup EXIT

echo "[*] Building gwsmoke, gateway…"
cargo build -p gwsmoke -p gateway >/dev/null

echo "[*] Prebuilding the test binary (no-run)…"
cargo test -p gateway --test http_read_path --no-run >/dev/null

# Locate the compiled test executable
TEST_BIN="$(find target/debug/deps -maxdepth 1 -type f -name 'http_read_path-*' -perm -111 | sort -r | head -n1 || true)"
if [[ -z "${TEST_BIN}" ]]; then
  echo "[!] Could not locate the test binary under target/debug/deps"; exit 1
fi

# Prefer running gwsmoke binary directly (avoids cargo-run buffering)
GWSMOKE_BIN="${GWSMOKE_BIN:-target/debug/gwsmoke}"
if [[ ! -x "${GWSMOKE_BIN}" ]]; then
  echo "[!] gwsmoke binary not found at ${GWSMOKE_BIN}"; exit 1
fi

echo "[*] Starting gwsmoke (services + gateway) on ${BIND}…"
GWSMOKE_HOLD_SEC="${HOLD_SEC}" RUST_LOG="${RUST_LOG}" \
"${GWSMOKE_BIN}" \
  --build \
  --root "${ROOT}" \
  --out-dir "${OUT_DIR}" \
  --bind "${BIND}" \
  --http-wait-sec "${HTTP_WAIT}" \
  --stream \
  --rust-log "${RUST_LOG}" \
  >"${SMOKE_LOG}" 2>&1 &
SMOKE_PID=$!

echo "[*] Waiting for readiness (log|tcp|http) and extracting OBJ_ADDR…"
OBJ_ADDR=""

ready=0
for _ in {1..400}; do
  # Parse "post addr  : <value>" robustly (strip label, trim, keep address intact)
  if [[ -z "${OBJ_ADDR}" ]] && [[ -s "${SMOKE_LOG}" ]]; then
    cand="$(sed -n 's/^post addr[[:space:]]*:[[:space:]]*//p' "${SMOKE_LOG}" | tail -n1 | tr -d '\r' | xargs)"
    if [[ -n "${cand}" ]]; then
      OBJ_ADDR="${cand}"
      # Normalize to b3:<hex>[.suffix] if missing
      if [[ "${OBJ_ADDR}" != b3:* ]]; then
        OBJ_ADDR="b3:${OBJ_ADDR}"
      fi
    fi
  fi

  # (a) log-based readiness
  if [[ -s "${SMOKE_LOG}" ]] && grep -q "gateway listening" "${SMOKE_LOG}"; then
    ready=1; break
  fi
  # (b) TCP open
  if nc -z "${BIND%:*}" "${BIND#*:}" >/dev/null 2>&1; then
    ready=1; break
  fi
  # (c) any HTTP status
  code="$(curl -s -o /dev/null -w '%{http_code}' "${GATEWAY_URL}/__probe__" || true)"
  if [[ -n "${code}" && "${code}" != "000" ]]; then
    ready=1; break
  fi

  # Process died early?
  if ! ps -p "${SMOKE_PID}" >/dev/null 2>&1; then
    echo "[!] gwsmoke ended early; last 200 lines:"; tail -n 200 "${SMOKE_LOG}" || true; exit 1
  fi
  sleep 0.05
done

if [[ ${ready} -ne 1 ]]; then
  echo "[!] Timed out waiting for readiness; last 200 lines of log:"; tail -n 200 "${SMOKE_LOG}" || true
  exit 1
fi

if [[ -z "${OBJ_ADDR}" ]]; then
  echo "[!] Could not extract OBJ_ADDR from gwsmoke logs (no 'post addr' yet). Last 120 lines:"
  tail -n 120 "${SMOKE_LOG}" || true
  exit 1
fi

echo "[*] Using OBJ_ADDR=${OBJ_ADDR}"
echo "[*] Running read-path tests (single thread)…"

export GATEWAY_URL="${GATEWAY_URL}"
export OBJ_ADDR="${OBJ_ADDR}"
export ACCEPTS="${ACCEPTS_HDR}"
export REL="${REL:-Manifest.toml}"

set +e
"${TEST_BIN}" --nocapture --test-threads=1
RC=$?
set -e

if [[ ${RC} -eq 0 ]]; then
  echo "[✓] Read-path tests passed."
else
  echo "[x] Read-path tests failed (rc=${RC}). See logs:"
  echo "    gwsmoke log: ${SMOKE_LOG}"
fi

exit ${RC}
