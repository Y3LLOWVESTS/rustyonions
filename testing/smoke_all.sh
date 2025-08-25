#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all
cargo build -p tldctl -p gateway
testing/smoke_402.sh
testing/smoke_range.sh
testing/smoke_relations.sh
echo "[PASS] all smokes green."
