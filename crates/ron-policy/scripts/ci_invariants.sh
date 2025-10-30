#!/usr/bin/env bash
# RO:WHAT — Fast CI invariants for ron-policy (fmt, clippy, tests)
# RO:INVARIANTS — no magic sleeps; fail fast on errors
set -euo pipefail
cargo fmt -p ron-policy -- --check
cargo clippy -p ron-policy --no-deps -- -D warnings
cargo test -p ron-policy
