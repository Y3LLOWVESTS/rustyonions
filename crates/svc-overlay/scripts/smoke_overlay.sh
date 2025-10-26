#!/usr/bin/env bash
# RO:WHAT
#   One-button smoke test for svc-overlay:
#     - fmt + clippy
#     - launch svc-overlay (background)
#     - wait for admin /healthz
#     - open a few test sockets (nc)
#     - scrape metrics (gauge + histogram)
#     - clean shutdown
#
# RO:USAGE
#   chmod +x crates/svc-overlay/scripts/smoke_overlay.sh
#   crates/svc-overlay/scripts/smoke_overlay.sh
#
# RO:CONFIG
#   ADMIN_ADDR:  admin HTTP bind (default 127.0.0.1:9600)
#   OVERLAY_ADDR: overlay TCP bind (default 127.0.0.1:9700)
#   RUST_LOG: set logging for the run (default svc_overlay=info)

set -euo pipefail

ADMIN_ADDR="${ADMIN_ADDR:-127.0.0.1:9600}"
OVERLAY_ADDR="${OVERLAY_ADDR:-127.0.0.1:9700}"
RUST_LOG="${RUST_LOG:-svc_overlay=info}"

log() { printf '[smoke] %s\n' "$*" >&2; }

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

log "fmt + clippy"
cargo fmt -p svc-overlay
cargo clippy -p svc-overlay --no-deps -- -D warnings

RUN_LOG="target/svc-overlay.smoke.log"
: > "$RUN_LOG"

log "launching svc-overlay (logs → $RUN_LOG)"
RUST_LOG="$RUST_LOG" cargo run -p svc-overlay >"$RUN_LOG" 2>&1 &
SVC_PID=$!

cleanup() {
  set +e
  if kill -0 "$SVC_PID" >/dev/null 2>&1; then
    log "stopping svc-overlay (pid $SVC_PID)"
    kill "$SVC_PID" || true
    sleep 0.5
    kill -9 "$SVC_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# Wait for /healthz
log "waiting for admin to be up at http://$ADMIN_ADDR/healthz"
for i in {1..60}; do
  if curl -sf "http://$ADMIN_ADDR/healthz" >/dev/null; then
    log "admin is up"
    break
  fi
  sleep 0.25
  if ! kill -0 "$SVC_PID" >/dev/null 2>&1; then
    log "svc-overlay terminated unexpectedly"
    tail -n 200 "$RUN_LOG" >&2 || true
    exit 1
  fi
  if [[ $i -eq 60 ]]; then
    log "timeout waiting for admin"
    tail -n 200 "$RUN_LOG" >&2 || true
    exit 1
  fi
done

# Helper to show key metrics
show_metrics() {
  curl -sSf "http://$ADMIN_ADDR/metrics" \
  | grep -E 'overlay_build_info|overlay_sessions_active|overlay_accept_latency_seconds_(count|sum)' \
  || true
}

log "initial metrics snapshot"
show_metrics

# Function to open a short-lived socket (which will handshake-timeout in ~3s)
short_socket() {
  # Use a subshell so we can background it cleanly
  (
    # macOS/BSD nc: -w <secs> is a write timeout; best effort: just sleep then exit
    # We want the server to see an accept and then a timeout/close
    exec nc "$(cut -d: -f1 <<<"$OVERLAY_ADDR")" "$(cut -d: -f2 <<<"$OVERLAY_ADDR")"
  ) &
  NC_PID=$!
  sleep 3.2
  kill -INT "$NC_PID" 2>/dev/null || true
  wait "$NC_PID" 2>/dev/null || true
}

# Function to open a long socket so we can watch the gauge at 1
long_socket_open() {
  (
    exec nc "$(cut -d: -f1 <<<"$OVERLAY_ADDR")" "$(cut -d: -f2 <<<"$OVERLAY_ADDR")"
  ) &
  echo $!
}

log "opening one long-lived socket to demonstrate overlay_sessions_active=1"
LONG_PID="$(long_socket_open)"
sleep 0.2
show_metrics

log "closing long-lived socket"
kill -INT "$LONG_PID" 2>/dev/null || true
wait "$LONG_PID" 2>/dev/null || true
sleep 0.2
show_metrics

log "opening two short-lived sockets to tick the latency histogram"
short_socket
show_metrics
short_socket
show_metrics

log "final metrics snapshot"
show_metrics

log "tail of runtime logs:"
tail -n 40 "$RUN_LOG" || true

log "done"
