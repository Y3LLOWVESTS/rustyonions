#!/usr/bin/env bash
set -euo pipefail

# svc-gateway QuickChain Phase-0 preflight gate.
#
# RO:WHAT — Runs the focused gateway QuickChain boundary suites plus proxy regressions.
# RO:WHY — Keeps gateway proxy/admission behavior from drifting into wallet, ledger, root, validator, bridge, or finality authority.
# RO:INTERACTS — cargo test, cargo clippy, svc-gateway tests.
# RO:INVARIANTS — no QuickChain runtime, no gateway-side economic mutation, no fake receipts/balances/finality.
# RO:TEST — run from repo root with: crates/svc-gateway/scripts/dev-quickchain-preflight.sh

cargo test -p svc-gateway --test quickchain_preflight_boundary
cargo test -p svc-gateway --test quickchain_preflight_docs
cargo test -p svc-gateway --test quickchain_preflight_no_fake_receipts
cargo test -p svc-gateway --test quickchain_preflight_cache_boundary

cargo test -p svc-gateway --test app_proxy
cargo test -p svc-gateway --test paid_storage_estimate_proxy
cargo test -p svc-gateway --test paid_storage_write_proxy
cargo test -p svc-gateway --test product_routes_proxy

cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings

echo "svc-gateway QuickChain preflight gate passed"
