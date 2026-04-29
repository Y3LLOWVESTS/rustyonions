#!/usr/bin/env bash
# RO:WHAT — Live WEB3 earning-side smoke: ron-accounting vector → svc-rewarder HTTP emit → svc-wallet issue.
# RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Proves rewards commit through wallet and replay does not double issue.
# RO:INTERACTS — ron-accounting vector CLI, svc-rewarder HTTP routes, svc-wallet HTTP routes.
# RO:INVARIANTS — no direct ledger mutation by accounting/rewarder; integer amounts only; idempotent wallet replay.
# RO:METRICS — checks wallet metrics for issue ops and idempotency replays.
# RO:CONFIG — SVC_WALLET_ADDR, SVC_REWARDER_ADDR, WEB3_SMOKE_DIR, WEB3_SMOKE_KEEP_LOGS.
# RO:SECURITY — dev-only Authorization: Bearer dev; no secrets printed.
# RO:TEST — run with: bash scripts/web3_accounting_rewarder_wallet_smoke.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SVC_WALLET_ADDR="${SVC_WALLET_ADDR:-127.0.0.1:18088}"
SVC_REWARDER_ADDR="${SVC_REWARDER_ADDR:-127.0.0.1:18090}"
WALLET_URL="http://${SVC_WALLET_ADDR}"
REWARDER_URL="http://${SVC_REWARDER_ADDR}"

WEB3_SMOKE_DIR="${WEB3_SMOKE_DIR:-target/web3-rewarder-smoke}"
KEEP_LOGS="${WEB3_SMOKE_KEEP_LOGS:-0}"

POLICY_ID="${POLICY_ID:-policy:v1}"
POLICY_HASH="${POLICY_HASH:-b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb}"

mkdir -p "$WEB3_SMOKE_DIR"

WALLET_LOG="$WEB3_SMOKE_DIR/svc-wallet.log"
REWARDER_LOG="$WEB3_SMOKE_DIR/svc-rewarder.log"
VECTOR_JSON="$WEB3_SMOKE_DIR/accounting-vector.json"
COMPUTE_JSON="$WEB3_SMOKE_DIR/rewarder-compute.json"
MANIFEST_JSON="$WEB3_SMOKE_DIR/rewarder-manifest.json"
MANIFEST_REPLAY_JSON="$WEB3_SMOKE_DIR/rewarder-manifest-replay.json"
SETTLEMENT_JSON="$WEB3_SMOKE_DIR/rewarder-settlement.json"
EMIT_JSON="$WEB3_SMOKE_DIR/rewarder-emit.json"
EMIT_REPLAY_JSON="$WEB3_SMOKE_DIR/rewarder-emit-replay.json"

pids=()

cleanup() {
  local status=$?

  for pid in "${pids[@]:-}"; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done

  for pid in "${pids[@]:-}"; do
    wait "$pid" >/dev/null 2>&1 || true
  done

  if [[ "$status" -ne 0 || "$KEEP_LOGS" == "1" ]]; then
    echo "Logs:"
    echo "  $WALLET_LOG"
    echo "  $REWARDER_LOG"
  fi

  exit "$status"
}
trap cleanup EXIT

need_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "missing required command: $cmd" >&2
    exit 1
  fi
}

need_cmd cargo
need_cmd curl
need_cmd jq
need_cmd cmp

print_log_tail() {
  local name="$1"
  local file="$2"

  echo ""
  echo "----- $name log tail -----"
  if [[ -f "$file" ]]; then
    tail -n 80 "$file" || true
  else
    echo "log file does not exist: $file"
  fi
  echo "----- end $name log tail -----"
  echo ""
}

wait_http_ok() {
  local url="$1"
  local name="$2"
  local log_file="$3"
  local deadline=$((SECONDS + 90))

  while (( SECONDS < deadline )); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi

    for pid in "${pids[@]:-}"; do
      if ! kill -0 "$pid" >/dev/null 2>&1; then
        echo "$name process exited before readiness succeeded" >&2
        print_log_tail "$name" "$log_file"
        return 1
      fi
    done

    sleep 0.25
  done

  echo "timed out waiting for $name at $url" >&2
  print_log_tail "$name" "$log_file"
  return 1
}

json_get() {
  local file="$1"
  local filter="$2"
  jq -er "$filter" "$file"
}

assert_json_eq() {
  local file="$1"
  local filter="$2"
  local expected="$3"
  local actual
  actual="$(json_get "$file" "$filter")"
  if [[ "$actual" != "$expected" ]]; then
    echo "assertion failed for $file $filter: expected '$expected', got '$actual'" >&2
    exit 1
  fi
}

post_json_file() {
  local url="$1"
  local body_file="$2"
  local out_file="$3"

  curl -fsS \
    -X POST "$url" \
    -H "Authorization: Bearer dev" \
    -H "Content-Type: application/json" \
    --data-binary "@$body_file" \
    -o "$out_file"
}

