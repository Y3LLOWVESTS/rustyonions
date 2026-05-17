#!/usr/bin/env bash
# RO:WHAT — Local NEXT_LEVEL green gate for paid b3 asset content_view quote/pay.
# RO:WHY — Locks the next visitor→creator ROC loop after paid named-site visits.
# RO:INTERACTS — omnigate, svc-gateway, svc-wallet through test dummies.
# RO:INVARIANTS — wallet remains mutation front-door; gateway proxy-only; no fake balances; no silent spend.
# RO:METRICS — route/service metrics are covered by child service tests; this script runs contract tests.
# RO:CONFIG — RON_RUN_CLIPPY.
# RO:SECURITY — local/dev tests only; no production secrets.
# RO:TEST — bash scripts/web3_nextlevel_content_view_green_gate.sh.

set -euo pipefail
IFS=$'\n\t'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RON_RUN_CLIPPY="${RON_RUN_CLIPPY:-0}"

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

need_file "crates/omnigate/src/routes/v1/content_view.rs"
need_file "crates/omnigate/tests/content_view.rs"
need_file "crates/svc-gateway/tests/content_view_routes_proxy.rs"

echo "RustyOnions NEXT_LEVEL paid content_view green gate"
echo "root:       ${ROOT}"
echo "run clippy: ${RON_RUN_CLIPPY}"

step "cargo fmt for content_view crates" cargo fmt -p omnigate -p svc-gateway

if [[ "${RON_RUN_CLIPPY}" == "1" ]]; then
  step "clippy omnigate content_view target only" \
    cargo clippy -p omnigate --test content_view --no-deps -- -D warnings

  step "clippy svc-gateway content_view proxy target only" \
    cargo clippy -p svc-gateway --test content_view_routes_proxy --no-deps -- -D warnings
else
  skip "clippy" "set RON_RUN_CLIPPY=1 for the stricter local content_view gate"
fi

step "omnigate content_view contract tests" cargo test -p omnigate --test content_view
step "svc-gateway content_view proxy tests" cargo test -p svc-gateway --test content_view_routes_proxy

cat <<SUMMARY

RustyOnions NEXT_LEVEL paid content_view gate passed.

Locked proofs:
- Omnigate b3 asset quote/pay contract
- Manifest-derived payout recipient
- Wallet transfer front-door
- Nonce recovery
- Wallet receipt return
- Gateway proxy-only route contract

Next allowed work:
- wire CrabLink article/post/comment asset pages to quote/pay before full content unlock
- keep gateway proxy-only
- keep QuickChain locked
- do not touch ron-kernel for this product-layer gate

SUMMARY