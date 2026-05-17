#!/usr/bin/env bash
# RO:WHAT — QuickChain preflight gate that verifies internal ROC proofs without implementing chain logic.
# RO:WHY — QUICKCHAIN.MD says chain work starts only after wallet receipts, replay, accounting, and rewarder gates are strong.
# RO:INTERACTS — svc-wallet, ron-ledger, ron-accounting, svc-rewarder, paid site_visit smoke, paid storage/rewarder smokes.
# RO:INVARIANTS — no ROX/Solana/external settlement; no gateway/omnigate ledger mutation; rewarder plans only.
# RO:METRICS — relies on child smoke/service metrics; prints an explicit locked/preflight summary.
# RO:CONFIG — RON_PREFLIGHT_RUN_LIVE, RON_PREFLIGHT_RUN_HEAVY, RON_PREFLIGHT_RUN_CLIPPY, RON_PREFLIGHT_ROC_ECONOMICS_PATH.
# RO:SECURITY — local/dev preflight only; does not create validators, bridges, staking, or public chain state.
# RO:TEST — bash scripts/web3_quickchain_preflight_gate.sh.

set -euo pipefail
IFS=$'\n\t'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RON_PREFLIGHT_RUN_LIVE="${RON_PREFLIGHT_RUN_LIVE:-0}"
RON_PREFLIGHT_RUN_HEAVY="${RON_PREFLIGHT_RUN_HEAVY:-0}"
RON_PREFLIGHT_RUN_CLIPPY="${RON_PREFLIGHT_RUN_CLIPPY:-0}"

# auto:
#   use economics mode when a known ROC economics config exists.
# 0:
#   force legacy paid-storage smoke mode.
# 1:
#   force economics paid-storage smoke mode.
RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS="${RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS:-auto}"

# Optional explicit override. If omitted, this gate prefers the developer
# economics file used by CrabLink local testing, then falls back to canonical.
RON_PREFLIGHT_ROC_ECONOMICS_PATH="${RON_PREFLIGHT_ROC_ECONOMICS_PATH:-}"
RON_PREFLIGHT_ROC_ECONOMICS_ACTION="${RON_PREFLIGHT_ROC_ECONOMICS_ACTION:-paid_storage_put}"

DEV_ECONOMICS_PATH="${ROOT}/configs/roc-economics.dev.toml"
CANON_ECONOMICS_PATH="${ROOT}/configs/roc-economics.toml"

SELF_BASENAME="$(basename "${BASH_SOURCE[0]}")"

step() {
  local label="$1"
  shift

  echo
  echo "============================================================"
  echo "${label}"
  echo "============================================================"
  "$@"
}

skip() {
  local label="$1"
  local reason="$2"

  echo
  echo "skip: ${label}"
  echo "reason: ${reason}"
}

need_file() {
  local file="$1"

  if [[ ! -f "${file}" ]]; then
    echo "error: missing required file: ${file}" >&2
    exit 1
  fi
}

scan_for_premature_quickchain_code() {
  local tmp_hits
  tmp_hits="$(mktemp "${TMPDIR:-/tmp}/rustyonions-quickchain-preflight-hits.XXXXXX")"

  find crates scripts configs \
    \( -path "*/target/*" -o -path "*/.git/*" \) -prune -o \
    -type f \
    \( \
      -name "*.rs" \
      -o -name "*.toml" \
      -o -name "*.json" \
      -o -name "*.yaml" \
      -o -name "*.yml" \
      -o -name "*.sh" \
      -o -name "*.bash" \
    \) \
    ! -name "${SELF_BASENAME}" \
    -print0 2>/dev/null \
    | xargs -0 grep -I -n \
      -e "svc-quickchain" \
      -e "quickchain-validator" \
      -e "quickchain bridge" \
      -e "ROX settlement" \
      >"${tmp_hits}" 2>/dev/null || true

  if [[ -s "${tmp_hits}" ]]; then
    cat "${tmp_hits}"
    rm -f "${tmp_hits}"

    echo
    echo "error: found premature QuickChain/external-settlement implementation strings"
    echo "QuickChain must remain locked until internal ROC gates are complete."
    exit 1
  fi

  rm -f "${tmp_hits}"
}

