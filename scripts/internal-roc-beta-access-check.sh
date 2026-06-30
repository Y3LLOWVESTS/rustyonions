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
    printf 'missing required Phase 6 access proof file: %s\n' "$1" >&2
    exit 1
  fi
}

printf '== Internal ROC Beta Phase 6 storage/index/gateway/omnigate access proof target ==\n'

require_file crates/svc-storage/tests/internal_roc_beta_paid_content_artifact_boundary.rs
require_file crates/svc-index/tests/internal_roc_beta_paid_content_pointer_boundary.rs
require_file crates/svc-gateway/tests/internal_roc_beta_paid_content_route_boundary.rs
require_file crates/svc-gateway/tests/internal_roc_beta_phase2_replay_visibility_boundary.rs
require_file crates/svc-gateway/tests/internal_roc_beta_phase4_confirmation_failure_boundary.rs
require_file crates/omnigate/tests/internal_roc_beta_paid_content_access_boundary.rs
require_file crates/omnigate/tests/internal_roc_beta_phase2_replay_visibility_boundary.rs
require_file crates/omnigate/tests/internal_roc_beta_phase4_confirmation_failure_boundary.rs

run "$CARGO" fmt -p svc-storage -- --check
run "$CARGO" fmt -p svc-index -- --check
run "$CARGO" fmt -p svc-gateway -- --check
run "$CARGO" fmt -p omnigate -- --check

run "$CARGO" test -p svc-storage --test internal_roc_beta_paid_content_artifact_boundary
run "$CARGO" test -p svc-index --test internal_roc_beta_paid_content_pointer_boundary
run "$CARGO" test -p svc-gateway --test internal_roc_beta_paid_content_route_boundary
run "$CARGO" test -p svc-gateway --test internal_roc_beta_phase2_replay_visibility_boundary
run "$CARGO" test -p svc-gateway --test internal_roc_beta_phase4_confirmation_failure_boundary
run "$CARGO" test -p omnigate --test internal_roc_beta_paid_content_access_boundary
run "$CARGO" test -p omnigate --test internal_roc_beta_phase2_replay_visibility_boundary
run "$CARGO" test -p omnigate --test internal_roc_beta_phase4_confirmation_failure_boundary

printf '\n== Internal ROC Beta Phase 6 access proof target passed ==\n'
printf '== storage/index/gateway/omnigate remain non-mutating boundaries with backend-derived paid access truth ==\n'
