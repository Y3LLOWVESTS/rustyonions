#!/usr/bin/env bash
# RO:WHAT — Final local parking gate for svc-wallet QuickChain Phase-0/preflight.
# RO:WHY — Parks svc-wallet only after exhaustive preflight, docs, and workspace-level checks pass.
# RO:INTERACTS — svc-wallet preflight script, crate-local docs, cargo workspace.
# RO:INVARIANTS — svc-wallet remains wallet mutation front-door; no roots, checkpoints, validators, settlement, external anchors, bridges, staking, liquidity, or live chain authority.
# RO:METRICS — none.
# RO:CONFIG — no runtime config changes.
# RO:SECURITY — no external settlement, bridge, staking, liquidity, or public-chain authority.
# RO:TEST — crates/svc-wallet/scripts/dev-quickchain-park.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

echo "== svc-wallet QuickChain parking gate =="
echo "workspace: $ROOT_DIR"
echo

echo "== crate preflight =="
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
echo

echo "== docs presence and boundary markers =="
DOC="crates/svc-wallet/docs/quickchain-preflight.md"
test -s "$DOC"

grep -q "svc-wallet is the ROC wallet mutation front-door" "$DOC"
grep -q "QuickChain is future settlement infrastructure" "$DOC"
grep -q "ron-ledger remains economic truth" "$DOC"
grep -q "no fake balances" "$DOC"
grep -q "no fake receipts" "$DOC"
grep -q "no silent spend" "$DOC"
grep -q "no roots" "$DOC"
grep -q "no checkpoints" "$DOC"
grep -q "no validators" "$DOC"
grep -q "no settlement" "$DOC"
grep -q "no external anchors" "$DOC"
grep -q "no bridges" "$DOC"
grep -q "idempotency_key = retry key only" "$DOC"
grep -q "operation_id = backend-assigned durable ledger-operation identity" "$DOC"

echo "crate-local QuickChain runbook exists and preserves wallet boundaries"
echo

echo "== docs regression test =="
cargo test -p svc-wallet --test quickchain_preflight_docs
echo

echo "== feature-gated docs regression test =="
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_docs
echo

echo "== package check with QuickChain preflight feature =="
cargo check -p svc-wallet --features quickchain-preflight
echo

echo "== workspace check =="
cargo check --workspace
echo

echo "== svc-wallet QuickChain parking gate passed =="
