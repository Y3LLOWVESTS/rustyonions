#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO="${CARGO:-cargo}"

run() {
  printf '\n== %s ==\n' "$*"
  "$@"
}

require_file() {
  if [[ ! -f "$1" ]]; then
    printf 'missing required Phase 6 tokenomics proof file: %s\n' "$1" >&2
    exit 1
  fi
}

printf '== Internal ROC Beta Phase 6 tokenomics/anti-farming proof target ==\n'

require_file configs/roc-economics.toml
require_file crates/ron-ledger/tests/internal_roc_beta_phase5_economics_config_non_authority.rs
require_file crates/svc-wallet/tests/internal_roc_beta_phase5_config_non_authority.rs
require_file crates/ron-accounting/tests/internal_roc_beta_phase5_config_label_non_authority.rs
require_file crates/ron-accounting/tests/internal_roc_beta_phase5_event_class_antifarming.rs
require_file crates/svc-rewarder/tests/internal_roc_beta_phase5_config_driven_planning.rs
require_file crates/svc-rewarder/tests/internal_roc_beta_phase5_antifarming_event_gates.rs
require_file crates/svc-rewarder/tests/internal_roc_beta_phase5_policy_gate_interlock.rs
require_file crates/ron-policy/tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs
require_file crates/ron-policy/tests/internal_roc_beta_phase5_antifarming_policy_gate.rs

run "$CARGO" fmt -p ron-ledger -- --check
run "$CARGO" fmt -p svc-wallet -- --check
run "$CARGO" fmt -p ron-accounting -- --check
run "$CARGO" fmt -p svc-rewarder -- --check
run "$CARGO" fmt -p ron-policy -- --check

run "$CARGO" test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase5_economics_config_non_authority
run "$CARGO" test -p svc-wallet --test internal_roc_beta_phase5_config_non_authority
run "$CARGO" test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority
run "$CARGO" test -p ron-accounting --test internal_roc_beta_phase5_event_class_antifarming
run "$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
run "$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
run "$CARGO" test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
run "$CARGO" test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
run "$CARGO" test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate

printf '\n== Internal ROC Beta Phase 6 tokenomics/anti-farming proof target passed ==\n'
printf '== config remains non-authority; raw engagement cannot directly mint/allocate ROC ==\n'
