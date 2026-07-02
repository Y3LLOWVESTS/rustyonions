#!/usr/bin/env bash
# RO:WHAT — Final aggregate Internal ROC Product Beta Readiness park/smoke/docs sweep.
# RO:WHY — Composes the already parked Internal ROC stabilization surfaces without reopening phases or adding runtime behavior.
# RO:INTERACTS — backend focused stabilization tests, parked pair scripts, optional CrabLink Tauri stabilization park gate.
# RO:INVARIANTS — svc-wallet mutates; ron-ledger records truth; accounting/rewarder/policy/index/storage/gateway/omnigate/client remain non-authority.
# RO:SECURITY — no fake balances/receipts/finality, silent spend, cache-only unlock, direct non-wallet ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
# RO:TEST — bash scripts/dev-internal-roc-product-beta-readiness-park.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUN_INTERNAL_ROC_FOCUSED_TESTS="${RUN_INTERNAL_ROC_FOCUSED_TESTS:-1}"
RUN_INTERNAL_ROC_PAIR_PARKS="${RUN_INTERNAL_ROC_PAIR_PARKS:-0}"
RUN_CRABLINK_TAURI_PARK="${RUN_CRABLINK_TAURI_PARK:-1}"

focused_tests_ran=0
pair_parks_ran=0
crablink_park_ran=0

run_cmd() {
  echo
  echo "+ $*"
  "$@"
}

resolve_crablink_repo() {
  local candidate
  local home="${HOME:-}"

  if [[ -n "${CRABLINK_REPO:-}" ]]; then
    if [[ -f "$CRABLINK_REPO/package.json" && -f "$CRABLINK_REPO/scripts/dev-internal-roc-stabilization-tauri-park.sh" ]]; then
      printf '%s\n' "$CRABLINK_REPO"
      return 0
    fi
    return 1
  fi

  local candidates=(
    "$ROOT/../crablink"
    "$ROOT/../CrabLink"
  )

  if [[ -n "$home" ]]; then
    candidates+=(
      "$home/Desktop/crablink"
      "$home/Desktop/CrabLink"
    )
  fi

  for candidate in "${candidates[@]}"; do
    if [[ -f "$candidate/package.json" && -f "$candidate/scripts/dev-internal-roc-stabilization-tauri-park.sh" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

run_backend_focused_tests() {
  if [[ "$RUN_INTERNAL_ROC_FOCUSED_TESTS" == "0" || "$RUN_INTERNAL_ROC_FOCUSED_TESTS" == "false" || "$RUN_INTERNAL_ROC_FOCUSED_TESTS" == "no" ]]; then
    echo "== Backend focused stabilization tests skipped by RUN_INTERNAL_ROC_FOCUSED_TESTS=$RUN_INTERNAL_ROC_FOCUSED_TESTS =="
    return 0
  fi

  echo "== Internal ROC Product Beta Readiness: backend focused stabilization tests =="

  run_cmd cargo test -p ron-proto --test internal_roc_stabilization_receipt_dto_boundary
  run_cmd cargo test -p ron-ledger --test internal_roc_stabilization_replay_truth_boundary

  run_cmd cargo test -p svc-wallet --test internal_roc_stabilization_mutation_frontdoor_boundary
  run_cmd cargo test -p ron-accounting --test internal_roc_stabilization_snapshot_non_authority_boundary

  run_cmd cargo test -p svc-rewarder --test internal_roc_stabilization_reward_policy_gate_boundary
  run_cmd cargo test -p ron-policy --test internal_roc_stabilization_policy_gate_non_authority_boundary

  run_cmd cargo test -p svc-gateway --test internal_roc_stabilization_paid_route_error_boundary
  run_cmd cargo test -p omnigate --test internal_roc_stabilization_paid_access_error_boundary

  run_cmd cargo test -p svc-storage --test internal_roc_stabilization_b3_artifact_non_authority_boundary
  run_cmd cargo test -p svc-index --test internal_roc_stabilization_pointer_non_authority_boundary

  focused_tests_ran=1
}

run_optional_pair_parks() {
  if [[ "$RUN_INTERNAL_ROC_PAIR_PARKS" == "0" || "$RUN_INTERNAL_ROC_PAIR_PARKS" == "false" || "$RUN_INTERNAL_ROC_PAIR_PARKS" == "no" ]]; then
    echo "== Pair park scripts not rerun by default; set RUN_INTERNAL_ROC_PAIR_PARKS=1 for full repark =="
    return 0
  fi

  echo "== Internal ROC Product Beta Readiness: optional full pair park rerun =="

  run_cmd bash scripts/dev-internal-roc-stabilization-proto-ledger-park.sh
  run_cmd bash scripts/dev-internal-roc-stabilization-wallet-accounting-park.sh
  run_cmd bash scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh
  run_cmd bash scripts/dev-internal-roc-stabilization-gateway-omnigate-paid-route-park.sh
  run_cmd bash scripts/dev-internal-roc-stabilization-storage-index-park.sh

  pair_parks_ran=1
}

run_crablink_tauri_park() {
  if [[ "$RUN_CRABLINK_TAURI_PARK" == "0" || "$RUN_CRABLINK_TAURI_PARK" == "false" || "$RUN_CRABLINK_TAURI_PARK" == "no" ]]; then
    echo "== CrabLink Tauri park skipped by RUN_CRABLINK_TAURI_PARK=$RUN_CRABLINK_TAURI_PARK =="
    return 0
  fi

  local crablink_repo
  if ! crablink_repo="$(resolve_crablink_repo)"; then
    echo "ERROR: CrabLink repo not found." >&2
    echo "Set CRABLINK_REPO=/path/to/crablink or set RUN_CRABLINK_TAURI_PARK=0 for backend-only validation." >&2
    exit 1
  fi

  echo "== Internal ROC Product Beta Readiness: CrabLink Tauri park =="
  echo "== CrabLink repo: $crablink_repo =="

  run_cmd bash -n "$crablink_repo/scripts/dev-internal-roc-stabilization-tauri-park.sh"

  (
    cd "$crablink_repo"
    run_cmd bash scripts/dev-internal-roc-stabilization-tauri-park.sh
  )

  crablink_park_ran=1
}

echo "== Internal ROC Product Beta Readiness: docs/scripts static gate =="
run_cmd bash scripts/check-internal-roc-product-beta-readiness.sh

run_backend_focused_tests
run_optional_pair_parks
run_crablink_tauri_park

echo
echo "== Internal ROC Product Beta Readiness aggregate gate summary =="
echo "focused_backend_tests_ran=$focused_tests_ran"
echo "optional_pair_parks_ran=$pair_parks_ran"
echo "crablink_tauri_park_ran=$crablink_park_ran"

if [[ "$focused_tests_ran" == "1" && "$crablink_park_ran" == "1" ]]; then
  echo
  echo "Internal ROC Stabilization / Product Beta Readiness —"
  echo "final aggregate value-loop product beta readiness gate:"
  echo "COMPLETE / GREEN / PARKED."
  echo
  echo "== backend truth path composed; CrabLink display/user-intent boundary composed =="
  echo "== no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, direct non-wallet ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement introduced =="
else
  echo
  echo "Partial aggregate validation passed, but the full product-beta readiness label requires focused backend tests and CrabLink Tauri park to run."
fi
