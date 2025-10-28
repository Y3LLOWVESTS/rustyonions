#!/usr/bin/env bash
set -euo pipefail
# run-local.sh — local smoke: readyz → provide → find → metrics

ADDR="${1:-127.0.0.1:5301}"
CID="${2:-b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef}"
NODE="${3:-local://nodeA}"
TTL="${4:-60}"

echo ">>> Waiting for readyz at http://${ADDR}/readyz ..."
for i in $(seq 1 100); do
  code=$(curl -s -o /dev/null -w "%{http_code}" "http://${ADDR}/readyz" || true)
  [ "$code" = "200" ] && echo "ready" && break
  sleep 0.1
done

echo ">>> Version:"
curl -s "http://${ADDR}/version" | jq -r '.'

echo ">>> Provide:"
curl -s -X POST "http://${ADDR}/dht/provide" \
  -H "content-type: application/json" \
  -d "{\"cid\":\"${CID}\",\"node\":\"${NODE}\",\"ttl_secs\":${TTL}}" | jq -r '.'

echo ">>> Find:"
curl -s "http://${ADDR}/dht/find_providers/${CID}" | jq -r '.'

echo ">>> Metrics (grep dht_*):"
curl -s "http://${ADDR}/metrics" | grep -E "dht_lookup_|dht_lookups_total|dht_provides_total" || true

echo "done"
