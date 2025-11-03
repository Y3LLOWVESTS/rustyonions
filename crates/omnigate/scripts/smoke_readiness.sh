#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT   Drive inflight & error proxies, then verify /readyz degrades (503) and recovers.
# RO:USE    OMNIGATE_DEV_READY=0 cargo run -p omnigate --bin omnigate
#           chmod +x crates/omnigate/scripts/smoke_readiness.sh
#           crates/omnigate/scripts/smoke_readiness.sh
# RO:ENV    BASE=http://127.0.0.1:5305  CONCURRENCY=600  DURATION=12  MS=800  POLL_TIMEOUT=25

BASE="${BASE:-http://127.0.0.1:5305}"
CONCURRENCY="${CONCURRENCY:-600}"   # macOS-safe; bump if your box can take it
DURATION="${DURATION:-12}"          # seconds to sustain load
MS="${MS:-800}"                     # /v1/sleep duration per request
POLL_TIMEOUT="${POLL_TIMEOUT:-25}"  # seconds to wait for a 503

get() { curl -sS -o /dev/null -w "%{http_code}" "$1"; }
say() { printf "%s\n" "$*"; }
ok() { printf "✅ %s\n" "$*"; }
bad() { printf "❌ %s\n" "$*" ; }

say ""
say "▶ Check /readyz before load"
code="$(get "$BASE/readyz")"
if [[ "$code" == "200" ]]; then ok "/readyz: 200"; else bad "/readyz: $code"; exit 1; fi

say ""
say "▶ Create inflight pressure via /v1/sleep?ms=$MS (CONCURRENCY=$CONCURRENCY for ${DURATION}s)"

# Background load generator (bounded duration)
(
  end=$((SECONDS + DURATION))
  while (( SECONDS < end )); do
    # Fire CONCURRENCY requests, then wait for them to complete
    for _ in $(seq 1 "$CONCURRENCY"); do
      curl -sS -o /dev/null "$BASE/v1/sleep?ms=$MS" &
    done
    wait
  done
) & LOAD_PID=$!

say ""
say "▶ Poll /readyz for degrade (expect 503 within a few seconds)"
deadline=$((SECONDS + POLL_TIMEOUT))
tripped=0
while (( SECONDS < deadline )); do
  code="$(get "$BASE/readyz")"
  if [[ "$code" == "503" ]]; then
    ok "Observed degrade: /readyz -> 503"
    tripped=1
    break
  fi
  sleep 0.25
done

# Stop background load if it’s still running
if ps -p "$LOAD_PID" >/dev/null 2>&1; then kill "$LOAD_PID" 2>/dev/null || true; wait "$LOAD_PID" 2>/dev/null || true; fi

if [[ "$tripped" == "0" ]]; then
  bad "Did not observe degrade (still 200)"
  # Show quick gauges to help debug thresholds
  say ""
  say "---- readiness gauges ----"
  curl -sS "$BASE/ops/metrics" | grep -E 'ready_(inflight_current|error_rate_pct|queue_saturated)' || true
  exit 1
fi

say ""
say "▶ Hold window check: /readyz should remain 503 briefly, then recover"
sleep 1
code="$(get "$BASE/readyz")"
say "   now: $code (will go back to 200 after hold_for_secs)"

say ""
say "---- readiness gauges ----"
curl -sS "$BASE/ops/metrics" | grep -E 'ready_(inflight_current|error_rate_pct|queue_saturated)'

ok "All checks executed."
