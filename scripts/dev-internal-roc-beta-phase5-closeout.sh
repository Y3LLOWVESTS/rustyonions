#!/usr/bin/env bash
# RO:WHAT — Root Internal ROC Beta Phase 5 closeout gate.
# RO:WHY — Consolidates tokenomics config validation, economics non-authority, anti-farming, event-class isolation, and policy/rewarder gates into one reproducible closeout command.
# RO:INTERACTS — configs/roc-economics.toml, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy.
# RO:INVARIANTS — svc-wallet remains mutation front-door; ron-ledger remains durable economic truth; accounting/rewarder/policy remain non-authoritative; raw engagement cannot mint/allocate protocol ROC.
# RO:METRICS — none.
# RO:CONFIG — validates that Phase 5 config exists and future bridge/staking remain inert through crate tests.
# RO:SECURITY — bash/cargo-only; no secrets; no bridge, staking, liquidity, external settlement, fake receipts, fake balances, fake finality, or silent spend.
# RO:TEST — bash scripts/dev-internal-roc-beta-phase5-closeout.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'Internal ROC Beta Phase 5 closeout failed: %s\n' "$*" >&2
  exit 1
}

say() {
  printf '\n== %s ==\n' "$*"
}

run() {
  printf '+'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

require_file() {
  local path="$1"
  [ -f "$path" ] || fail "missing required file: $path"
}

require_no_python_helpers_under() {
  local crate_dir="$1"
  local crate_name="$2"

  local hits
  hits="$(find "$crate_dir" \
    -path "$crate_dir/target" -prune -o \
    -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"

  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    fail "$crate_name closeout tooling must remain bash/cargo-only"
  fi
}

run_test() {
  local package="$1"
  local target="$2"

  require_file "crates/$package/tests/$target.rs"
  run "$CARGO" test -p "$package" --test "$target"
}

run_fmt_check() {
  local package="$1"
  run "$CARGO" fmt -p "$package" -- --check
}

run_clippy() {
  local package="$1"

  case "$package" in
    ron-policy)
      run "$CARGO" clippy -p "$package" --all-targets --no-deps -- -D warnings
      ;;
    *)
      run "$CARGO" clippy -p "$package" --all-targets -- -D warnings
      ;;
  esac
}

say "Internal ROC Beta Phase 5 closeout inventory"

require_file "configs/roc-economics.toml"
require_file "docs/internal-roc-beta/PHASE5_CLOSEOUT.md"

require_file "crates/ron-proto/tests/internal_roc_beta_phase5_economics_config_dto.rs"
require_file "crates/ron-ledger/tests/internal_roc_beta_phase5_economics_config_non_authority.rs"
require_file "crates/svc-wallet/tests/internal_roc_beta_phase5_config_non_authority.rs"
require_file "crates/ron-accounting/tests/internal_roc_beta_phase5_config_label_non_authority.rs"
require_file "crates/ron-accounting/tests/internal_roc_beta_phase5_event_class_antifarming.rs"
require_file "crates/svc-rewarder/tests/internal_roc_beta_phase5_config_driven_planning.rs"
require_file "crates/svc-rewarder/tests/internal_roc_beta_phase5_antifarming_event_gates.rs"
require_file "crates/svc-rewarder/tests/internal_roc_beta_phase5_policy_gate_interlock.rs"
require_file "crates/ron-policy/tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs"
require_file "crates/ron-policy/tests/internal_roc_beta_phase5_antifarming_policy_gate.rs"

if [ -d "crates/svc-ads" ]; then
  say "svc-ads feature gate check"
  require_file "crates/svc-ads/docs/INTERNAL_ROC_BETA_ADS_FEATURE_GATE.md"
  printf 'svc-ads exists and carries Internal ROC Beta ads feature-gate parking note.\n'
else
  say "svc-ads feature gate check"
  printf 'svc-ads crate is absent in this checkout; ads scope remains deferred/parked.\n'
fi

say "tooling boundary"
for crate in ron-proto ron-ledger svc-wallet ron-accounting svc-rewarder ron-policy; do
  require_no_python_helpers_under "crates/$crate" "$crate"
