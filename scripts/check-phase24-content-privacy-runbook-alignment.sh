#!/usr/bin/env bash
set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1
  pwd
)"

GUIDE="docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md"

STORAGE="$ROOT/crates/svc-storage/docs/RUNBOOK.MD"
DHT="$ROOT/crates/svc-dht/docs/RUNBOOK.MD"
CHECKLIST="$ROOT/docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md"

fail() {
  printf 'Phase 24D content/privacy runbook check failed: %s\n' "$*" >&2
  exit 1
}

for file in "$STORAGE" "$DHT" "$CHECKLIST"; do
  [[ -f "$file" ]] || fail "missing ${file#$ROOT/}"
done

for file in "$STORAGE" "$DHT"; do
  count="$(
    grep -Foc \
      "## BUILD_PLAN_Z Phase 24 private-beta content and privacy posture" \
      "$file" || true
  )"

  [[ "$count" -eq 1 ]] ||
    fail "${file#$ROOT/} must contain exactly one content/privacy posture"

  grep -Fq "$GUIDE" "$file" ||
    fail "${file#$ROOT/} does not reference the authoritative guide"
done

storage_truth=(
  "Serving requires complete BLAKE3 validation"
  "global deny defeats local allow"
  "owner tombstone defeats local allow"
  "An oversized object rejection must not evict already-valid cached content."
  "Persistence approval means the object is eligible for durable treatment."
  "A local prune must not claim:"
  "Moderation and pruning do not mutate wallet or ledger state."
)

for truth in "${storage_truth[@]}"; do
  grep -Fq "$truth" "$STORAGE" ||
    fail "svc-storage runbook is missing: $truth"
done

dht_truth=(
  "crab://node/<node-id>"
  "residential IP address"
  "Expired or stale provider records must not produce fake lookup success."
  "A locally unavailable provider must not become available again merely because"
  "Safe output must not include a hidden raw route"
  "Tor/onion addressing is not the current"
  "CrabLink product contract and does not override the canonical \`crab://\`"
  "CrabLink verification fails"
  "before User Node submission rather than falling back to a direct public"
)

for truth in "${dht_truth[@]}"; do
  grep -Fq "$truth" "$DHT" ||
    fail "svc-dht runbook is missing: $truth"
done

for legacy_scheme in 'relay://' 'onion://' 'service://'; do
  count="$(grep -Foc "$legacy_scheme" "$DHT" || true)"

  [[ "$count" -ge 1 ]] ||
    fail "svc-dht runbook must explicitly reject $legacy_scheme"
done

grep -Fq \
  "[x] storage and DHT runbooks document moderation, persistence, pruning, and provider privacy" \
  "$CHECKLIST" ||
  fail "central readiness checklist does not record content/privacy alignment"

printf '%s\n' \
  "Phase 24D content and provider-privacy runbook alignment passed." \
  "Canonical B3/OAP integrity, moderation precedence, amnesia-first cache," \
  "persistence eligibility, truthful pruning, canonical crab:// identity," \
  "stale-provider rejection, and residential-IP privacy are documented."
