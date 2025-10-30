#!/usr/bin/env bash
set -euo pipefail

cargo build -p svc-index --release

INDEX_BIND="${INDEX_BIND:-127.0.0.1:5304}"
RUST_LOG="${RUST_LOG:-warn}"

target/release/svc-index >/tmp/svc-index.bench.release.log 2>&1 &
PID=$!
trap 'kill $PID >/dev/null 2>&1 || true' EXIT

# wait for server
for i in $(seq 1 200); do
  code=$(curl -s -o /dev/null -w "%{http_code}" "http://$INDEX_BIND/healthz" || true)
  [ "$code" = "200" ] && break
  sleep 0.05
done

warmup() {
  D=${1:-5}
  end=$(( $(date +%s) + D ))
  while [ "$(date +%s)" -lt "$end" ]; do curl -s -o /dev/null "http://$INDEX_BIND/healthz" || true; done
}

run_wrk() {
  echo "wrk /healthz"
  wrk -t4 -c128 -d30s "http://$INDEX_BIND/healthz"
  echo
  echo "wrk /version"
  wrk -t4 -c128 -d30s "http://$INDEX_BIND/version"
  echo
  echo "wrk /metrics"
  wrk -t4 -c64  -d30s "http://$INDEX_BIND/metrics"
}

run_hey() {
  echo "hey /healthz"
  hey -z 30s -c 128 "http://$INDEX_BIND/healthz"
  echo
  echo "hey /version"
  hey -z 30s -c 128 "http://$INDEX_BIND/version"
  echo
  echo "hey /metrics"
  hey -z 30s -c 64  "http://$INDEX_BIND/metrics"
}

run_pure() {
  one() {
    URL="$1"; CONC="$2"; DUR="$3"
    end=$(( $(date +%s) + DUR ))
    tmpdir="$(mktemp -d)"
    pids=()
    for w in $(seq 1 "$CONC"); do
      (
        cnt=0
        while [ "$(date +%s)" -lt "$end" ]; do curl -s -o /dev/null "$URL" || true; cnt=$((cnt+1)); done
        echo "$cnt" > "$tmpdir/$w.count"
      ) & pids+=("$!")
    done
    for p in "${pids[@]}"; do wait "$p"; done
    total=0
    for f in "$tmpdir"/*.count; do [ -f "$f" ] && total=$(( total + $(cat "$f") )); done
    rm -rf "$tmpdir"
    rps=$(awk "BEGIN { printf \"%.1f\", $total/$DUR }")
    echo "purebash url=$URL conc=$CONC dur=${DUR}s total_reqs=$total rps=$rps"
  }
  one "http://$INDEX_BIND/healthz" 128 30
  echo
  one "http://$INDEX_BIND/version" 128 30
  echo
  one "http://$INDEX_BIND/metrics" 64  30
}

warmup 5

if command -v wrk >/dev/null 2>&1; then
  run_wrk
elif command -v hey >/dev/null 2>&1; then
  run_hey
else
  run_pure
fi
