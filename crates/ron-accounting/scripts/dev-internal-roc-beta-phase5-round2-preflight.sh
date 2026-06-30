#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 Round 2 preflight for ron-accounting event-class anti-farming gates.
# RO:WHY — Proves accounting isolates analytics_only, metering, proof_eligible, ad_budgeted, and economic_receipt lanes without direct protocol ROC allocation.
# RO:INTERACTS — event_class DTOs, usage-event DTOs, Phase 5 config labels, prior Phase 3 event-class boundaries.
# RO:INVARIANTS — accounting remains derivative/report infrastructure; no balance truth, receipt truth, wallet/ledger mutation, paid unlock, bridge, staking, liquidity, or external settlement.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets.
# RO:TEST — bash crates/ron-accounting/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-accounting"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'ron-accounting internal ROC beta phase5 round2 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f "$CRATE_DIR/src/accounting/event_class.rs" ] || fail "missing event-class anti-farming DTO"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_event_class_antifarming.rs" ] || fail "missing Phase 5 Round 2 event-class test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "ron-accounting Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p ron-accounting -- --check
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase5_event_class_antifarming
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority
"$CARGO" clippy -p ron-accounting --all-targets -- -D warnings

printf '\n== Internal ROC Beta Phase 5 Round 2 ron-accounting event-class anti-farming preflight passed ==\n'
printf '== analytics_only/metering/proof_eligible/ad_budgeted/economic_receipt lanes stay isolated and non-authoritative ==\n'
