#!/usr/bin/env bash
# RO:WHAT — Static docs/scripts checker for the final Internal ROC product-beta readiness aggregate gate.
# RO:WHY — The final sweep must verify parked boundaries compose without adding runtime behavior or reopening completed phases.
# RO:INTERACTS — docs/internal-roc product-beta docs, backend stabilization docs/tests/scripts, aggregate park scripts.
# RO:INVARIANTS — required docs/scripts/tests exist; final docs preserve wallet/ledger truth and bridge docs-only boundary.
# RO:SECURITY — rejects missing proof surfaces, missing forbidden-scope language, git steps, and workspace clippy in the final aggregate gate.
# RO:TEST — bash scripts/check-internal-roc-product-beta-readiness.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

need_file() {
  local rel="$1"
  [[ -f "$rel" ]] || fail "missing required file: $rel"
}

need_text() {
  local rel="$1"
  local needle="$2"

  if [[ ! -f "$rel" ]]; then
    fail "cannot inspect missing file: $rel"
    return
  fi

  grep -Fq -- "$needle" "$rel" || fail "missing marker in $rel: $needle"
}

need_absent() {
  local rel="$1"
  local needle="$2"

  if [[ ! -f "$rel" ]]; then
    fail "cannot inspect missing file for absence check: $rel"
    return
  fi

  if grep -Fq -- "$needle" "$rel"; then
    fail "forbidden marker present in $rel: $needle"
  fi
}

echo "== Internal ROC Product Beta Readiness checker: required files =="

required_files=(
  "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md"
  "docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md"
  "docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md"

  "scripts/check-internal-roc-product-beta-readiness.sh"
  "scripts/dev-internal-roc-product-beta-readiness-park.sh"

  "scripts/dev-internal-roc-stabilization-proto-ledger-park.sh"
  "scripts/dev-internal-roc-stabilization-wallet-accounting-park.sh"
  "scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh"
  "scripts/dev-internal-roc-stabilization-gateway-omnigate-paid-route-park.sh"
  "scripts/dev-internal-roc-stabilization-storage-index-park.sh"

  "crates/ron-proto/docs/internal-roc-stabilization-receipt-dto.md"
  "crates/ron-proto/tests/internal_roc_stabilization_receipt_dto_boundary.rs"
  "crates/ron-proto/scripts/dev-internal-roc-stabilization-receipt-dto-preflight.sh"

  "crates/ron-ledger/docs/internal-roc-stabilization-replay-truth.md"
  "crates/ron-ledger/tests/internal_roc_stabilization_replay_truth_boundary.rs"
  "crates/ron-ledger/scripts/dev-internal-roc-stabilization-replay-truth-preflight.sh"

  "crates/svc-wallet/docs/internal-roc-stabilization-mutation-frontdoor.md"
  "crates/svc-wallet/tests/internal_roc_stabilization_mutation_frontdoor_boundary.rs"
  "crates/svc-wallet/scripts/dev-internal-roc-stabilization-mutation-frontdoor-preflight.sh"

  "crates/ron-accounting/docs/internal-roc-stabilization-snapshot-non-authority.md"
  "crates/ron-accounting/tests/internal_roc_stabilization_snapshot_non_authority_boundary.rs"
  "crates/ron-accounting/scripts/dev-internal-roc-stabilization-snapshot-non-authority-preflight.sh"

  "crates/svc-rewarder/docs/internal-roc-stabilization-reward-policy-gate.md"
  "crates/svc-rewarder/tests/internal_roc_stabilization_reward_policy_gate_boundary.rs"
  "crates/svc-rewarder/scripts/dev-internal-roc-stabilization-reward-policy-gate-preflight.sh"

  "crates/ron-policy/docs/internal-roc-stabilization-policy-gate-non-authority.md"
  "crates/ron-policy/tests/internal_roc_stabilization_policy_gate_non_authority_boundary.rs"
  "crates/ron-policy/scripts/dev-internal-roc-stabilization-policy-gate-preflight.sh"

  "crates/svc-gateway/docs/internal-roc-stabilization-paid-route.md"
  "crates/svc-gateway/tests/internal_roc_stabilization_paid_route_error_boundary.rs"
  "crates/svc-gateway/scripts/dev-internal-roc-stabilization-paid-route-preflight.sh"

  "crates/omnigate/docs/internal-roc-stabilization-paid-access.md"
  "crates/omnigate/tests/internal_roc_stabilization_paid_access_error_boundary.rs"
  "crates/omnigate/scripts/dev-internal-roc-stabilization-paid-access-preflight.sh"

  "crates/svc-storage/docs/internal-roc-stabilization-b3-artifact-non-authority.md"
  "crates/svc-storage/tests/internal_roc_stabilization_b3_artifact_non_authority_boundary.rs"
  "crates/svc-storage/scripts/dev-internal-roc-stabilization-b3-artifact-preflight.sh"

  "crates/svc-index/docs/internal-roc-stabilization-pointer-non-authority.md"
  "crates/svc-index/tests/internal_roc_stabilization_pointer_non_authority_boundary.rs"
  "crates/svc-index/scripts/dev-internal-roc-stabilization-pointer-preflight.sh"
)

