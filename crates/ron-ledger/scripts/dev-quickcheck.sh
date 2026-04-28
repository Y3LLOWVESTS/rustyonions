#!/usr/bin/env bash
set -euo pipefail
cargo fmt --check
cargo clippy -p ron-ledger --all-targets -- -D warnings
cargo test -p ron-ledger --all-targets
