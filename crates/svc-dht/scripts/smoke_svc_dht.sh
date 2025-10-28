#!/usr/bin/env bash
set -euo pipefail

echo "== svc-dht smoke start =="
echo "== format + clippy =="
cargo fmt -p svc-dht
cargo clippy -p svc-dht --no-deps -- -D warnings

echo "== unit/integration tests =="
cargo test -p svc-dht -- --nocapture

echo "== bench (short) =="
cargo bench -p svc-dht --bench lookup_bench -- --measurement-time 2 --warm-up-time 1 || true

echo "== local E2E (provide/find/metrics) =="
# runs against a locally started svc-dht (in another terminal)
crates/svc-dht/scripts/run-local.sh || true

echo "== done =="
