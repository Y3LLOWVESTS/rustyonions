#!/usr/bin/env bash
# RO:WHAT — Local NEXT_LEVEL green gate for the paid visitor→creator ROC loop.
# RO:WHY — Locks the proven site_visit creator economy path before paid content-view and QuickChain preflight work.
# RO:INTERACTS — omnigate, svc-gateway, svc-wallet, ron-ledger, scripts/web3_paid_site_visit_smoke.sh.
# RO:INVARIANTS — wallet remains mutation front-door; gateway proxy-only; no fake balances; no silent spend.
# RO:METRICS — child services expose /metrics; this script prints txid/receipt/root via the live smoke.
# RO:CONFIG — RON_RUN_LIVE_SITE_VISIT, RON_RUN_CLIPPY, SITE_NAME, PAYER_ACCOUNT, RECIPIENT_ACCOUNT.
# RO:SECURITY — dev bearer/local endpoints only; no production secrets.
# RO:TEST — bash scripts/web3_nextlevel_creator_loop_green_gate.sh.

set -euo pipefail
IFS=$'\n\t'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RON_RUN_CLIPPY="${RON_RUN_CLIPPY:-0}"
RON_RUN_LIVE_SITE_VISIT="${RON_RUN_LIVE_SITE_VISIT:-0}"

SITE_NAME="${SITE_NAME:-ron7}"
PAYER_ACCOUNT="${PAYER_ACCOUNT:-acct_visitor_b}"
RECIPIENT_ACCOUNT="${RECIPIENT_ACCOUNT:-acct_dev}"

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

need_file "scripts/web3_paid_site_visit_smoke.sh"
need_file "crates/omnigate/tests/site_visit.rs"
need_file "crates/svc-gateway/tests/site_visit_routes_proxy.rs"
need_file "crates/svc-wallet/tests/i_14_site_visit_receipt_replay.rs"

echo "RustyOnions NEXT_LEVEL paid creator loop green gate"
echo "root:                  ${ROOT}"
echo "site_name:             ${SITE_NAME}"
echo "payer_account:         ${PAYER_ACCOUNT}"
echo "recipient_account:     ${RECIPIENT_ACCOUNT}"
echo "run clippy:            ${RON_RUN_CLIPPY}"
echo "run live site_visit:   ${RON_RUN_LIVE_SITE_VISIT}"

step "cargo fmt for paid creator loop crates" cargo fmt -p omnigate -p svc-gateway -p svc-wallet

if [[ "${RON_RUN_CLIPPY}" == "1" ]]; then
  step "clippy omnigate" cargo clippy -p omnigate --all-targets -- -D warnings
  step "clippy svc-gateway" cargo clippy -p svc-gateway --all-targets -- -D warnings
  step "clippy svc-wallet" cargo clippy -p svc-wallet --all-targets -- -D warnings
else
  skip "clippy" "set RON_RUN_CLIPPY=1 for the stricter local gate"
fi

step "omnigate site_visit contract tests" cargo test -p omnigate --test site_visit
step "svc-gateway site_visit proxy tests" cargo test -p svc-gateway --test site_visit_routes_proxy
step "svc-wallet site_visit receipt replay tests" cargo test -p svc-wallet --test i_14_site_visit_receipt_replay

if [[ "${RON_RUN_LIVE_SITE_VISIT}" == "1" ]]; then
  step "live paid site_visit smoke" env \
    SITE_NAME="${SITE_NAME}" \
    PAYER_ACCOUNT="${PAYER_ACCOUNT}" \
    RECIPIENT_ACCOUNT="${RECIPIENT_ACCOUNT}" \
    bash scripts/web3_paid_site_visit_smoke.sh
else
  skip "live paid site_visit smoke" "start the dev stack, then set RON_RUN_LIVE_SITE_VISIT=1"
fi

cat <<SUMMARY

RustyOnions NEXT_LEVEL paid creator loop gate passed.

Locked proofs:
- Omnigate quote/pay contract
- Gateway proxy-only route contract
- Wallet-level site_visit transfer receipt replay contract

Optional live proof:
- run with RON_RUN_LIVE_SITE_VISIT=1 after the local stack is running

SUMMARY