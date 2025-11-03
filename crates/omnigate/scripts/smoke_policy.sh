#!/usr/bin/env bash
set -euo pipefail

API_ADDR="${API_ADDR:-127.0.0.1:5305}"
ADMIN_ADDR="${ADMIN_ADDR:-127.0.0.1:9605}"
MAX_BODY="${OMNIGATE_MAX_BODY:-10485760}"

# We scrape the default Prom registry exposed by the API on /ops/metrics
METRICS_URL="${METRICS_URL:-http://${API_ADDR}/ops/metrics}"

say() { printf "\n\033[1m▶ %s\033[0m\n" "$*"; }
status() { curl -s -o /dev/null -w "%{http_code}" "$@"; }

metric_sum() {
  local metric="$1"
  curl -s "$METRICS_URL" \
  | awk -v re="^"$(printf "%s" "$metric" | sed 's/[].[^$*+?{}()|/\\]/\\&/g') \
      '$0 ~ re { v=$NF; if (v ~ /^[0-9]+([.][0-9]+)?$/) s+=v+0 } END { printf("%.0f\n", (s==""?0:s)) }'
}

metric_show() {
  local metric="$1"
  echo "---- ${metric} lines ----"
  curl -s "$METRICS_URL" \
  | awk -v re="^"$(printf "%s" "$metric" | sed 's/[].[^$*+?{}()|/\\]/\\&/g') '$0 ~ re { print }'
  echo "-------------------------"
}

say "Check /readyz"
[ "$(status "http://${ADMIN_ADDR}/readyz")" = "200" ] && echo "✅ /readyz: 200" || { echo "❌ /readyz not 200"; exit 1; }

say "GET /v1/ping (expect 200)"
[ "$(status -X GET "http://${API_ADDR}/v1/ping")" = "200" ] && echo "✅ GET /v1/ping: 200" || { echo "❌ ping"; exit 1; }

POLICY_METRIC="policy_middleware_shortcircuits_total"
BODY_METRIC="body_reject_total"
DECOMP_METRIC="decompress_reject_total"

P_BEFORE="$(metric_sum "$POLICY_METRIC")"
B_BEFORE="$(metric_sum "$BODY_METRIC")"
D_BEFORE="$(metric_sum "$DECOMP_METRIC")"

say "PUT /v1/ping with Content-Length: 0 (expect policy 403)"
[ "$(status -X PUT -H "Content-Length: 0" "http://${API_ADDR}/v1/ping")" = "403" ] && echo "✅ PUT /v1/ping (CL:0): 403" || { echo "❌ policy 403"; exit 1; }

say "PUT /v1/ping with stacked encodings (expect 415)"
[ "$(status -X PUT -H "Content-Encoding: gzip, br" -H "Content-Length: 0" "http://${API_ADDR}/v1/ping")" = "415" ] && echo "✅ PUT /v1/ping (stacked encodings): 415" || { echo "❌ 415"; exit 1; }

say "PUT /v1/ping oversize body (expect 413)"
TMP_BIG="$(mktemp -t omnigate-big.XXXXXX)"
if head -c "$((MAX_BODY + 1))" /dev/zero > "$TMP_BIG" 2>/dev/null; then :; else
  dd if=/dev/zero bs=1 count=$((MAX_BODY + 1)) of="$TMP_BIG" status=none
fi
[ "$(status -X PUT --data-binary @"$TMP_BIG" "http://${API_ADDR}/v1/ping")" = "413" ] && echo "✅ PUT /v1/ping (oversize): 413" || { echo "❌ 413"; rm -f "$TMP_BIG"; exit 1; }
rm -f "$TMP_BIG"

P_AFTER="$(metric_sum "$POLICY_METRIC")"
B_AFTER="$(metric_sum "$BODY_METRIC")"
D_AFTER="$(metric_sum "$DECOMP_METRIC")"

say "Metrics deltas"
printf "  %s: %s -> %s  (Δ=%s)\n" "$POLICY_METRIC" "$P_BEFORE" "$P_AFTER" "$((P_AFTER - P_BEFORE))"
printf "  %s: %s -> %s  (Δ=%s)\n" "$BODY_METRIC"    "$B_BEFORE" "$B_AFTER" "$((B_AFTER - B_BEFORE))"
printf "  %s: %s -> %s  (Δ=%s)\n" "$DECOMP_METRIC"  "$D_BEFORE" "$D_AFTER" "$((D_AFTER - D_BEFORE))"

metric_show "$POLICY_METRIC"
metric_show "$BODY_METRIC"
metric_show "$DECOMP_METRIC"

say "All checks passed."
