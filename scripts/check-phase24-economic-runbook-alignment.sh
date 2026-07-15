#!/usr/bin/env bash
set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1
  pwd
)"

GUIDE="docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md"

REGISTRY="$ROOT/crates/svc-registry/docs/RUNBOOK.MD"
REWARDER="$ROOT/crates/svc-rewarder/docs/RUNBOOK.MD"
WALLET="$ROOT/crates/svc-wallet/docs/RUNBOOK.MD"
LEDGER="$ROOT/crates/ron-ledger/docs/RUNBOOK.MD"
CHECKLIST="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md"

fail() {
  printf 'Phase 24C economic runbook check failed: %s\n' "$*" >&2
  exit 1
}

for file in \
  "$REGISTRY" \
  "$REWARDER" \
  "$WALLET" \
  "$LEDGER" \
  "$CHECKLIST"
do
  [[ -f "$file" ]] || fail "missing ${file#$ROOT/}"
done

for file in "$REGISTRY" "$REWARDER" "$WALLET" "$LEDGER"; do
  count="$(
    grep -Foc \
      "## BUILD_PLAN_Z Phase 24 private-beta economic posture" \
      "$file" || true
  )"

  [[ "$count" -eq 1 ]] ||
    fail "${file#$ROOT/} must contain exactly one economic posture"

  grep -Fq "$GUIDE" "$file" ||
    fail "${file#$ROOT/} does not reference the authoritative guide"
done

registry_truth=(
  "service_node_id"
  "reward_recipient_account_id"
  "A rejected replacement rotation must not:"
  "There is no permanent founder-approved or manually trusted production-node"
  "many Sybil identities cannot gain control through node count alone"
)

for truth in "${registry_truth[@]}"; do
  grep -Fq "$truth" "$REGISTRY" ||
    fail "svc-registry runbook is missing: $truth"
done

rewarder_truth=(
  "\`svc-rewarder\` does not directly mutate \`ron-ledger\`."
  "Older wording below that describes direct ledger-intent egress is superseded"
  "A reward plan is not a payout."
  "new reward computation is rejected"
  "wallet emission is rejected before egress"
  "Raw engagement does not directly allocate protocol ROC."
)

for truth in "${rewarder_truth[@]}"; do
  grep -Fq "$truth" "$REWARDER" ||
    fail "svc-rewarder runbook is missing: $truth"
done

wallet_truth=(
  "\`svc-wallet\` is the only approved Internal ROC mutation front-door."
  "A single Service Node cannot prepare or execute a payout."
  "The same idempotency key with a different request body must conflict."
  "a second issued-supply mutation"
  "CrabLink may display confirmed ROC only after durable accepted wallet and"
)

for truth in "${wallet_truth[@]}"; do
  grep -Fq "$truth" "$WALLET" ||
    fail "svc-wallet runbook is missing: $truth"
done

ledger_truth=(
  "\`ron-ledger\` is the durable Internal ROC economic truth."
  "Runtime economic mutation must arrive through the approved \`svc-wallet\`"
  "Replay order comes from accepted ledger history."
  "tampered approved-payout receipt reference"
  "Confirmed ROC exists only after an accepted durable ledger receipt."
  "a public blockchain"
)

for truth in "${ledger_truth[@]}"; do
  grep -Fq "$truth" "$LEDGER" ||
    fail "ron-ledger runbook is missing: $truth"
done

grep -Fq \
  "[x] registry, rewarder, wallet, and ledger runbooks document the real private-beta economic path" \
  "$CHECKLIST" ||
  fail "central readiness checklist does not record economic alignment"

printf '%s\n' \
  "Phase 24C economic runbook alignment check passed." \
  "Registry binding, deterministic reward planning, wallet execution," \
  "ledger replay/conservation, receipt honesty, and confirmed-ROC truth" \
  "now follow one documented private-beta path."
