#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

echo "== ron-accounting QuickChain parking gate =="
echo "workspace: $ROOT_DIR"
echo

echo "== crate preflight =="
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
echo

echo "== workflow package-name hygiene =="
if grep -R "ron-accounting2" crates/ron-accounting/.github/workflows >/tmp/ron_accounting_stale_workflow_refs.txt 2>/dev/null; then
  cat /tmp/ron_accounting_stale_workflow_refs.txt
  echo "stale ron-accounting2 workflow package references remain"
  exit 1
fi
echo "workflow package names are clean"
echo

echo "== docs presence =="
test -s crates/ron-accounting/docs/quickchain-preflight.md
grep -q "Accounting is not balance truth" crates/ron-accounting/docs/quickchain-preflight.md
grep -q "Handoff to svc-rewarder" crates/ron-accounting/docs/quickchain-preflight.md
echo "crate-local QuickChain runbook exists"
echo

echo "== workspace check =="
cargo check --workspace
echo

echo "== ron-accounting QuickChain parking gate passed =="
