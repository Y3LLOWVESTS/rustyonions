
#!/usr/bin/env bash
# RO:WHAT — Runs the WEB3_2 product proof green gate.
# RO:WHY — Batch 10C reproducible backend acceptance before Chrome extension work.
# RO:INTERACTS — omnigate tests, svc-gateway tests, product stack smoke scripts.
# RO:INVARIANTS — does not run paid image upload; mutation-enabled site create is explicit and deterministic.
# RO:METRICS — prints pass/fail gate sections; service logs are emitted by web3_product_stack_smoke.sh.
# RO:CONFIG — RUN_FULL_GATE, RUN_STACK_SMOKE, RUN_SITE_CREATE_SMOKE.
# RO:SECURITY — local product stack smoke disables omnigate policy only in generated temporary smoke config.
# RO:TEST — manual: scripts/web3_product_green_gate.sh.

set -euo pipefail

RUN_FULL_GATE="${RUN_FULL_GATE:-1}"
RUN_STACK_SMOKE="${RUN_STACK_SMOKE:-1}"
RUN_SITE_CREATE_SMOKE="${RUN_SITE_CREATE_SMOKE:-1}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 127
  }
}

run_step() {
  local label="$1"
  shift

  echo
  echo "==> ${label}"
  "$@"
}

need_cmd cargo
need_cmd chmod

chmod +x scripts/web3_product_stack_smoke.sh

if [ "$RUN_FULL_GATE" = "1" ]; then
  run_step "cargo fmt" cargo fmt

  run_step "omnigate clippy" \
    cargo clippy -p omnigate --all-targets --no-deps -- -D warnings

  run_step "omnigate site_launch test" \
    cargo test -p omnigate --test site_launch

  run_step "omnigate all-targets tests" \
    cargo test -p omnigate --all-targets

  run_step "svc-gateway clippy" \
    cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings

  run_step "svc-gateway product route proxy test" \
    cargo test -p svc-gateway --test product_routes_proxy

  run_step "svc-gateway all-targets tests" \
    cargo test -p svc-gateway --all-targets
else
  echo "skip: cargo/test gate because RUN_FULL_GATE=${RUN_FULL_GATE}"
fi

if [ "$RUN_STACK_SMOKE" = "1" ]; then
  run_step "safe WEB3 product stack smoke" \
    scripts/web3_product_stack_smoke.sh
else
  echo "skip: safe stack smoke because RUN_STACK_SMOKE=${RUN_STACK_SMOKE}"
fi

if [ "$RUN_SITE_CREATE_SMOKE" = "1" ]; then
  run_step "mutation-enabled site create/resolve stack smoke" \
    env RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
else
  echo "skip: site create smoke because RUN_SITE_CREATE_SMOKE=${RUN_SITE_CREATE_SMOKE}"
fi

echo
echo "WEB3_2 product green gate passed"