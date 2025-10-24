#!/usr/bin/env bash
# ron-bus smoke: format (write), lint, test, example, benches (perf-biased)
# Not battery-friendly: uses native CPU flags and longer measurement windows.
# Emits artifacts under artifacts/ron-bus/<timestamp>.

set -euo pipefail

# Resolve repo root (even if invoked from this scripts/ dir)
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

CRATE="ron-bus"
DATE_TAG="$(date +%Y%m%d-%H%M%S)"
BASELINE="smoke-${DATE_TAG}"
ART_DIR="${REPO_ROOT}/artifacts/ron-bus/${DATE_TAG}"
LOG_DIR="${ART_DIR}/logs"
mkdir -p "${LOG_DIR}"

echo "== ron-bus smoke start @ ${DATE_TAG} =="

echo "== format (write) =="
cargo fmt -p "${CRATE}"

echo "== format (verify) =="
cargo fmt -p "${CRATE}" -- --check

echo "== clippy =="
cargo clippy -p "${CRATE}" --all-targets -- -D warnings | tee "${LOG_DIR}/clippy.txt"

echo "== tests =="
cargo test -p "${CRATE}" | tee "${LOG_DIR}/tests.txt"

echo "== example: publish_smoke =="
cargo run -p "${CRATE}" --example publish_smoke | tee "${LOG_DIR}/publish_smoke.txt"

# Performance-oriented flags: native CPU (not battery-friendly)
export RUSTFLAGS="-C target-cpu=native"

echo "== benches: throughput =="
cargo bench -p "${CRATE}" --bench throughput -- \
  --warm-up-time 2 --measurement-time 5 --sample-size 30 \
  | tee "${LOG_DIR}/bench_throughput.txt"

echo "== benches: latency =="
cargo bench -p "${CRATE}" --bench latency -- \
  --warm-up-time 2 --measurement-time 5 --sample-size 30 \
  | tee "${LOG_DIR}/bench_latency.txt"

echo "== benches: A/B compare =="
cargo bench -p "${CRATE}" --bench ab_compare -- \
  --warm-up-time 3 --measurement-time 10 --sample-size 50 \
  --save-baseline "${BASELINE}" \
  | tee "${LOG_DIR}/bench_ab_compare.txt"

# Copy full Criterion reports
if [ -d "target/criterion" ]; then
  mkdir -p "${ART_DIR}"
  cp -R target/criterion "${ART_DIR}/criterion"
fi

# Quick-and-dirty summary from Criterion files (heuristic)
SUMMARY="${ART_DIR}/ab_compare_summary.txt"
echo "== summarizing ab_compare results =="

AB_DIRS="$(find target/criterion -maxdepth 2 -type d -name 'ab_*' 2>/dev/null || true)"
{
  echo "ron-bus A/B summary (${DATE_TAG})"
  echo "baseline: ${BASELINE}"
  echo
  if [ -n "${AB_DIRS}" ]; then
    # Pull function names + 'time: [' lines from report text files
    grep -RHEn "ab_.*|time:\s+\[" ${AB_DIRS} 2>/dev/null \
      | sed -E 's|^.*/||' \
      | sed -E 's|.*/report.txt:||' || true
  else
    echo "No ab_* groups found under target/criterion."
  fi
} > "${SUMMARY}"

echo "== artifacts =="
echo "Logs:       ${LOG_DIR}"
echo "Criterion:  ${ART_DIR}/criterion"
echo "A/B summary:${SUMMARY}"

echo "== ron-bus smoke done =="