post_empty() {
  local url="$1"
  local out_file="$2"

  curl -fsS \
    -X POST "$url" \
    -H "Authorization: Bearer dev" \
    -o "$out_file"
}

wallet_balance() {
  local account="$1"
  local out_file="$2"

  curl -fsS \
    "$WALLET_URL/v1/balance?account=${account}&asset=roc" \
    -H "Authorization: Bearer dev" \
    -o "$out_file"
}

echo "Building live smoke binaries"
cargo build -q -p svc-wallet
cargo build -q -p svc-rewarder
cargo build -q -p ron-accounting --bin ron_accounting_reward_snapshot_vector

WALLET_BIN="$ROOT_DIR/target/debug/svc-wallet"
REWARDER_BIN="$ROOT_DIR/target/debug/svc-rewarder"
ACCOUNTING_VECTOR_BIN="$ROOT_DIR/target/debug/ron_accounting_reward_snapshot_vector"

if [[ ! -x "$WALLET_BIN" ]]; then
  echo "missing built wallet binary: $WALLET_BIN" >&2
  exit 1
fi

if [[ ! -x "$REWARDER_BIN" ]]; then
  echo "missing built rewarder binary: $REWARDER_BIN" >&2
  exit 1
fi

if [[ ! -x "$ACCOUNTING_VECTOR_BIN" ]]; then
  echo "missing built accounting vector binary: $ACCOUNTING_VECTOR_BIN" >&2
  exit 1
fi

echo "Starting svc-wallet on $SVC_WALLET_ADDR"
RUST_LOG="${RUST_LOG:-info}" \
SVC_WALLET_ADDR="$SVC_WALLET_ADDR" \
  "$WALLET_BIN" >"$WALLET_LOG" 2>&1 &
pids+=("$!")

echo "Starting svc-rewarder on $SVC_REWARDER_ADDR"
RUST_LOG="${RUST_LOG:-info}" \
SVC_REWARDER_BIND_ADDR="$SVC_REWARDER_ADDR" \
SVC_REWARDER_WALLET_BASE_URL="$WALLET_URL" \
SVC_REWARDER_WALLET_ISSUE_PATH="/v1/issue" \
  "$REWARDER_BIN" >"$REWARDER_LOG" 2>&1 &
pids+=("$!")

wait_http_ok "$WALLET_URL/readyz" "svc-wallet" "$WALLET_LOG"
wait_http_ok "$REWARDER_URL/readyz" "svc-rewarder" "$REWARDER_LOG"

echo "Generating ron-accounting reward snapshot vector"
"$ACCOUNTING_VECTOR_BIN" --json > "$VECTOR_JSON"

EPOCH_ID="$(json_get "$VECTOR_JSON" '.epoch_id')"
SNAPSHOT_CID="$(json_get "$VECTOR_JSON" '.snapshot_cid')"

echo "Accounting vector:"
echo "  epoch_id=$EPOCH_ID"
echo "  snapshot_cid=$SNAPSHOT_CID"

jq -n \
  --arg inputs_cid "$SNAPSHOT_CID" \
  --arg policy_id "$POLICY_ID" \
  --arg policy_hash "$POLICY_HASH" \
  --slurpfile vector "$VECTOR_JSON" \
  '{
    inputs_cid: $inputs_cid,
    policy_id: $policy_id,
    policy_hash: $policy_hash,
    dry_run: false,
    snapshot: $vector[0].snapshot
  }' > "$COMPUTE_JSON"

echo "Computing reward epoch through svc-rewarder"
post_json_file \
  "$REWARDER_URL/rewarder/epochs/$EPOCH_ID/compute" \
  "$COMPUTE_JSON" \
  "$MANIFEST_JSON"

assert_json_eq "$MANIFEST_JSON" '.epoch_id' "$EPOCH_ID"
assert_json_eq "$MANIFEST_JSON" '.inputs_cid' "$SNAPSHOT_CID"
assert_json_eq "$MANIFEST_JSON" '.status' "ok"
assert_json_eq "$MANIFEST_JSON" '.ledger.result' "accepted"
assert_json_eq "$MANIFEST_JSON" '.totals.pool_minor_units' "1000"
assert_json_eq "$MANIFEST_JSON" '.totals.payout_minor_units' "999"
assert_json_eq "$MANIFEST_JSON" '.totals.residual_minor_units' "1"
assert_json_eq "$MANIFEST_JSON" '.payouts | length' "2"
assert_json_eq "$MANIFEST_JSON" '.payouts[0].account' "acct_a"
assert_json_eq "$MANIFEST_JSON" '.payouts[0].amount_minor_units' "356"
assert_json_eq "$MANIFEST_JSON" '.payouts[1].account' "acct_b"
assert_json_eq "$MANIFEST_JSON" '.payouts[1].amount_minor_units' "643"

