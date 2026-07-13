#!/usr/bin/env bash
set -euo pipefail
CID="${1:-b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef}"
NODE="${2:-crab://node/00000000000000000000000000000000000000000000000000000000000000a1}"
TTL="${3:-60}"
ADDR="${4:-127.0.0.1:5301}"

curl -s -X POST "http://${ADDR}/dht/provide" \
  -H "content-type: application/json" \
  -d "{\"cid\":\"${CID}\",\"node\":\"${NODE}\",\"ttl_secs\":${TTL}}" | jq

curl -s "http://${ADDR}/dht/find_providers/${CID}" | jq
curl -s "http://${ADDR}/metrics" | grep -E "dht_lookup_latency_seconds_count|dht_lookup_hops|dht_lookups_total" || true
