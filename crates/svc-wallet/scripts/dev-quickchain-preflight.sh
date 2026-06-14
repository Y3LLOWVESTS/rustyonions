#!/usr/bin/env bash
# RO:WHAT — Local svc-wallet gate including inert QuickChain preflight compatibility.
# RO:WHY — Keeps wallet as mutation front-door while verifying it can compile
# against ron-ledger's gated QuickChain pre-root surface.
# RO:INTERACTS — svc-wallet, ron-ledger quickchain-preflight feature, cargo.
# RO:INVARIANTS — does not enable roots, validators, settlement, bridges,
# external anchors, pruning, staking, liquidity, or live chain authority.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — QuickChain remains feature-gated and disabled by default.
# RO:TEST — run from workspace root.

set -euo pipefail

cargo fmt -p svc-wallet -- --check
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets

cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
cargo test -p svc-wallet --all-targets --features quickchain-preflight
