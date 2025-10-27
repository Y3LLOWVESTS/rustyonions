#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT  — Lightweight soak/chaos for svc-overlay echo path.
# RO:USAGE — ./crates/svc-overlay/scripts/soak_overlay.sh [seconds] [concurrency]
#            Defaults: 30s, 16 clients
# RO:NOTE  — Uses the existing oap_client example via the roundtrip driver.

DURATION="${1:-30}"
CONCURRENCY="${2:-16}"

echo "[soak] duration=${DURATION}s concurrency=${CONCURRENCY}"
echo "[soak] RON_OVERLAY_TX_WATERMARK=${RON_OVERLAY_TX_WATERMARK:-<default>} RON_OVERLAY_HANDSHAKE_MS=${RON_OVERLAY_HANDSHAKE_MS:-<default>}"

# Ensure server is up
if ! curl -fsS http://127.0.0.1:9600/healthz >/dev/null ; then
  echo "[soak] server not up on 127.0.0.1:9600 — start it first (e.g., cargo run -p svc-overlay)"
  exit 1
fi

# Warm baseline metrics
echo "[soak] baseline overlay_* sample:"
curl -fsS http://127.0.0.1:9600/metrics | rg '^overlay_' | head -n 20 || true

echo "[soak] kicking off clients…"
end=$(( SECONDS + DURATION ))
i=0
fails=0

run_client() {
  # Same client path the roundtrip script uses
  target/debug/examples/oap_client 127.0.0.1:9700 >/dev/null 2>&1 || return 1
  return 0
}

# Fire-and-forget workers
while [ $SECONDS -lt $end ]; do
  for _ in $(seq 1 "$CONCURRENCY"); do
    run_client & pid=$!
    pids+=("$pid")
  done
  # Reap in batches
  for p in "${pids[@]:-}"; do
    if ! wait "$p"; then
      fails=$((fails+1))
    fi
  done
  unset pids
  i=$((i+CONCURRENCY))
done

echo "[soak] completed launches: ${i}, failures: ${fails}"

echo "[soak] post-run overlay_* sample:"
curl -fsS http://127.0.0.1:9600/metrics | rg '^overlay_' | head -n 50 || true

echo "[soak] readiness snapshot:"
curl -fsS http://127.0.0.1:9600/readyz | jq .
