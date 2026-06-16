#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

echo "== ron-accounting QuickChain Phase-0 preflight =="
echo "workspace: $ROOT_DIR"
echo

echo "== fmt check =="
cargo fmt -p ron-accounting -- --check
echo

echo "== clippy strict gate =="
cargo clippy -p ron-accounting --all-targets -- -D warnings
echo

echo "== ron-accounting all-target tests =="
cargo test -p ron-accounting --all-targets
echo

echo "== ron-accounting WAL feature tests =="
cargo test -p ron-accounting --all-targets --features wal
echo

echo "== focused QuickChain preflight tests =="
cargo test -p ron-accounting --test quickchain_preflight_boundary
cargo test -p ron-accounting --test quickchain_preflight_ingest_poisoning
cargo test -p ron-accounting --test quickchain_preflight_snapshot_non_authority
cargo test -p ron-accounting --test quickchain_preflight_reward_dto_strictness
cargo test -p ron-accounting --test quickchain_preflight_reward_projection_boundary
cargo test -p ron-accounting --test quickchain_preflight_event_class_boundary
echo

echo "== ron-accounting QuickChain preflight gate passed =="
