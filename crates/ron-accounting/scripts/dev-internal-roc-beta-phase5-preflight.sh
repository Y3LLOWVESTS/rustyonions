#!/usr/bin/env bash
# RO:WHAT — Focused Internal ROC Beta Phase 5 preflight for ron-accounting economics-config labels.
# RO:WHY — Proves accounting labels config schema/version/hash/source only and cannot become balance, receipt, payout, finality, wallet, ledger, unlock, bridge, staking, liquidity, or external-settlement authority.
# RO:INTERACTS — configs/roc-economics.toml, economics_config label DTO, Phase 5 tests, prior Internal ROC regressions.
# RO:INVARIANTS — accounting remains derivative snapshot/report infrastructure; config labels are report-only and label-only.
# RO:METRICS — none.
# RO:CONFIG — reads canonical economics TOML fixture for label extraction.
# RO:SECURITY — bash/cargo-only; no checked-in Python helpers; no secrets, wallet mutation, ledger mutation, or paid unlock authority.
# RO:TEST — bash crates/ron-accounting/scripts/dev-internal-roc-beta-phase5-preflight.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-accounting"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

fail() {
  printf 'ron-accounting internal ROC beta phase5 preflight failed: %s\n' "$*" >&2
  exit 1
}

[ -f configs/roc-economics.toml ] || fail "missing configs/roc-economics.toml"
[ -f "$CRATE_DIR/src/accounting/economics_config.rs" ] || fail "missing economics config label DTO"
[ -f "$CRATE_DIR/tests/internal_roc_beta_phase5_config_label_non_authority.rs" ] || fail "missing Phase 5 config label non-authority test"

python_hits="$(find "$CRATE_DIR" \
  -path "$CRATE_DIR/target" -prune -o \
  -type f \( -name '*.py' -o -name '*.pyi' -o -name '*.pyc' \) -print)"
if [ -n "$python_hits" ]; then
  echo "$python_hits"
  fail "ron-accounting Internal ROC Beta preflight must remain bash/cargo-only"
fi

"$CARGO" fmt -p ron-accounting -- --check
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase3_approved_payout_observation_boundary
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary
"$CARGO" test -p ron-accounting --test internal_roc_beta_phase2_snapshot_cannot_change_replay
"$CARGO" clippy -p ron-accounting --all-targets -- -D warnings

printf '\n== Internal ROC Beta Phase 5 ron-accounting config-label preflight passed ==\n'
printf '== accounting labels config version/hash/source only; no balance/receipt/payout/finality/wallet/ledger authority ==\n'
