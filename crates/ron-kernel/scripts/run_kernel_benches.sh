#!/usr/bin/env bash
# RO: scripts/run_kernel_benches.sh
set -euo pipefail

echo "=== System ==="
uname -a || true
rustc -Vv
cargo -V

echo
echo "=== Running benches (stable) ==="
cargo bench -p ron-kernel

echo
echo "=== Criterion reports ==="
echo "Open: target/criterion/report/index.html"
