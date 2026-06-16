#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

echo "== svc-storage QuickChain preflight =="
echo "repo: $ROOT"

test -f crates/svc-storage/docs/quickchain-preflight.md

echo
echo "== format check =="
cargo fmt -p svc-storage -- --check

echo
echo "== focused QuickChain preflight tests =="
cargo test -p svc-storage --test quickchain_preflight_boundary
cargo test -p svc-storage --test quickchain_preflight_b3_integrity
cargo test -p svc-storage --test quickchain_preflight_no_direct_mutation
cargo test -p svc-storage --test quickchain_preflight_paid_cache
cargo test -p svc-storage --test quickchain_preflight_economics_quote
cargo test -p svc-storage --test quickchain_preflight_settlement_boundary
cargo test -p svc-storage --test quickchain_preflight_range_media
cargo test -p svc-storage --test quickchain_preflight_observability
cargo test -p svc-storage --test quickchain_preflight_docs

echo
echo "== svc-storage all-targets test =="
cargo test -p svc-storage --all-targets

echo
echo "== svc-storage clippy =="
cargo clippy -p svc-storage --all-targets -- -D warnings

echo
echo "== svc-storage QuickChain preflight gate passed =="
