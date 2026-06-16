#!/usr/bin/env bash
# RO:WHAT — Fast CI invariants for ron-policy.
# RO:WHY — Runs strict formatting, full tests including QuickChain preflight, and strict clippy.
# RO:INVARIANTS — no magic sleeps; fail fast; policy remains declarative/non-authoritative.
set -euo pipefail

cargo fmt -p ron-policy -- --check
cargo test -p ron-policy
cargo clippy -p ron-policy --all-targets -- -D warnings
