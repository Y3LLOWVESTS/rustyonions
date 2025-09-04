#!/usr/bin/env bash
set -euo pipefail

PORT="${OAP_PORT:-9444}"
LOG="/tmp/oap_server.log"

# kill any existing server on the port to avoid "address already in use"
PIDS="$(lsof -ti tcp:${PORT} -sTCP:LISTEN || true)"
if [ -n "${PIDS}" ]; then
  echo "killing old listeners on :${PORT} -> ${PIDS}"
  kill ${PIDS} || true
  # give the OS a moment to release the socket
  sleep 0.3
fi

# build (reuse cache)
cargo build -p gateway --examples

# start server in the background
: > "${LOG}"
cargo run -p gateway --example oap_server_demo > "${LOG}" 2>&1 &
SERVER_PID=$!

# wait for the server to bind (up to ~5s)
tries=0
until lsof -ti tcp:${PORT} -sTCP:LISTEN >/dev/null 2>&1; do
  tries=$((tries+1))
  if [ ${tries} -ge 50 ]; then
    echo "server failed to bind on :${PORT}"
    echo "---- server log ----"
    cat "${LOG}" || true
    echo "--------------------"
    kill "${SERVER_PID}" 2>/dev/null || true
    exit 1
  fi
  sleep 0.1
done

# run client (prints to stdout)
cargo run -p gateway --example oap_client_demo

# show server log tail for quick sanity
echo
echo "---- server log ----"
tail -n 50 "${LOG}" || true
echo "--------------------"

# cleanup
kill "${SERVER_PID}" 2>/dev/null || true
wait "${SERVER_PID}" 2>/dev/null || true
