#!/usr/bin/env bash
# RO:WHAT — Workspace QuickChain Phase-0 / QC-0A umbrella preflight runner.
# RO:WHY — Proves the parked crate-local QuickChain gates still pass together before Phase-0 acceptance.
# RO:INTERACTS — ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-storage, svc-gateway, omnigate, svc-index, ron-policy.
# RO:INVARIANTS — no roots/checkpoints/validators/settlement/bridges/staking/liquidity/ROX/Solana or client chain authority.
# RO:METRICS — none; this is a local developer/CI gate.
# RO:CONFIG — CARGO may override cargo binary; QUICKCHAIN_SKIP_WORKSPACE_CHECK=1 skips final cargo check.
# RO:SECURITY — fails closed on missing docs/scripts/tests; does not start services or touch secrets.
# RO:TEST — bash -n scripts/dev-quickchain-phase0.sh; scripts/dev-quickchain-phase0.sh --check.
set -euo pipefail

CARGO="${CARGO:-cargo}"
mode="run"
list_only="0"

usage() {
  cat <<'USAGE'
Usage: scripts/dev-quickchain-phase0.sh [--check] [--list]

Runs the RustyOnions QuickChain Phase-0 / QC-0A workspace umbrella gate.

Options:
  --check   use cargo fmt --check before running gates; avoids intentional formatting writes
  --list    print the gate order without running commands
  -h,--help show this help

Environment:
  CARGO=<path>                         cargo binary override
  QUICKCHAIN_SKIP_WORKSPACE_CHECK=1    skip final cargo check --workspace
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --list)
      list_only="1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_dir() {
  local path="$1"
  if [[ ! -d "$path" ]]; then
    echo "missing required directory: $path" >&2
    exit 1
  fi
}

section() {
  echo
  echo "========================================================================"
  echo "== $*"
  echo "========================================================================"
}

run() {
  echo "+ $*"
  "$@"
}

run_bash_script() {
  local script="$1"
  require_file "$script"
  run bash "$script"
}

fmt_pkg() {
  local pkg="$1"
  if [[ "$mode" == "check" ]]; then
    run "$CARGO" fmt -p "$pkg" -- --check
  else
    run "$CARGO" fmt -p "$pkg"
  fi
}

clippy_pkg() {
  local pkg="$1"
  shift || true
  run "$CARGO" clippy -p "$pkg" --all-targets "$@" -- -D warnings
}

run_quickchain_tests_matching() {
  local pkg="$1"
  local crate_dir="$2"
  local feature_args="$3"
  local found="0"

  require_dir "$crate_dir/tests"

  for test_path in "$crate_dir"/tests/quickchain*.rs; do
    if [[ ! -f "$test_path" ]]; then
      continue
    fi
    found="1"
    local test_name
    test_name="$(basename "$test_path" .rs)"
    if [[ -n "$feature_args" ]]; then
      # shellcheck disable=SC2086
      run "$CARGO" test -p "$pkg" $feature_args --test "$test_name"
    else
      run "$CARGO" test -p "$pkg" --test "$test_name"
    fi
  done

  if [[ "$found" != "1" ]]; then
    echo "no quickchain*.rs tests found for $pkg under $crate_dir/tests" >&2
    exit 1
  fi
}

run_ron_proto_gate() {
  section "1/10 ron-proto — DTOs, canonical bytes, strict validators"
  fmt_pkg ron-proto
  run_quickchain_tests_matching ron-proto crates/ron-proto ""
  clippy_pkg ron-proto
}

run_ron_ledger_gate() {
  section "2/10 ron-ledger — economic truth, replay, idempotency, holds"
  fmt_pkg ron-ledger
  run_quickchain_tests_matching ron-ledger crates/ron-ledger ""
  clippy_pkg ron-ledger
}

run_svc_wallet_gate() {
  section "3/10 svc-wallet — mutation front-door, no chain authority"
  run_bash_script crates/svc-wallet/scripts/dev-quickchain-preflight.sh
}

run_ron_accounting_gate() {
  section "4/10 ron-accounting — snapshots/projections, not balance truth"
  run_bash_script crates/ron-accounting/scripts/dev-quickchain-preflight.sh
}

run_svc_rewarder_gate() {
  section "5/10 svc-rewarder — payout planning, no ledger mutation"
  run_bash_script crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
}

run_svc_storage_gate() {
  section "6/10 svc-storage — bytes/ranges/paid enforcement boundaries"
  run_bash_script crates/svc-storage/scripts/dev-quickchain-preflight.sh
}

run_svc_gateway_gate() {
  section "7/10 svc-gateway — public boundary, no economic mutation"
  run_bash_script crates/svc-gateway/scripts/dev-quickchain-preflight.sh
}

