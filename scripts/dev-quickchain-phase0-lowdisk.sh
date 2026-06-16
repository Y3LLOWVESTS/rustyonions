#!/usr/bin/env bash
# RO:WHAT — Low-disk QuickChain Phase-0 / QC-0A runner.
# RO:WHY — Runs one gate at a time and deletes build artifacts between gates.
# RO:INTERACTS — ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-storage, svc-gateway, omnigate, svc-index, ron-policy.
# RO:INVARIANTS — no roots/checkpoints/validators/settlement/bridges/staking/liquidity/ROX/Solana.
# RO:CONFIG — QUICKCHAIN_LOW_DISK_TARGET_BASE may override temp target root; QUICKCHAIN_KEEP_TARGET=1 keeps artifacts for debugging.
# RO:SECURITY — local build runner only; does not start services or touch secrets.
set -euo pipefail

CARGO="${CARGO:-cargo}"
MODE="check"
ONLY=""
KEEP_TARGET="${QUICKCHAIN_KEEP_TARGET:-0}"

usage() {
  cat <<'USAGE'
Usage: scripts/dev-quickchain-phase0-lowdisk.sh [--run] [--check] [--only <gate>] [--list]

Low-disk QuickChain Phase-0 / QC-0A runner.

Options:
  --check        use fmt --check where supported; default
  --run          allow fmt writes where crate-local scripts do so
  --only <gate>  run one gate only
  --list         list gate names
  -h,--help      show help

Gate names:
  ron-proto
  ron-ledger
  svc-wallet
  ron-accounting
  svc-rewarder
  svc-storage
  svc-gateway
  omnigate
  svc-index
  ron-policy

Environment:
  CARGO=<path>                                  cargo binary override
  QUICKCHAIN_LOW_DISK_TARGET_BASE=<path>        isolated target root
  QUICKCHAIN_KEEP_TARGET=1                      do not delete target after each gate
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      MODE="check"
      shift
      ;;
    --run)
      MODE="run"
      shift
      ;;
    --only)
      ONLY="${2:-}"
      if [[ -z "$ONLY" ]]; then
        echo "missing value for --only" >&2
        exit 2
      fi
      shift 2
      ;;
    --list)
      cat <<'LIST'
ron-proto
ron-ledger
svc-wallet
ron-accounting
svc-rewarder
svc-storage
svc-gateway
omnigate
svc-index
ron-policy
LIST
      exit 0
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

TARGET_BASE="${QUICKCHAIN_LOW_DISK_TARGET_BASE:-$ROOT_DIR/.quickchain-target-lowdisk}"

export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export RUSTFLAGS="${RUSTFLAGS:-} -C debuginfo=0"

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

cleanup_target() {
  local gate="$1"
  local dir="$TARGET_BASE/$gate"
  if [[ "$KEEP_TARGET" == "1" ]]; then
    echo "keeping target dir: $dir"
  else
    echo "cleaning target dir: $dir"
    rm -rf "$dir"
  fi
}

with_gate_target() {
  local gate="$1"
  shift

  local dir="$TARGET_BASE/$gate"
  rm -rf "$dir"
  mkdir -p "$dir"

  export CARGO_TARGET_DIR="$dir"

  section "$gate"
  echo "target: $CARGO_TARGET_DIR"
  echo "mode: $MODE"
  echo "CARGO_INCREMENTAL=$CARGO_INCREMENTAL"
  echo "RUSTFLAGS=$RUSTFLAGS"

  "$@"

  cleanup_target "$gate"
}

fmt_pkg() {
  local pkg="$1"
  if [[ "$MODE" == "check" ]]; then
    run "$CARGO" fmt -p "$pkg" -- --check
  else
    run "$CARGO" fmt -p "$pkg"
  fi
}

run_quickchain_tests_matching() {
  local pkg="$1"
  local crate_dir="$2"
  shift 2

  local found="0"
  for test_path in "$crate_dir"/tests/quickchain*.rs; do
    if [[ ! -f "$test_path" ]]; then
      continue
    fi
    found="1"
    local test_name
    test_name="$(basename "$test_path" .rs)"
    run "$CARGO" test -p "$pkg" "$@" --test "$test_name"
  done

  if [[ "$found" != "1" ]]; then
    echo "no quickchain*.rs tests found for $pkg under $crate_dir/tests" >&2
    exit 1
  fi
}

