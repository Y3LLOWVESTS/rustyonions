#!/usr/bin/env bash
# RO:WHAT  — End-to-end smoke for ron-transport loopback (robust macOS/Linux).
# RO:NOTE  — Uses Python socket send first (non-blocking), falls back to nc with safe flags.

set -euo pipefail

HOST_DEFAULT="127.0.0.1"
HOST="${HOST:-$HOST_DEFAULT}"
PORT="${PORT:-}"
LOG_FILE="$(mktemp -t ron_transport_smoke.XXXXXX.log)"
SERVER_PID=""
CLEANUP_DONE=0

cleanup() {
  if [[ $CLEANUP_DONE -eq 1 ]]; then return; fi
  CLEANUP_DONE=1
  if [[ -n "${SERVER_PID}" ]]; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
  rm -f "$LOG_FILE" || true
}
trap cleanup EXIT INT TERM

info()  { printf "[INFO] %s\n" "$*"; }
ok()    { printf "[ OK ] %s\n" "$*"; }
warn()  { printf "[WARN] %s\n" "$*"; }
fail()  { printf "[FAIL] %s\n" "$*"; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { fail "missing '$1' in PATH"; exit 1; }
}

spawn_server_if_needed() {
  if [[ -n "${PORT}" ]]; then
    info "Using provided PORT=${PORT}, HOST=${HOST}; will not spawn server."
    return
  fi

  info "Starting bench_echo to auto-discover port (HOST=${HOST})…"
  RUST_LOG=info cargo run -q -p ron-transport --example bench_echo >"$LOG_FILE" 2>&1 &
  SERVER_PID=$!

  # Wait until it prints the listening line (timeout ~5s).
  for _ in {1..50}; do
    if grep -q "ron-transport listening on" "$LOG_FILE"; then
      break
    fi
    sleep 0.1
  done
  if ! grep -q "ron-transport listening on" "$LOG_FILE"; then
    warn "Could not detect listener line; recent log:"
    tail -n +1 "$LOG_FILE" || true
    fail "Server failed to start or log was not captured."
    exit 1
  fi

  local line
  line="$(grep "ron-transport listening on" "$LOG_FILE" | tail -n1)"
  PORT="$(awk -F: '{print $NF}' <<<"$line" | tr -d '[:space:]')"
  HOST="$(sed -E 's/.* on ([0-9\.]+):[0-9]+/\1/' <<<"$line")"
  ok "Server is up: ${HOST}:${PORT} (pid ${SERVER_PID})"
}

nc_support_flags() {
  # Detect safe close flag for this nc variant.
  local help; help="$( (nc -h 2>&1 || true) )"
  if grep -q -- " -N" <<<"$help"; then
    echo "-N"           # OpenBSD/macOS: close on stdin EOF
  elif grep -q -- " -q " <<<"$help"; then
    echo "-q 1"         # GNU netcat: quit 1s after EOF on stdin
  else
    echo ""             # Unknown; we’ll guard with timeout anyway
  fi
}

run_probes() {
  # 1) Quick TCP connect check with nc -vz (non-blocking)
  info "Probing with nc (TCP connect)…"
  if command -v nc >/dev/null 2>&1; then
    if nc -vz -w 2 "${HOST}" "${PORT}" >/dev/null 2>&1; then
      ok "nc connect succeeded"
    else
      warn "nc connect failed (continuing)"
    fi
  else
    warn "nc not found; skipping"
  fi

  # 2) Send bytes via Python socket (preferred, never hangs)
  info "Python one-liner send…"
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<PY || warn "python send failed (continuing)"
import socket
s=socket.create_connection(("${HOST}", int("${PORT}")), 2)
s.sendall(b"hello\\n")
s.close()
PY
    ok "python send completed"
  else
    warn "python3 not found; skipping python send"
  fi

  # 3) Optional: nc one-shot send with safe close (guarded)
  if command -v nc >/dev/null 2>&1; then
    local CLOSE_FLAGS; CLOSE_FLAGS="$(nc_support_flags)"
    info "Sending bytes via nc (one-shot, flags: ${CLOSE_FLAGS:-none})…"
    # Run nc in the background with a hard kill after 3s as a final guard.
    ( printf 'hello ron-transport\n' | nc ${CLOSE_FLAGS} -w 2 "${HOST}" "${PORT}" ) >/dev/null 2>&1 & 
    local nc_pid=$!
    # Hard timeout guard:
    ( sleep 3; kill "$nc_pid" >/dev/null 2>&1 || true ) &
    wait "$nc_pid" >/dev/null 2>&1 || true
    ok "sent bytes with nc (no response expected)"
  fi

  # 4) curl proof (expect timeout; add connect-timeout)
  info "curl smoke (expect timeout, proves non-HTTP raw TCP)…"
  if command -v curl >/dev/null 2>&1; then
    if echo -n 'hello' | curl --no-progress-meter --data-binary @- \
         --connect-timeout 1 --max-time 2 "http://${HOST}:${PORT}/" >/dev/null; then
      warn "curl returned success (unexpected for raw TCP), continuing"
    else
      ok "curl timed out as expected (raw TCP, no HTTP)"
    fi
  else
    warn "curl not found; skipping"
  fi

  info "All probes done."
}

main() {
  require_cmd cargo
  spawn_server_if_needed
  run_probes
}

main