done
printf 'bash/cargo-only helper boundary clean for Phase 5 closeout crate set.\n'

say "fmt check"
for crate in ron-proto ron-ledger svc-wallet ron-accounting svc-rewarder ron-policy; do
  run_fmt_check "$crate"
done

say "Phase 5 Round 1 tokenomics/config proof"
run_test ron-proto internal_roc_beta_phase5_economics_config_dto
run_test ron-ledger internal_roc_beta_phase5_economics_config_non_authority
run_test svc-wallet internal_roc_beta_phase5_config_non_authority
run_test ron-accounting internal_roc_beta_phase5_config_label_non_authority
run_test svc-rewarder internal_roc_beta_phase5_config_driven_planning
run_test ron-policy internal_roc_beta_phase5_economics_toml_policy_validation

say "Phase 5 Round 2 anti-farming/event-class/policy-gate proof"
run_test ron-accounting internal_roc_beta_phase5_event_class_antifarming
run_test svc-rewarder internal_roc_beta_phase5_antifarming_event_gates
run_test svc-rewarder internal_roc_beta_phase5_policy_gate_interlock
run_test ron-policy internal_roc_beta_phase5_antifarming_policy_gate

say "Phase 3 carry-forward reward/payout boundary regressions"
run_test ron-ledger internal_roc_beta_phase3_reward_plan_non_authority
run_test ron-ledger internal_roc_beta_phase3_approved_payout_replay
run_test svc-wallet internal_roc_beta_phase3_accounting_observer_boundary
run_test svc-wallet internal_roc_beta_phase3_approved_payout_execution_boundary
run_test ron-accounting internal_roc_beta_phase3_snapshot_event_class_boundary
run_test ron-accounting internal_roc_beta_phase3_approved_payout_observation_boundary
run_test svc-rewarder internal_roc_beta_phase3_reward_plan_boundary
run_test svc-rewarder internal_roc_beta_phase3_approved_payout_intent_boundary
run_test ron-policy internal_roc_beta_phase3_reward_plan_policy_gate
run_test ron-policy internal_roc_beta_phase3_approved_payout_policy_gate

say "QuickChain parked boundary spot-checks"
run_test svc-rewarder quickchain_preflight_raw_engagement
run_test svc-rewarder quickchain_preflight_no_direct_mutation
run_test svc-rewarder quickchain_preflight_funding_source
run_test ron-policy quickchain_preflight_decision_non_authority
run_test ron-accounting quickchain_preflight_snapshot_non_authority
run_test svc-wallet quickchain_preflight_no_runtime_authority
run_test ron-ledger quickchain_pre_root_boundary
run_test ron-proto quickchain_ids_and_money

say "strict clippy once per Phase 5 closeout crate"
for crate in ron-proto ron-ledger svc-wallet ron-accounting svc-rewarder ron-policy; do
  run_clippy "$crate"
done

say "Internal ROC Beta Phase 5 closeout result"
printf 'Internal ROC Beta Phase 5 tokenomics config/anti-farming proof complete.\n'
printf 'Phase 5 status: COMPLETE / GREEN / PARKED.\n'
printf '\n'
printf 'Locked boundaries:\n'
printf -- '- configs/roc-economics.toml is the mutable economics config proof surface.\n'
printf -- '- no floats or unsafe tokenomics values pass validation.\n'
printf -- '- bridge/staking placeholders remain inert.\n'
printf -- '- raw engagement cannot directly mint or allocate ROC.\n'
printf -- '- analytics_only and metering cannot directly become payout material.\n'
printf -- '- proof_eligible requires verification/caps/policy gate.\n'
printf -- '- ad_budgeted remains explicit-budget/deferred and cannot use protocol-pool emission.\n'
printf -- '- accounting remains derivative snapshot/report infrastructure.\n'
printf -- '- policy remains declarative-only.\n'
printf -- '- rewarder remains non-mutating planning-only.\n'
printf -- '- svc-wallet remains mutation front-door.\n'
printf -- '- ron-ledger remains durable receipt/balance truth.\n'
printf -- '- QuickChain remains parked; no public chain/runtime completion is implied.\n'