resolve_roc_economics_path() {
  if [[ -n "${RON_PREFLIGHT_ROC_ECONOMICS_PATH}" ]]; then
    if [[ "${RON_PREFLIGHT_ROC_ECONOMICS_PATH}" = /* ]]; then
      printf '%s\n' "${RON_PREFLIGHT_ROC_ECONOMICS_PATH}"
    else
      printf '%s/%s\n' "${ROOT}" "${RON_PREFLIGHT_ROC_ECONOMICS_PATH}"
    fi
    return 0
  fi

  if [[ -f "${DEV_ECONOMICS_PATH}" ]]; then
    printf '%s\n' "${DEV_ECONOMICS_PATH}"
    return 0
  fi

  if [[ -f "${CANON_ECONOMICS_PATH}" ]]; then
    printf '%s\n' "${CANON_ECONOMICS_PATH}"
    return 0
  fi

  printf '\n'
}

resolve_paid_storage_economics_mode() {
  local economics_path
  economics_path="$(resolve_roc_economics_path)"

  case "${RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS}" in
    0|false|FALSE|no|NO)
      printf '0\n'
      return 0
      ;;
    1|true|TRUE|yes|YES)
      printf '1\n'
      return 0
      ;;
    auto|AUTO)
      if [[ -n "${economics_path}" && -f "${economics_path}" ]]; then
        printf '1\n'
      else
        printf '0\n'
      fi
      return 0
      ;;
    *)
      echo "error: invalid RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS=${RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS}" >&2
      echo "valid values: auto, 0, 1" >&2
      exit 1
      ;;
  esac
}

run_paid_storage_live_smoke() {
  local use_economics
  local economics_path

  use_economics="$(resolve_paid_storage_economics_mode)"
  economics_path="$(resolve_roc_economics_path)"

  if [[ "${use_economics}" == "1" ]]; then
    if [[ -z "${economics_path}" || ! -f "${economics_path}" ]]; then
      echo "error: paid-storage economics mode requested, but no economics config was found" >&2
      echo "looked for:" >&2
      echo "  ${DEV_ECONOMICS_PATH}" >&2
      echo "  ${CANON_ECONOMICS_PATH}" >&2
      echo "or set RON_PREFLIGHT_ROC_ECONOMICS_PATH=/absolute/path/to/config.toml" >&2
      echo "rerun with RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS=0 to force legacy mode." >&2
      exit 1
    fi

    echo "paid storage economics mode: enabled"
    echo "economics path: ${economics_path}"
    echo "economics action: ${RON_PREFLIGHT_ROC_ECONOMICS_ACTION}"

    env \
      WEB3_PAID_STORAGE_USE_ECONOMICS=1 \
      RON_STORAGE_ROC_ECONOMICS_PATH="${economics_path}" \
      RON_STORAGE_ROC_ECONOMICS_ACTION="${RON_PREFLIGHT_ROC_ECONOMICS_ACTION}" \
      bash scripts/web3_paid_storage_live_smoke.sh
  else
    echo "paid storage economics mode: disabled"
    echo "reason: no economics config found or RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS=0"
    env \
      WEB3_PAID_STORAGE_USE_ECONOMICS=0 \
      bash scripts/web3_paid_storage_live_smoke.sh
  fi
}

need_file "QUICKCHAIN.MD"
need_file "scripts/web3_nextlevel_creator_loop_green_gate.sh"
need_file "scripts/web3_paid_site_visit_smoke.sh"
need_file "scripts/web3_accounting_rewarder_wallet_smoke.sh"
need_file "scripts/web3_paid_storage_live_smoke.sh"

echo "RustyOnions QuickChain preflight gate"
echo "root:                              ${ROOT}"
echo "run live smokes:                   ${RON_PREFLIGHT_RUN_LIVE}"
echo "run heavy crate tests:             ${RON_PREFLIGHT_RUN_HEAVY}"
echo "run clippy:                        ${RON_PREFLIGHT_RUN_CLIPPY}"
echo "paid storage economics mode input: ${RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS}"
echo "economics config candidate:        $(resolve_roc_economics_path || true)"
echo "economics action:                  ${RON_PREFLIGHT_ROC_ECONOMICS_ACTION}"

scan_for_premature_quickchain_code

step "fmt core value-plane crates" cargo fmt -p svc-wallet -p ron-ledger -p ron-accounting -p svc-rewarder -p omnigate -p svc-gateway

if [[ "${RON_PREFLIGHT_RUN_CLIPPY}" == "1" ]]; then
  step "clippy value-plane crates" cargo clippy \
    -p svc-wallet \
    -p ron-ledger \
    -p ron-accounting \
    -p svc-rewarder \
    -p omnigate \
    -p svc-gateway \
    --all-targets -- -D warnings
else
  skip "clippy value-plane crates" "set RON_PREFLIGHT_RUN_CLIPPY=1 for strict local preflight"
fi

step "paid creator loop contract gate" bash scripts/web3_nextlevel_creator_loop_green_gate.sh

if [[ "${RON_PREFLIGHT_RUN_HEAVY}" == "1" ]]; then
  step "ron-ledger tests" cargo test -p ron-ledger --all-targets
  step "svc-wallet tests" cargo test -p svc-wallet --all-targets
  step "ron-accounting tests" cargo test -p ron-accounting --all-targets
  step "svc-rewarder tests" cargo test -p svc-rewarder --all-targets
else
  skip "heavy crate test sweep" "set RON_PREFLIGHT_RUN_HEAVY=1 to run ledger/wallet/accounting/rewarder full test suites"
fi

if [[ "${RON_PREFLIGHT_RUN_LIVE}" == "1" ]]; then
  step "live paid site_visit smoke" bash scripts/web3_paid_site_visit_smoke.sh
  step "live paid storage smoke" run_paid_storage_live_smoke
  step "live accounting→rewarder→wallet smoke" bash scripts/web3_accounting_rewarder_wallet_smoke.sh
else
  skip "live value-plane smokes" "start the local dev stack, then set RON_PREFLIGHT_RUN_LIVE=1"
fi

cat <<SUMMARY

QuickChain preflight passed.

Meaning:
- QuickChain remains locked.
- No chain/ROX/Solana bridge implementation was introduced by this gate.
- Internal ROC proof surfaces are the active prerequisite:
  wallet receipts
  ledger roots/replay tests
  paid site_visit transfer proof
  paid storage hold/capture/release proof
  accounting snapshots
  rewarder payout planning
  wallet-committed payout receipts

Paid-storage economics mode:
- auto prefers configs/roc-economics.dev.toml
- auto falls back to configs/roc-economics.toml when present
- auto falls back to WEB3_PAID_STORAGE_USE_ECONOMICS=0 if neither config exists
- force economics with RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS=1
- force legacy with RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS=0
- override path with RON_PREFLIGHT_ROC_ECONOMICS_PATH=/absolute/path/to/config.toml

Next allowed work:
- strengthen internal replay/accounting/rewarder gates
- add paid content-view quote/pay
- keep QUICKCHAIN.MD as blueprint only

SUMMARY