#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Launch two svc-dht nodes on different admin ports, seed them, prove cross-node lookup.
# RO:RUN
#   chmod +x crates/svc-dht/scripts/two-node-local.sh
#   crates/svc-dht/scripts/two-node-local.sh

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="target/debug/svc-dht"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq required"; exit 1
fi

echo "== build =="
cargo build -p svc-dht >/dev/null

CID="b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

killall -q svc-dht || true
sleep 0.2

echo "== start node A (5301) =="
RON_DHT_ADMIN_ADDR=127.0.0.1:5301 \
RON_DHT_SEEDS="" \
RON_DHT_NODE_URI="local://nodeA" \
"${BIN}" >/tmp/svc-dht-A.log 2>&1 &

echo "== start node B (5302) =="
RON_DHT_ADMIN_ADDR=127.0.0.1:5302 \
RON_DHT_SEEDS="http://127.0.0.1:5301" \
RON_DHT_NODE_URI="local://nodeB" \
"${BIN}" >/tmp/svc-dht-B.log 2>&1 &

ready() { curl -fsS "$1/readyz" >/dev/null 2>&1; }

echo "== wait ready =="
for i in {1..50}; do
  ready http://127.0.0.1:5301 && ready http://127.0.0.1:5302 && break
  sleep 0.1
done

echo "== provide on node A =="
curl -fsS -X POST http://127.0.0.1:5301/provide \
  -H 'content-type: application/json' \
  -d "{\"cid\":\"${CID}\",\"node\":\"local://nodeA\",\"ttl_secs\":60}" | jq .

echo "== find from node B (should discover A) =="
curl -fsS "http://127.0.0.1:5302/find/${CID}" | jq .

echo "== metrics B (grep dht_) =="
curl -fsS http://127.0.0.1:5302/metrics | grep -E '^dht_' || true

echo "== tail logs (A/B hints) =="
echo "-- A --"; tail -n 3 /tmp/svc-dht-A.log || true
echo "-- B --"; tail -n 3 /tmp/svc-dht-B.log || true

echo "== done =="
