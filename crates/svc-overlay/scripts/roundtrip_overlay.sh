#!/usr/bin/env bash
# RO:WHAT
#   Launch svc-overlay, run the OAP client N times, scrape metrics, teardown.
# RO:USAGE
#   chmod +x crates/svc-overlay/scripts/roundtrip_overlay.sh
#   crates/svc-overlay/scripts/roundtrip_overlay.sh 3

set -euo pipefail

RUNS="${1:-3}"
ADMIN_ADDR="${ADMIN_ADDR:-127.0.0.1:9600}"
OVERLAY_ADDR="${OVERLAY_ADDR:-127.0.0.1:9700}"
RUST_LOG="${RUST_LOG:-svc_overlay=info}"

log(){ printf '[roundtrip] %s\n' "$*" >&2; }

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

log "fmt + clippy"
cargo fmt -p svc-overlay
cargo clippy -p svc-overlay --no-deps -- -D warnings

log "building server + oap_client example (only)"
cargo build -p svc-overlay --example oap_client

RUN_LOG="target/svc-overlay.roundtrip.log"
: > "$RUN_LOG"
log "launching svc-overlay (logs → $RUN_LOG)"
RUST_LOG="$RUST_LOG" cargo run -p svc-overlay >"$RUN_LOG" 2>&1 &
SVC_PID=$!

cleanup(){
  set +e
  if kill -0 "$SVC_PID" >/dev/null 2>&1; then
    log "stopping svc-overlay (pid $SVC_PID)"
    kill "$SVC_PID" || true
    sleep 0.3
    kill -9 "$SVC_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

log "waiting for admin @ http://$ADMIN_ADDR/healthz"
for i in {1..60}; do
  curl -sf "http://$ADMIN_ADDR/healthz" >/dev/null && break
  sleep 0.2
  [[ $i -eq 60 ]] && { log "timeout waiting for admin"; tail -n 120 "$RUN_LOG" || true; exit 1; }
done

# Also wait for TCP port to accept (macOS-friendly: use nc -z)
HOST="${OVERLAY_ADDR%:*}"
PORT="${OVERLAY_ADDR##*:}"
log "waiting for overlay TCP ${HOST}:${PORT}"
for i in {1..60}; do
  if nc -z "$HOST" "$PORT" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
  [[ $i -eq 60 ]] && { log "timeout waiting for overlay TCP"; tail -n 120 "$RUN_LOG" || true; exit 1; }
done

show_metrics(){
  curl -sSf "http://$ADMIN_ADDR/metrics" \
    | grep -E 'overlay_build_info|overlay_sessions_active|overlay_(accept_latency_seconds|frames_in_total|frames_out_total|bytes_in_total|bytes_out_total)' \
    || true
}

log "baseline metrics"
show_metrics

for n in $(seq 1 "$RUNS"); do
  log "client run $n/$RUNS"
  OVERLAY_ADDR="$OVERLAY_ADDR" cargo run -q -p svc-overlay --example oap_client || {
    log "client failed on run $n"
    tail -n 120 "$RUN_LOG" || true
    exit 1
  }
done

log "post-runs metrics"
show_metrics

log "recent server logs:"
tail -n 120 "$RUN_LOG" || true

log "done"