gate_ron_proto() {
  fmt_pkg ron-proto
  run_quickchain_tests_matching ron-proto crates/ron-proto
  run "$CARGO" clippy -p ron-proto --all-targets -- -D warnings
}

gate_ron_ledger() {
  fmt_pkg ron-ledger
  run_quickchain_tests_matching ron-ledger crates/ron-ledger --features quickchain-preflight
  run "$CARGO" clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
}

gate_script() {
  local script="$1"
  if [[ ! -f "$script" ]]; then
    echo "missing script: $script" >&2
    exit 1
  fi

  if [[ "$MODE" == "check" && "$script" == "crates/ron-policy/scripts/dev-quickchain-preflight.sh" ]]; then
    run bash "$script" --check
  else
    run bash "$script"
  fi
}

gate_workspace_docs() {
  section "workspace doctrine/inventory checks"
  for path in \
    Cargo.toml \
    QUICKCHAIN_REVIEW_BUNDLE.MD \
    QUICKCHAIN.MD \
    QUICKCHAIN_QC0A_COMBINED.MD \
    crates/omnigate/docs/quickchain-preflight.md \
    crates/ron-accounting/docs/quickchain-preflight.md \
    crates/ron-policy/docs/quickchain-preflight.md \
    crates/svc-gateway/docs/quickchain-preflight.md \
    crates/svc-index/docs/quickchain-preflight.md \
    crates/svc-rewarder/docs/quickchain-preflight.md \
    crates/svc-storage/docs/quickchain-preflight.md \
    crates/svc-wallet/docs/quickchain-preflight.md
  do
    if [[ ! -f "$path" ]]; then
      echo "missing required file: $path" >&2
      exit 1
    fi
  done
  echo "workspace QuickChain doctrine files are present"
}

run_gate() {
  local gate="$1"

  case "$gate" in
    ron-proto)
      with_gate_target "$gate" gate_ron_proto
      ;;
    ron-ledger)
      with_gate_target "$gate" gate_ron_ledger
      ;;
    svc-wallet)
      with_gate_target "$gate" gate_script crates/svc-wallet/scripts/dev-quickchain-preflight.sh
      ;;
    ron-accounting)
      with_gate_target "$gate" gate_script crates/ron-accounting/scripts/dev-quickchain-preflight.sh
      ;;
    svc-rewarder)
      with_gate_target "$gate" gate_script crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
      ;;
    svc-storage)
      with_gate_target "$gate" gate_script crates/svc-storage/scripts/dev-quickchain-preflight.sh
      ;;
    svc-gateway)
      with_gate_target "$gate" gate_script crates/svc-gateway/scripts/dev-quickchain-preflight.sh
      ;;
    omnigate)
      with_gate_target "$gate" gate_script crates/omnigate/scripts/dev-quickchain-preflight.sh
      ;;
    svc-index)
      with_gate_target "$gate" gate_script crates/svc-index/scripts/dev-quickchain-preflight.sh
      ;;
    ron-policy)
      with_gate_target "$gate" gate_script crates/ron-policy/scripts/dev-quickchain-preflight.sh
      ;;
    *)
      echo "unknown gate: $gate" >&2
      exit 2
      ;;
  esac
}

GATES=(
  ron-proto
  ron-ledger
  svc-wallet
  ron-accounting
  svc-rewarder
  svc-storage
  svc-gateway
  omnigate
  svc-index
  ron-policy
)

section "QuickChain Phase-0 / QC-0A low-disk runner"
echo "repo: $ROOT_DIR"
echo "target base: $TARGET_BASE"
echo "This runner cleans each gate before moving to the next."

if [[ -n "$ONLY" ]]; then
  run_gate "$ONLY"
else
  for gate in "${GATES[@]}"; do
    run_gate "$gate"
  done
  gate_workspace_docs
fi

section "low-disk QuickChain run complete"
