#!/usr/bin/env bash
set -euo pipefail

# One-shot test for decode guard:
# - gzip body -> 413 encoded_body_unsupported (or decoded_cap if your cap is tiny)
# - stacked encodings -> 415 stacked_encoding
# - declared len > cap -> 413 decoded_cap
#
# Starts the gateway with DEV routes + metrics and DECODE_ABS_CAP_BYTES=8,
# runs probes, prints relevant metrics, and stops the process.

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
PID_FILE="$ROOT/target/gateway.pid"

export SVC_GATEWAY_DEV_ROUTES=1
export SVC_GATEWAY_DEV_METRICS=1
export SVC_GATEWAY_DECODE_ABS_CAP_BYTES=8

"${ROOT}/crates/svc-gateway/scripts/run_gateway.sh"

PID="$(cat "$PID_FILE")"
sleep 0.4

echo
echo "== 1) encoded body (gzip) => expect 415 encoded_body_unsupported (or 413 if cap trips)"
printf hi | gzip | curl -s -i -X POST --data-binary @- \
  -H 'Content-Encoding: gzip' \
  http://127.0.0.1:5304/dev/echo | sed -n '1,20p'

echo
echo "== 2) stacked encodings => expect 415 stacked_encoding"
curl -s -i -X POST --data-binary 'x' \
  -H 'Content-Encoding: gzip, br' \
  http://127.0.0.1:5304/dev/echo | sed -n '1,20p'

echo
echo "== 3) declared length > cap (16 > 8) => expect 413 decoded_cap"
curl -s -i -X POST --data-binary @<(head -c 16 </dev/zero | tr '\0' A) \
  http://127.0.0.1:5304/dev/echo | sed -n '1,20p'

echo
echo "== metrics snapshot (decode-related rejects) =="
curl -s http://127.0.0.1:5304/metrics | egrep 'gateway_rejections_total{reason="(decode_cap|stacked_encoding|encoded_body)"}' || true

"${ROOT}/crates/svc-gateway/scripts/stop_gateway.sh"
