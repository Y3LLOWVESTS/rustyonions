#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo clippy -p ron-ledger --all-targets -- -D warnings
cargo test -p ron-ledger --all-targets
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
cargo test -p ron-ledger --all-targets --features quickchain-preflight