#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

echo "== svc-rewarder QuickChain preflight =="
echo "repo: $ROOT"

test -f crates/svc-rewarder/docs/quickchain-preflight.md

echo
echo "== format check =="
if ! cargo fmt -p svc-rewarder -- --check; then
  echo
  echo "cargo fmt --check failed."
  echo "Run this from the repository root, then rerun this script:"
  echo
  echo "  cargo fmt -p svc-rewarder"
  echo "  crates/svc-rewarder/scripts/dev-quickchain-preflight.sh"
  echo
  exit 1
fi

echo
echo "== focused QuickChain preflight tests =="
cargo test -p svc-rewarder --test quickchain_preflight_boundary
cargo test -p svc-rewarder --test quickchain_preflight_raw_engagement
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_docs

echo
echo "== svc-rewarder all-targets test =="
cargo test -p svc-rewarder --all-targets

echo
echo "== svc-rewarder clippy =="
cargo clippy -p svc-rewarder --all-targets -- -D warnings

echo
echo "== svc-rewarder QuickChain preflight gate passed =="