#!/usr/bin/env bash
set -euo pipefail

echo "fmt+clippy+build…"
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway

pkill -f svc-gateway || true
RUST_LOG=info,target=svc_gateway=debug cargo run -p svc-gateway &

sleep 0.5
echo "healthz:";  curl -si http://127.0.0.1:5304/healthz | head -n 1
echo "readyz:";   curl -si http://127.0.0.1:5304/readyz  | head -n 1
echo "metrics:";  curl -s  http://127.0.0.1:5304/metrics | head -n 10
