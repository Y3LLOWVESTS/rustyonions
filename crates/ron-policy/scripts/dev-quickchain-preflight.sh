#!/usr/bin/env bash
# RO:WHAT — Crate-local QuickChain Phase-0 preflight runner for ron-policy.
# RO:WHY — Keeps policy declarative and proves it is not wallet/ledger/root/settlement authority.
# RO:INTERACTS — docs, schema, quickchain_preflight_* tests, existing ron-policy unit/economics tests.
# RO:INVARIANTS — no roots/checkpoints/validators/settlement; no fake receipts/balances/unlocks.
set -euo pipefail

mode="${1:-}"
if [[ "$mode" == "--check" ]]; then
  cargo fmt -p ron-policy -- --check
elif [[ -z "$mode" ]]; then
  cargo fmt -p ron-policy
else
  echo "usage: $0 [--check]" >&2
  exit 2
fi

cargo test -p ron-policy --test quickchain_preflight_docs
cargo test -p ron-policy --test quickchain_preflight_boundary
cargo test -p ron-policy --test quickchain_preflight_public_surface_boundary
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_identifier_non_authority
cargo test -p ron-policy --test quickchain_preflight_schema_non_authority
cargo test -p ron-policy --test quickchain_preflight_parser_consistency

cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --test unit_model_serde_strict
cargo test -p ron-policy --test unit_eval_determinism
cargo test -p ron-policy --test unit_first_match_wins
cargo test -p ron-policy --test golden_reasons

cargo clippy -p ron-policy --all-targets -- -D warnings
