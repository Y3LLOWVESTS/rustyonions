#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   STRICT=0 ROUNDS=3 MEASTIME=10 bash crates/ron-kernel/scripts/perf_gate.sh
#   STRICT=1 ROUNDS=7 MEASTIME=20 bash crates/ron-kernel/scripts/perf_gate.sh   # CI/server

ROUNDS="${ROUNDS:-3}"
MEASTIME="${MEASTIME:-10}"

BASE_CLASSIC="core-2025-10-22"
BASE_BATCHED="core-2025-10-22-batched"

if ! command -v critcmp >/dev/null 2>&1; then
  echo "critcmp not found. Installing..."
  cargo install critcmp
fi

# Thresholds
one_sub_regress_pct=10     # batched one_sub may not be >10% slower than classic
fanout_gain_pct=8          # local default
lagged_gain_pct=8          # local default
if [[ "${STRICT:-0}" == "1" ]]; then
  fanout_gain_pct=15
  lagged_gain_pct=8
fi

ns_from_token() {
  local tok="$1" time unit
  time="$(echo "$tok" | sed -E 's/±.*//')"
  unit="$(echo "$tok" | sed -E 's/.*(ns|µs|ms)$/\1/')"
  case "$unit" in
    ns) awk -v v="$time" 'BEGIN{printf "%.6f", v}' ;;
    µs) awk -v v="$time" 'BEGIN{printf "%.6f", v*1000}' ;;
    ms) awk -v v="$time" 'BEGIN{printf "%.6f", v*1000000}' ;;
  esac
}

extract_time_ns() {
  local group="$1" cmp_out="$2" which="$3"
  # which: "first" for classic time on that row, "last" for batched time on that row
  local line tok
  line="$(echo "$cmp_out" | grep -F "$group" | tail -n1 || true)"
  [[ -z "$line" ]] && { echo "NaN"; return; }
  if [[ "$which" == "first" ]]; then
    tok="$(echo "$line" | grep -Eo '[0-9]+(\.[0-9]+)?±[0-9\.]+(ns|µs|ms)' | head -n1)"
  else
    tok="$(echo "$line" | grep -Eo '[0-9]+(\.[0-9]+)?±[0-9\.]+(ns|µs|ms)' | tail -n1)"
  fi
  ns_from_token "$tok"
}

median() { awk '{a[NR]=$1} END{ n=NR; asort(a); if(n%2) printf "%.6f", a[(n+1)/2]; else printf "%.6f", (a[n/2]+a[n/2+1])/2 }'; }
pct_improve(){ awk -v a="$1" -v b="$2" 'BEGIN{ printf "%.2f", (a-b)/a*100 }'; }

classic_fanout_ns=(); batched_fanout_ns=()
classic_lagged_ns=(); batched_lagged_ns=()
classic_one_ns=();    batched_one_ns=()

for r in $(seq 1 "$ROUNDS"); do
  echo "=== Round $r/$ROUNDS: classic (measurement ${MEASTIME}s)"
  cargo bench -p ron-kernel --bench bus_publish \
    -- --warm-up-time 3 --measurement-time "${MEASTIME}" --save-baseline "${BASE_CLASSIC}"

  echo "=== Round $r/$ROUNDS: batched (measurement ${MEASTIME}s)"
  cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish \
    -- --warm-up-time 3 --measurement-time "${MEASTIME}" --save-baseline "${BASE_BATCHED}"

  CMP_OUT="$(critcmp "${BASE_CLASSIC}" "${BASE_BATCHED}")"
  echo "$CMP_OUT"

  cf="$(extract_time_ns 'classic_fanout/burst256_fanout4_cap2048' "$CMP_OUT" "first")"
  bf="$(extract_time_ns 'batched_fanout/burst256_fanout4_cap2048' "$CMP_OUT" "last")"
  cl="$(extract_time_ns 'classic_lagged_fanout/burst256_fanout4_cap1' "$CMP_OUT" "first")"
  bl="$(extract_time_ns 'batched_lagged_fanout/burst256_fanout4_cap1' "$CMP_OUT" "last")"
  co="$(extract_time_ns 'one_subscriber/publish()' "$CMP_OUT" "first")"
  bo="$(extract_time_ns 'one_subscriber/publish()' "$CMP_OUT" "last")"

  classic_fanout_ns+=("$cf");   batched_fanout_ns+=("$bf")
  classic_lagged_ns+=("$cl");   batched_lagged_ns+=("$bl")
  classic_one_ns+=("$co");      batched_one_ns+=("$bo")
done

mf="$(printf "%s\n" "${classic_fanout_ns[@]}" | median)"
nbf="$(printf "%s\n" "${batched_fanout_ns[@]}" | median)"
ml="$(printf "%s\n" "${classic_lagged_ns[@]}" | median)"
nbl="$(printf "%s\n" "${batched_lagged_ns[@]}" | median)"
mo="$(printf "%s\n" "${classic_one_ns[@]}" | median)"
nbo="$(printf "%s\n" "${batched_one_ns[@]}" | median)"

echo "---- MEDIANS (ns) ----"
echo "classic fanout : $mf"
echo "batched fanout : $nbf"
echo "classic lagged : $ml"
echo "batched lagged : $nbl"
echo "classic one_sub: $mo"
echo "batched one_sub: $nbo"

fanout_delta="$(pct_improve "$mf" "$nbf")"
lagged_delta="$(pct_improve "$ml" "$nbl")"
one_delta="$(pct_improve "$mo" "$nbo")"

echo "improve fanout : ${fanout_delta}%"
echo "improve lagged : ${lagged_delta}%"
echo "improve one_sub: ${one_delta}%"

fail=0
awk -v d="$one_delta"  -v thr="-$one_sub_regress_pct" 'BEGIN{ if (d < thr) exit 1 }' || { echo "FAIL: one_sub regression exceeds '${one_sub_regress_pct}%'"; fail=1; }
awk -v d="$fanout_delta" -v thr="$fanout_gain_pct"     'BEGIN{ if (d < thr) exit 1 }' || { echo "FAIL: fanout gain < '${fanout_gain_pct}%'"; fail=1; }
awk -v d="$lagged_delta" -v thr="$lagged_gain_pct"     'BEGIN{ if (d < thr) exit 1 }' || { echo "FAIL: lagged gain < '${lagged_gain_pct}%'"; fail=1; }

if [[ "$fail" -ne 0 ]]; then
  echo "PERF GATE FAILED"
  exit 1
fi
echo "PERF GATE PASSED"
