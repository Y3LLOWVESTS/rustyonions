#!/usr/bin/env bash
# RO:WHAT — Local svc-index QuickChain Phase-0/preflight gate.
# RO:WHY — Prove svc-index remains lookup/pointer infrastructure, not economic or QuickChain authority.
# RO:INTERACTS — docs/quickchain-preflight.md and quickchain_preflight_* tests.
# RO:INVARIANTS — no roots/checkpoints/validators/settlement/wallet mutation/ledger mutation/fake receipts.
# RO:TEST — run from repo root with `bash crates/svc-index/scripts/dev-quickchain-preflight.sh`.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

echo "== svc-index QuickChain Phase-0 preflight =="
echo "repo: $ROOT"

echo
echo "== format =="
cargo fmt -p svc-index

echo
echo "== focused QuickChain preflight tests =="
cargo test -p svc-index --test quickchain_preflight_docs
cargo test -p svc-index --test quickchain_preflight_boundary
cargo test -p svc-index --test quickchain_preflight_pointer_authority
cargo test -p svc-index --test quickchain_preflight_routes

echo
echo "== existing svc-index contract smoke =="
cargo test -p svc-index --test http_contract
cargo test -p svc-index --test integration

echo
echo "== clippy =="
cargo clippy -p svc-index --all-targets -- -D warnings

echo
echo "svc-index QuickChain Phase-0 preflight passed."
