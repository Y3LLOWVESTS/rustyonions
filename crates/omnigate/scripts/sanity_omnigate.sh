#!/usr/bin/env bash
set -euo pipefail

API_ADDR="${API_ADDR:-127.0.0.1:5305}"
METRICS_ADDR="${METRICS_ADDR:-127.0.0.1:9605}"
BIN_PKG="omnigate"
RUST_LOG_LEVEL="${RUST_LOG_LEVEL:-info}"
TRACE_LOG_COMPONENT="${TRACE_LOG_COMPONENT:-omnigate=trace}"
CARGO="${CARGO:-cargo}"
CONFIG_PATH="${CONFIG_PATH:-crates/omnigate/configs/omnigate.toml}"   # default to repo config
SKIP_NET_BURST="${SKIP_NET_BURST:-0}"                                 # set to 1 to skip the 429 burst check

req()  { printf "[sanity] %s\n" "$*"; }
fail() { printf "[sanity][FAIL] %s\n" "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || fail "missing required tool: $1"; }

# --- prerequisites ---
need curl; need awk; need jq; need dd

# --- helpers ---
CHILD_PID=""
stop_child() {
  if [[ -n "${CHILD_PID:-}" ]] && kill -0 "$CHILD_PID" 2>/dev/null; then
    # Try graceful first, then a hard kill if needed
    kill "$CHILD_PID" 2>/dev/null || true
    sleep 0.2
    kill -0 "$CHILD_PID" 2>/dev/null && kill -9 "$CHILD_PID" 2>/dev/null || true
    wait "$CHILD_PID" 2>/dev/null || true
    CHILD_PID=""
  fi
}
trap stop_child EXIT

http_code() {
  local url="$1"
  curl -s -o /dev/null -w "%{http_code}" "$url"
}

wait_200() {
  local url="$1" tries="${2:-80}" delay="${3:-0.125}"
  for ((i=1;i<=tries;i++)); do
    local code; code="$(http_code "$url" || true)"
    [[ "$code" == "200" ]] && return 0
    sleep "$delay"
  done
  return 1
}

assert_http_200() {
  local url="$1"
  local code; code="$(http_code "$url" || true)"
  [[ "$code" == "200" ]] || fail "expected 200: $url (got $code)"
}

assert_json_true() {
  local url="$1" key="$2"
  local val; val="$(curl -s "$url" | jq -r ".${key}")"
  [[ "$val" == "true" ]] || fail "expected ${key}=true at $url (got $val)"
}

metric_value() {
  local metric="$1"
  # Grab the first *unlabeled* sample line:  "<name> <value>"
  curl -s "http://${METRICS_ADDR}/metrics" \
    | awk -v m="$metric" '$1==m{print $2; exit}'
}

assert_metric_eq() {
  local metric="$1" expect="$2"
  local val; val="$(metric_value "$metric")"
  [[ "${val:-}" == "$expect" ]] || fail "metric $metric expected $expect got ${val:-<missing>}"
}

print_metric() {
  local metric="$1"
  curl -s "http://${METRICS_ADDR}/metrics" | awk -v m="$metric" '$1==m{print}'
}

# --- build & tests ---
req "fmt + clippy + build"
$CARGO fmt -p "$BIN_PKG"
$CARGO clippy -p "$BIN_PKG" --no-deps -- -D warnings
$CARGO build -p "$BIN_PKG"

req "unit/integration tests"
$CARGO test -p "$BIN_PKG" --test dto_serialization --test ready_truth --test zk_receipts

# --- run helper ---
run_omnigate() {
  local dev_ready="$1" # 0 or 1
  stop_child
  req "starting omnigate (OMNIGATE_DEV_READY=${dev_ready})"
  req "using --config $CONFIG_PATH"

  OMNIGATE_DEV_READY="$dev_ready" \
  OMNIGATE_AMNESIA=on \
  RUST_LOG="${RUST_LOG_LEVEL},${TRACE_LOG_COMPONENT}" \
  $CARGO run -p "$BIN_PKG" --quiet -- --config "$CONFIG_PATH" &

  CHILD_PID=$!

  req "waiting for /healthz 200"
  wait_200 "http://${API_ADDR}/healthz" || fail "healthz did not become 200"

  req "waiting for /readyz 200"
  wait_200 "http://${API_ADDR}/readyz" || fail "readyz did not become 200"

  req "waiting for /metrics 200"
  wait_200 "http://${METRICS_ADDR}/metrics" || req "WARN: metrics endpoint not yet 200 (continuing best-effort)"

  # small settle to avoid initial scrape races
  sleep 0.15
}

# --- Phase A: DEV override ON ---
run_omnigate 1

req "check admin plane & v1 routes"
assert_http_200 "http://${API_ADDR}/healthz"
assert_http_200 "http://${API_ADDR}/readyz"
assert_json_true "http://${API_ADDR}/v1/ping" "ok"

req "versionz contains version (git may be null)"
curl -s "http://${API_ADDR}/versionz" | jq -e '.version | length > 0' >/dev/null \
  || fail "/versionz missing 'version'"

req "assert amnesia_mode == 1"
assert_metric_eq amnesia_mode 1

stop_child

# --- Phase B: DEV override OFF (real gates) ---
run_omnigate 0

req "assert readyz payload is true"
assert_json_true "http://${API_ADDR}/readyz" "ready"

req "gate gauges (best-effort if present)"
for m in listeners_bound metrics_bound cfg_loaded ready_state; do
  val="$(metric_value "$m" || true)"
  if [[ -n "$val" && "$val" != "1" ]]; then
    req "WARN: $m present but not 1 (got $val)"
  fi
done

# --- Fair queue cap header check ---
req "fair queue cap header exposes different caps for priorities"
curl -s -i -H 'x-omnigate-priority: interactive' "http://${API_ADDR}/v1/ping" \
  | awk 'BEGIN{IGNORECASE=1}/^x-omnigate-cap:/{print}'
curl -s -i -H 'x-omnigate-priority: bulk'        "http://${API_ADDR}/v1/ping" \
  | awk 'BEGIN{IGNORECASE=1}/^x-omnigate-cap:/{print}'

# --- Global quota burst to try for 429s ---
if [[ "$SKIP_NET_BURST" != "1" ]]; then
  req "burst load to trigger some 429 (best-effort)"
  if command -v xargs >/dev/null 2>&1; then
    seq 1 1200 | xargs -n1 -P64 -I{} curl -s -o /dev/null -w "%{http_code}\n" \
      "http://${API_ADDR}/v1/ping" | sort | uniq -c || true
  else
    req "WARN: xargs not available; skipping burst"
  fi
else
  req "SKIP_NET_BURST=1 — skipping the 429 burst check"
fi

req "check quota exhaust counter (if burst produced 429s)"
print_metric admission_quota_exhausted_total || req "NOTE: admission_quota_exhausted_total not present yet"

# --- Body caps: oversize -> 413 + counter ---
req "body caps oversize 413 + metric"
TMP_BIG="/tmp/omnigate_big.bin"
dd if=/dev/urandom of="$TMP_BIG" bs=1k count=1100 status=none
code="$(curl -s -o /dev/null -w "%{http_code}" -X POST --data-binary @"$TMP_BIG" "http://${API_ADDR}/v1/ping")"
if [[ "$code" != "413" ]]; then
  req "WARN: oversize POST did not return 413 (got $code) — verify route setup for POST on /v1/ping or test an endpoint that accepts body"
fi
print_metric body_reject_total || req "NOTE: body_reject_total not present yet"

# --- Decompression guard: unknown + stacked encodings ---
req "decompress guard rejects: unknown encoding"
code="$(curl -s -o /dev/null -w "%{http_code}" -H 'Content-Encoding: compress' -X POST --data 'abc' "http://${API_ADDR}/v1/ping")" || true
if [[ "$code" != "415" ]]; then
  req "WARN: unknown encoding did not return 415 (got $code) — ensure POST route is guarded by decompress layer"
fi

req "decompress guard rejects: stacked encodings"
code="$(curl -s -o /dev/null -w "%{http_code}" -H 'Content-Encoding: gzip, deflate' -X POST --data 'abc' "http://${API_ADDR}/v1/ping")" || true
if [[ "$code" != "415" ]]; then
  req "WARN: stacked encodings did not return 415 (got $code) — ensure POST route is guarded by decompress layer"
fi
print_metric decompress_reject_total || req "NOTE: decompress_reject_total not present yet"

# --- Ready trip/recover (best-effort) ---
req "ready trip metrics snapshot"
curl -s "http://${METRICS_ADDR}/metrics" | egrep 'ready_trips_total|ready_state_changes_total|ready_error_rate_pct|ready_inflight_current' || true

# Done
stop_child
req "OK — all sanity checks finished"
