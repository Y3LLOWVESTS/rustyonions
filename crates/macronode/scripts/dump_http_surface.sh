#!/usr/bin/env bash
# RO:WHAT — Dump macronode admin HTTP surface.
# RO:WHY  — Quick visibility into /version, /healthz, /readyz, /metrics, /api/v1/status.
# RO:INVARIANTS —
#   - Never fails hard if optional tools (jq) are missing.
#   - Uses RON_HTTP_ADDR if set, otherwise 127.0.0.1:8080.

set -euo pipefail

ADDR="${RON_HTTP_ADDR:-127.0.0.1:8080}"
BASE="http://${ADDR}"

say() { printf '[macronode] %s\n' "$*"; }

say "Dumping admin HTTP surface from ${BASE}"

has_jq=0
if command -v jq >/dev/null 2>&1; then
  has_jq=1
fi

echo
say "GET /version"
if [[ "${has_jq}" == "1" ]]; then
  curl -sS "${BASE}/version" | jq . || true
else
  curl -sS "${BASE}/version" || true
fi
echo

say "GET /healthz"
curl -sS "${BASE}/healthz" || true
echo

say "GET /readyz"
curl -sS "${BASE}/readyz" || true
echo

say "HEAD /metrics (first headers)"
curl -sSI "${BASE}/metrics" | sed -n '1,15p' || true
echo

say "GET /api/v1/status"
if [[ "${has_jq}" == "1" ]]; then
  curl -sS "${BASE}/api/v1/status" | jq . || true
else
  curl -sS "${BASE}/api/v1/status" || true
fi
echo

say "Done."
