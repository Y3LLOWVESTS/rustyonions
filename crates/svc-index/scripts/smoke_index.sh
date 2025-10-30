#!/usr/bin/env bash
set -euo pipefail

cargo fmt -p svc-index
cargo clippy -p svc-index --no-deps -- -D warnings
cargo build -p svc-index

INDEX_BIND="${INDEX_BIND:-127.0.0.1:5304}"
RUST_LOG="${RUST_LOG:-info}"

target/debug/svc-index >/tmp/svc-index.log 2>&1 &
PID=$!

trap 'kill $PID >/dev/null 2>&1 || true' EXIT

for i in $(seq 1 60); do
  code=$(curl -s -o /dev/null -w "%{http_code}" "http://$INDEX_BIND/healthz" || true)
  [ "$code" = "200" ] && break
  sleep 0.1
done

CID_ZERO="b3:0000000000000000000000000000000000000000000000000000000000000000"

curl -s -o /dev/null -w "%{http_code}\n" "http://$INDEX_BIND/healthz"
curl -s -o /dev/null -w "%{http_code}\n" "http://$INDEX_BIND/readyz"
curl -s -o /dev/null -w "%{http_code}\n" "http://$INDEX_BIND/version"
curl -s -o /dev/null -w "%{http_code}\n" "http://$INDEX_BIND/resolve/name:does-not-exist"
curl -s -o /dev/null -w "%{http_code}\n" "http://$INDEX_BIND/providers/not-a-cid"
curl -s -o /dev/null -w "%{http_code}\n" "http://$INDEX_BIND/providers/$CID_ZERO"

kill $PID