for rel in "${required_files[@]}"; do
  need_file "$rel"
done

echo "== Internal ROC Product Beta Readiness checker: shell syntax =="

shell_scripts=(
  "scripts/check-internal-roc-product-beta-readiness.sh"
  "scripts/dev-internal-roc-product-beta-readiness-park.sh"
  "scripts/dev-internal-roc-stabilization-proto-ledger-park.sh"
  "scripts/dev-internal-roc-stabilization-wallet-accounting-park.sh"
  "scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh"
  "scripts/dev-internal-roc-stabilization-gateway-omnigate-paid-route-park.sh"
  "scripts/dev-internal-roc-stabilization-storage-index-park.sh"
)

for rel in "${shell_scripts[@]}"; do
  if [[ -f "$rel" ]]; then
    bash -n "$rel" || fail "bash syntax failed: $rel"
  fi
done

echo "== Internal ROC Product Beta Readiness checker: final docs markers =="

need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "final aggregate value-loop product beta readiness gate"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "svc-wallet"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "ron-ledger receipt/balance truth"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "CrabLink display-only receipt/access UX"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "backend-derived balance refresh"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "ron-accounting snapshots"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "svc-rewarder capped planning"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "ron-policy gating"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "No fake balances"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md" "No runtime"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md" "Internal ROC product-beta readiness gate is parked."
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md" "Bridge remains future/docs-only."
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md" "svc-wallet remains the only mutation front-door"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md" "CrabLink direct ledger mutation blocked"
need_text "docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md" "Bridge remains docs / threat-model / decision-gate only"

echo "== Internal ROC Product Beta Readiness checker: aggregate script markers =="

need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p ron-proto --test internal_roc_stabilization_receipt_dto_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_stabilization_replay_truth_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p svc-wallet --test internal_roc_stabilization_mutation_frontdoor_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p ron-accounting --test internal_roc_stabilization_snapshot_non_authority_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p svc-rewarder --test internal_roc_stabilization_reward_policy_gate_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p ron-policy --test internal_roc_stabilization_policy_gate_non_authority_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p svc-gateway --test internal_roc_stabilization_paid_route_error_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p omnigate --test internal_roc_stabilization_paid_access_error_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p svc-storage --test internal_roc_stabilization_b3_artifact_non_authority_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo test -p svc-index --test internal_roc_stabilization_pointer_non_authority_boundary"
need_text "scripts/dev-internal-roc-product-beta-readiness-park.sh" "dev-internal-roc-stabilization-tauri-park.sh"

need_absent "scripts/dev-internal-roc-product-beta-readiness-park.sh" "cargo clippy --workspace"
need_absent "scripts/dev-internal-roc-product-beta-readiness-park.sh" "git add"
need_absent "scripts/dev-internal-roc-product-beta-readiness-park.sh" "git commit"
need_absent "scripts/dev-internal-roc-product-beta-readiness-park.sh" "git push"

if (( failures > 0 )); then
  echo "Internal ROC Product Beta Readiness checker failed: failures=$failures" >&2
  exit 1
fi

echo "== Internal ROC Product Beta Readiness checker passed =="
echo "== required docs/scripts/tests exist; final docs preserve wallet/ledger truth; no workspace clippy or git steps in aggregate gate =="
