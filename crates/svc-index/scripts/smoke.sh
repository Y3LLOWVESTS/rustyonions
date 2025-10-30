#!/usr/bin/env bash
# RO:WHAT — Smoke test for svc-index: boot and hit endpoints.
# RO:WHY  — CI/local quick check without magic sleeps.

set -euo pipefail
BIND="${BIND:-127.0.0.1:5304}"
BIN="${BIN:-cargo run -p svc-index}"
LOG="/tmp/svc-index.log"

$BIN > "$LOG" 2>&1 &
PID=$!

deadline=$((SECONDS+10))
until curl -fsS "http://$BIND/readyz" >/dev/null; do
  [[ $SECONDS -gt $deadline ]] && { echo "readyz timeout"; kill $PID || true; exit 1; }
  sleep 0.2
done

curl -fsS "http://$BIND/healthz" | grep -q ok
curl -fsS "http://$BIND/version" | grep -q svc-index
curl -fsS "http://$BIND/resolve/name:hello" || true
curl -fsS "http://$BIND/providers/b3:0000000000000000000000000000000000000000000000000000000000000000" || true

kill $PID || true
echo "✅ svc-index smoke passed"
