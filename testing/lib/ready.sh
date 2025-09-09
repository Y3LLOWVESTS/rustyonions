#!/usr/bin/env bash
# FILE: testing/lib/ready.sh
# Readiness polling helpers to avoid arbitrary sleeps in tests/scripts.
# Bash 3.2 compatible (macOS default). No mapfile/assoc arrays/etc.

set -euo pipefail

_now() { date +%s; }

# _poll_until "<shell command>" [timeout_sec=10] [interval_sec=0.10]
# Repeatedly runs the command until it succeeds (exit 0) or times out.
_poll_until() {
  local cmd="${1:-}"; shift || true
  local timeout="${1:-10}"; shift || true
  local interval="${1:-0.10}"

  if [ -z "${cmd}" ]; then
    echo "[ready.sh] _poll_until: missing command" >&2
    return 2
  fi

  local start end
  start="$(_now)"
  end=$(( start + timeout ))

  while [ "$(_now)" -lt "${end}" ]; do
    if bash -c "${cmd}" >/dev/null 2>&1; then
      return 0
    fi
    sleep "${interval}"  # allow-sleep (bounded, short, poll loop)
  done
  return 1
}

# Wait for a Unix domain socket path to exist.
#   wait_udsocket "/tmp/ron/svc-overlay.sock" [timeout=10]
wait_udsocket() {
  local sock="${1:?need path to socket}"
  local timeout="${2:-10}"
  _poll_until "[[ -S \"${sock}\" ]]" "${timeout}" "0.05"
}

# Wait for a TCP port to open (host:port).
#   wait_tcp 127.0.0.1 9080 [timeout=10]
wait_tcp() {
  local host="${1:?need host}"
  local port="${2:?need port}"
  local timeout="${3:-10}"
  _poll_until "nc -z \"${host}\" \"${port}\"" "${timeout}" "0.10"
}

# Wait for an HTTP 2xx/3xx response.
#   wait_http_ok "http://127.0.0.1:9080/healthz" [timeout=15]
wait_http_ok() {
  local url="${1:?need url}"
  local timeout="${2:-15}"
  _poll_until "curl -fsS -o /dev/null \"${url}\"" "${timeout}" "0.25"
}

# Wait for an exact HTTP status code (e.g., 200, 404, 503).
#   wait_http_status "http://..." 200 [timeout=15]
wait_http_status() {
  local url="${1:?need url}"
  local want="${2:?need status code}"
  local timeout="${3:-15}"
  _poll_until "[[ \$(curl -s -o /dev/null -w \"%{http_code}\" \"${url}\") -eq ${want} ]]" "${timeout}" "0.25"
}

# Wait for file presence.
#   wait_file "/path/to/file" [timeout=10]
wait_file() {
  local path="${1:?need file path}"
  local timeout="${2:-10}"
  _poll_until "[[ -e \"${path}\" ]]" "${timeout}" "0.05"
}

# Wait for a log pattern to appear in a file (simple poll via ripgrep).
#   wait_log_pattern "/path/to/log" "pattern text" [timeout=20]
wait_log_pattern() {
  local file="${1:?need log file}"
  local pattern="${2:?need pattern}"
  local timeout="${3:-20}"
  _poll_until "rg -n \"${pattern}\" \"${file}\"" "${timeout}" "0.25"
}

# Wait for a PID to exit (process gone).
#   wait_pid_gone 12345 [timeout=20]
wait_pid_gone() {
  local pid="${1:?need pid}"
  local timeout="${2:-20}"
  _poll_until "! kill -0 ${pid}" "${timeout}" "0.10"
}
