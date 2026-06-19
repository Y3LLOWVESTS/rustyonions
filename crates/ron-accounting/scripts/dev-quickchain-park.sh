#!/usr/bin/env bash
# RO:WHAT — Final local parking gate for ron-accounting QuickChain Phase-0/preflight.
# RO:WHY — Parks ron-accounting only after exhaustive preflight, docs regression, workflow hygiene, and workspace check pass.
# RO:INTERACTS — ron-accounting preflight script, docs, cargo workspace.
# RO:INVARIANTS — accounting remains derivative metering/snapshot infrastructure; no balance truth, wallet/ledger mutation, roots, checkpoints, validators, settlement, anchors, bridges, staking, liquidity, or pruning.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — no fake balances, fake receipts, silent spend, external settlement, bridges, staking, liquidity, or public-chain authority.
# RO:TEST — crates/ron-accounting/scripts/dev-quickchain-park.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/ron-accounting"
CARGO="${CARGO:-cargo}"

cd "$ROOT_DIR"

tmp_refs="$(mktemp)"
trap 'rm -f "$tmp_refs"' EXIT

echo "== ron-accounting QuickChain parking gate =="
echo "workspace: $ROOT_DIR"
echo

echo "== crate preflight =="
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
echo

echo "== docs regression test =="
"$CARGO" test -p ron-accounting --test quickchain_preflight_docs
echo

echo "== workflow package-name hygiene =="
if [ -d "$CRATE_DIR/.github/workflows" ] && grep -R "ron-accounting2" "$CRATE_DIR/.github/workflows" >"$tmp_refs" 2>/dev/null; then
  cat "$tmp_refs"
  echo "stale ron-accounting2 workflow package references remain"
  exit 1
fi
echo "workflow package names are clean"
echo

echo "== docs presence and boundary markers =="
DOC="$CRATE_DIR/docs/quickchain-preflight.md"
test -s "$DOC"

grep -q "Accounting is not balance truth" "$DOC"
grep -q "Handoff to svc-rewarder" "$DOC"
grep -q "Raw engagement must never directly mint, allocate, transfer, or mutate protocol ROC" "$DOC"
grep -q "A reward snapshot CID is an artifact hash" "$DOC"
grep -q "They do not make accounting a QuickChain root producer" "$DOC"
grep -q "no roots" "$DOC"
grep -q "no checkpoints" "$DOC"
grep -q "no validators" "$DOC"
grep -q "no settlement" "$DOC"
grep -q "no anchors" "$DOC"
grep -q "no external anchors" "$DOC"
grep -q "no bridges" "$DOC"
grep -q "no staking" "$DOC"
grep -q "no liquidity" "$DOC"
grep -q "no Solana/ROX/external settlement path" "$DOC"
grep -q "no fake balances" "$DOC"
grep -q "no fake receipts" "$DOC"
grep -q "no silent spend" "$DOC"
grep -q "no wallet mutation" "$DOC"
grep -q "no ledger mutation" "$DOC"

echo "crate-local QuickChain runbook exists and preserves accounting non-authority boundaries"
echo

echo "== workspace check =="
"$CARGO" check --workspace
echo

echo "== ron-accounting QuickChain parking gate passed =="