echo "Replaying rewarder compute; should not create a different manifest"
post_json_file \
  "$REWARDER_URL/rewarder/epochs/$EPOCH_ID/compute" \
  "$COMPUTE_JSON" \
  "$MANIFEST_REPLAY_JSON"

cmp -s "$MANIFEST_JSON" "$MANIFEST_REPLAY_JSON" || {
  echo "rewarder replay response differs from first manifest" >&2
  exit 1
}

echo "Fetching deterministic wallet settlement batch from svc-rewarder"
curl -fsS \
  "$REWARDER_URL/rewarder/epochs/$EPOCH_ID/settlement" \
  -H "Authorization: Bearer dev" \
  -o "$SETTLEMENT_JSON"

assert_json_eq "$SETTLEMENT_JSON" '.wallet_path' "/v1/issue"
assert_json_eq "$SETTLEMENT_JSON" '.total_minor_units' "999"
assert_json_eq "$SETTLEMENT_JSON" '.requests | length' "2"
assert_json_eq "$SETTLEMENT_JSON" '.requests[0].to' "acct_a"
assert_json_eq "$SETTLEMENT_JSON" '.requests[0].amount_minor' "356"
assert_json_eq "$SETTLEMENT_JSON" '.requests[1].to' "acct_b"
assert_json_eq "$SETTLEMENT_JSON" '.requests[1].amount_minor' "643"

echo "Emitting reward payouts from svc-rewarder to svc-wallet"
post_empty \
  "$REWARDER_URL/rewarder/epochs/$EPOCH_ID/emit" \
  "$EMIT_JSON"

assert_json_eq "$EMIT_JSON" '.result' "accepted"
assert_json_eq "$EMIT_JSON" '.batch.total_minor_units' "999"
assert_json_eq "$EMIT_JSON" '.receipts | length' "2"
assert_json_eq "$EMIT_JSON" '.receipts[0].op' "issue"
assert_json_eq "$EMIT_JSON" '.receipts[0].asset' "roc"
assert_json_eq "$EMIT_JSON" '.receipts[1].op' "issue"
assert_json_eq "$EMIT_JSON" '.receipts[1].asset' "roc"

wallet_balance "acct_a" "$WEB3_SMOKE_DIR/acct_a.balance.json"
wallet_balance "acct_b" "$WEB3_SMOKE_DIR/acct_b.balance.json"

assert_json_eq "$WEB3_SMOKE_DIR/acct_a.balance.json" '.account' "acct_a"
assert_json_eq "$WEB3_SMOKE_DIR/acct_a.balance.json" '.asset' "roc"
assert_json_eq "$WEB3_SMOKE_DIR/acct_a.balance.json" '.amount_minor' "356"

assert_json_eq "$WEB3_SMOKE_DIR/acct_b.balance.json" '.account' "acct_b"
assert_json_eq "$WEB3_SMOKE_DIR/acct_b.balance.json" '.asset' "roc"
assert_json_eq "$WEB3_SMOKE_DIR/acct_b.balance.json" '.amount_minor' "643"

echo "Replaying svc-rewarder emit; wallet balances must not double issue"
post_empty \
  "$REWARDER_URL/rewarder/epochs/$EPOCH_ID/emit" \
  "$EMIT_REPLAY_JSON"

cmp -s "$EMIT_JSON" "$EMIT_REPLAY_JSON" || {
  echo "rewarder emit replay response differs from first emit" >&2
  exit 1
}

wallet_balance "acct_a" "$WEB3_SMOKE_DIR/acct_a.after_replay.balance.json"
wallet_balance "acct_b" "$WEB3_SMOKE_DIR/acct_b.after_replay.balance.json"

assert_json_eq "$WEB3_SMOKE_DIR/acct_a.after_replay.balance.json" '.amount_minor' "356"
assert_json_eq "$WEB3_SMOKE_DIR/acct_b.after_replay.balance.json" '.amount_minor' "643"

curl -fsS "$WALLET_URL/metrics" -o "$WEB3_SMOKE_DIR/wallet.metrics.txt"

grep -q 'wallet_ops_total{op="issue"} 2' "$WEB3_SMOKE_DIR/wallet.metrics.txt" || {
  echo "wallet metrics did not show exactly two issue operations" >&2
  print_log_tail "svc-wallet" "$WALLET_LOG"
  exit 1
}

grep -q 'wallet_idempotency_replays_total 2' "$WEB3_SMOKE_DIR/wallet.metrics.txt" || {
  echo "wallet metrics did not show two idempotency replays" >&2
  print_log_tail "svc-wallet" "$WALLET_LOG"
  exit 1
}

echo "WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green"
echo "  acct_a=356 ROC minor units"
echo "  acct_b=643 ROC minor units"
echo "  payout_total=999 ROC minor units"
echo "  replay=no double issue"