run_omnigate_gate() {
  section "8/10 omnigate — hydrator/orchestrator, no ledger mutation"
  run_bash_script crates/omnigate/scripts/dev-quickchain-preflight.sh
}

run_svc_index_gate() {
  section "9/10 svc-index — lookup/pointer service, not authority"
  run_bash_script crates/svc-index/scripts/dev-quickchain-preflight.sh
}

run_ron_policy_gate() {
  section "10/10 ron-policy — declarative policy, not receipt/balance/finality authority"
  require_file crates/ron-policy/scripts/dev-quickchain-preflight.sh
  if [[ "$mode" == "check" ]]; then
    run bash crates/ron-policy/scripts/dev-quickchain-preflight.sh --check
  else
    run bash crates/ron-policy/scripts/dev-quickchain-preflight.sh
  fi
}

run_workspace_doctrine_checks() {
  section "workspace doctrine files and parked gate inventory"

  require_file Cargo.toml
  require_dir crates
  require_dir scripts
  require_file QUICKCHAIN_REVIEW_BUNDLE.MD
  require_file QUICKCHAIN.MD
  require_file QUICKCHAIN_QC0A_COMBINED.MD

  for doc in \
    crates/omnigate/docs/quickchain-preflight.md \
    crates/ron-accounting/docs/quickchain-preflight.md \
    crates/ron-policy/docs/quickchain-preflight.md \
    crates/svc-gateway/docs/quickchain-preflight.md \
    crates/svc-index/docs/quickchain-preflight.md \
    crates/svc-rewarder/docs/quickchain-preflight.md \
    crates/svc-storage/docs/quickchain-preflight.md \
    crates/svc-wallet/docs/quickchain-preflight.md
  do
    require_file "$doc"
  done

  for script in \
    crates/omnigate/scripts/dev-quickchain-preflight.sh \
    crates/ron-accounting/scripts/dev-quickchain-preflight.sh \
    crates/ron-policy/scripts/dev-quickchain-preflight.sh \
    crates/svc-gateway/scripts/dev-quickchain-preflight.sh \
    crates/svc-index/scripts/dev-quickchain-preflight.sh \
    crates/svc-rewarder/scripts/dev-quickchain-preflight.sh \
    crates/svc-storage/scripts/dev-quickchain-preflight.sh \
    crates/svc-wallet/scripts/dev-quickchain-preflight.sh
  do
    require_file "$script"
  done

  echo "required QuickChain docs/scripts are present"
}

print_gate_order() {
  cat <<'ORDER'
QuickChain Phase-0 / QC-0A workspace gate order:
  1. ron-proto
  2. ron-ledger
  3. svc-wallet
  4. ron-accounting
  5. svc-rewarder
  6. svc-storage
  7. svc-gateway
  8. omnigate
  9. svc-index
 10. ron-policy
 11. workspace doctrine/inventory checks
 12. cargo check --workspace, unless QUICKCHAIN_SKIP_WORKSPACE_CHECK=1
ORDER
}

if [[ "$list_only" == "1" ]]; then
  print_gate_order
  exit 0
fi

section "QuickChain Phase-0 / QC-0A workspace umbrella gate"
echo "repo: $ROOT_DIR"
echo "mode: $mode"
echo "cargo: $CARGO"
echo
echo "This gate is intentionally pre-root/pre-validator/pre-settlement."
echo "It must not create roots, checkpoints, validators, anchors, bridges, staking, liquidity, ROX, Solana, or client chain authority."

if [[ "$mode" == "check" ]]; then
  section "workspace format check"
  run "$CARGO" fmt --all -- --check
fi

run_ron_proto_gate
run_ron_ledger_gate
run_svc_wallet_gate
run_ron_accounting_gate
run_svc_rewarder_gate
run_svc_storage_gate
run_svc_gateway_gate
run_omnigate_gate
run_svc_index_gate
run_ron_policy_gate
run_workspace_doctrine_checks

if [[ "${QUICKCHAIN_SKIP_WORKSPACE_CHECK:-0}" != "1" ]]; then
  section "final workspace compile check"
  run "$CARGO" check --workspace
else
  section "final workspace compile check skipped"
  echo "QUICKCHAIN_SKIP_WORKSPACE_CHECK=1"
fi

section "QuickChain Phase-0 / QC-0A workspace umbrella gate passed"
echo "All parked crate-local QuickChain preflight gates completed in stable order."
echo "QuickChain remains future settlement infrastructure; Phase 1 roots/checkpoints/validators remain blocked until locked, independently reproducible vectors are complete."
