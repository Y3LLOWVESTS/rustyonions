#!/usr/bin/env bash
set -euo pipefail

# Simple end-to-end smoke: build, run demos, drive traffic, print metric deltas.

# Colors (optional)
ok() { printf "\033[32m%s\033[0m\n" "$*"; }
info() { printf "\033[36m%s\033[0m\n" "$*"; }

# Build
info "Building ron-kernel…"
cargo build -p ron-kernel

# Start node_index (9097 admin, 8086 app)
info "Starting node_index…"
RUST_LOG=info ./target/debug/node_index > /tmp/node_index.log 2>&1 &
PID_INDEX=$!
sleep 0.6

# Drive requests
info "Driving 5 PUTs and 5 GETs against node_index…"
for i in {1..5}; do
  curl -s -X POST http://127.0.0.1:8086/put -H 'content-type: application/json' -d "{\"addr\":\"A$i\",\"dir\":\"B$i\"}" > /dev/null
done
for i in {1..5}; do
  curl -s http://127.0.0.1:8086/resolve/A$i > /dev/null
done

# Print metrics summary
ok "node_index metrics:"
curl -s http://127.0.0.1:9097/metrics | grep -E 'request_latency_seconds(_count|_sum)' || true

# Start node_overlay (9090 admin, 8071 app)
info "Starting node_overlay…"
RUST_LOG=info ./target/debug/node_overlay > /tmp/node_overlay.log 2>&1 &
PID_OVERLAY=$!
sleep 0.6

# Drive overlay requests
info "Driving 20 echos against node_overlay…"
for i in {1..20}; do
  curl -s -X POST http://127.0.0.1:8071/echo -H 'content-type: application/json' -d '{"payload":"ping"}' > /dev/null
done

# Print metrics summary
ok "node_overlay metrics:"
curl -s http://127.0.0.1:9090/metrics | grep -E 'request_latency_seconds(_count|_sum)' || true

# Health/ready checks
ok "Health/Ready checks:"
echo -n "index /healthz: "; curl -s http://127.0.0.1:9097/healthz; echo
echo -n "index /readyz : "; curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9097/readyz
echo -n "overlay /healthz: "; curl -s http://127.0.0.1:9090/healthz; echo
echo -n "overlay /readyz : "; curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9090/readyz

# Cleanup
info "Stopping demos…"
kill "$PID_INDEX" "$PID_OVERLAY" 2>/dev/null || true
wait "$PID_INDEX" "$PID_OVERLAY" 2>/dev/null || true
ok "Done."
