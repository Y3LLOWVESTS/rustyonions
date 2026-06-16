#!/usr/bin/env bash
# RO:WHAT — Local svc-wallet gate including inert QuickChain preflight compatibility.
# RO:WHY — Keeps wallet as mutation front-door while verifying it can compile
# against ron-ledger's gated QuickChain pre-root surface.
# RO:INTERACTS — svc-wallet, ron-ledger quickchain-preflight feature, cargo,
# crates/svc-wallet/docs/quickchain-preflight.md.
# RO:INVARIANTS — does not enable roots, validators, settlement, bridges,
# external anchors, pruning, staking, liquidity, or live chain authority.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — QuickChain remains feature-gated and disabled by default.
# RO:TEST — run from workspace root.

set -euo pipefail

echo "== svc-wallet normal gate =="
cargo fmt -p svc-wallet -- --check
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets

echo "== svc-wallet focused QuickChain preflight gates =="
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_request_poisoning_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_projection_validation_matrix

echo "== svc-wallet full QuickChain preflight gate =="
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
cargo test -p svc-wallet --all-targets --features quickchain-preflight

echo "== svc-wallet QuickChain preflight gate passed =="
