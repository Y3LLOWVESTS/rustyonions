#!/usr/bin/env bash
set -euo pipefail

cargo build -p svc-index

INDEX_BIND="${INDEX_BIND:-127.0.0.1:5304}"
RUST_LOG="${RUST_LOG:-warn}"

target/debug/svc-index >/tmp/svc-index.bench.log 2>&1 &
PID=$!
trap 'kill $PID >/dev/null 2>&1 || true' EXIT

# Wait for service
for i in $(seq 1 120); do
  code=$(curl -s -o /dev/null -w "%{http_code}" "http://$INDEX_BIND/healthz" || true)
  [ "$code" = "200" ] && break
  sleep 0.05
done

bench_wrk() {
  wrk -t4 -c64 -d15s "http://$INDEX_BIND/healthz"
  echo
  wrk -t4 -c64 -d15s "http://$INDEX_BIND/version"
  echo
  wrk -t4 -c32 -d15s "http://$INDEX_BIND/metrics"
}

bench_hey() {
  hey -z 15s -c 64 "http://$INDEX_BIND/healthz"
  echo
  hey -z 15s -c 64 "http://$INDEX_BIND/version"
  echo
  hey -z 15s -c 32 "http://$INDEX_BIND/metrics"
}

bench_pure() {
  one() {
    URL="$1"; CONC="$2"; DUR="$3"
    end=$(( $(date +%s) + DUR ))
    tmpdir="$(mktemp -d)"
    pids=()
    for w in $(seq 1 "$CONC"); do
      (
        cnt=0
        while [ "$(date +%s)" -lt "$end" ]; do
          curl -s -o /dev/null "$URL" || true
          cnt=$((cnt+1))
        done
        echo "$cnt" > "$tmpdir/$w.count"
      ) &
      pids+=("$!")
    done
    for p in "${pids[@]}"; do wait "$p"; done
    total=0
    for f in "$tmpdir"/*.count; do
      [ -f "$f" ] || continue
      n=$(cat "$f")
      total=$((total + n))
    done
    rm -rf "$tmpdir"
    rps=$(awk "BEGIN { printf \"%.1f\", $total/$DUR }")
    echo "purebash url=$URL conc=$CONC dur=${DUR}s total_reqs=$total rps=$rps"
  }
  one "http://$INDEX_BIND/healthz" 64 15
  echo
  one "http://$INDEX_BIND/version" 64 15
  echo
  one "http://$INDEX_BIND/metrics" 32 15
}

if command -v wrk >/dev/null 2>&1; then
  bench_wrk
elif command -v hey >/dev/null 2>&1; then
  bench_hey
else
  bench_pure
fi
