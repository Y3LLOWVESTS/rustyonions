#!/usr/bin/env bash
set -euo pipefail

# ron-kms Beta verification script
# - Formats, lints, tests (incl. soft-seal), compiles feature combos
# - Runs quick latency benches (serial + parallel)
# - Enforces perf gates if perf_gate.sh exists
#
# Env knobs:
#   RUSTFLAGS           (default: "-C target-cpu=native")
#   RAYON_NUM_THREADS   (default: "4")
#   BENCH_TIME          (default: "4")   # seconds per bench group
#   BENCH_SAMPLES       (default: "25")  # Criterion samples
#   SKIP_BENCH=1        (skip benches and perf gate)
#   VERBOSE=1           (set -x)

[[ "${VERBOSE:-}" == "1" ]] && set -x

CRATE="ron-kms"
RUSTFLAGS="${RUSTFLAGS:-"-C target-cpu=native"}"
export RUSTFLAGS
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-4}"
BENCH_TIME="${BENCH_TIME:-4}"
BENCH_SAMPLES="${BENCH_SAMPLES:-25}"

say() { printf "\n\033[1;36m[beta-check]\033[0m %s\n" "$*"; }
ok()  { printf "\033[1;32mOK\033[0m %s\n" "${1:-}"; }
die() { printf "\033[1;31mFAIL\033[0m %s\n" "$*" ; exit 1; }

say "1) fmt + clippy + unit tests"
cargo fmt -p "$CRATE"
cargo clippy -p "$CRATE" --no-deps -- -D warnings
cargo test  -p "$CRATE"
ok "core tests"

say "2) feature: soft-seal (tests)"
cargo test -p "$CRATE" --features soft-seal --test soft_seal_roundtrip
ok "soft-seal tests"

say "3) feature: with-metrics (compile sanity)"
cargo check -p "$CRATE" --features with-metrics
ok "metrics compiles"

say "4) feature combos (compile sanity)"
cargo check -p "$CRATE" --features dalek-batch
cargo check -p "$CRATE" --features "dalek-batch,parallel-batch"
# Optional: ring fast lane (compile only; if not linked on your box, comment this out)
cargo check -p "$CRATE" --features fast || true
ok "feature combos compile"

if [[ "${SKIP_BENCH:-0}" != "1" ]]; then
  say "5) benches: batch verify (serial multiscalar)"
  cargo bench -p "$CRATE" --features dalek-batch \
    --bench batch_verify -- \
    --measurement-time "$BENCH_TIME" --sample-size "$BENCH_SAMPLES"

  say "6) benches: batch verify (parallel multiscalar)"
  cargo bench -p "$CRATE" --features "dalek-batch,parallel-batch" \
    --bench batch_verify -- \
    --measurement-time "$BENCH_TIME" --sample-size "$BENCH_SAMPLES"

  # Optional throughput smoke (short)
  say "7) benches: throughput smoke (parallel path)"
  cargo bench -p "$CRATE" --features "dalek-batch,parallel-batch" \
    --bench throughput_batch -- \
    --measurement-time "$BENCH_TIME" --sample-size "$BENCH_SAMPLES"

  # Perf gate (if present)
  if [[ -x "crates/ron-kms/scripts/perf_gate.sh" ]]; then
    say "8) perf-gate"
    crates/ron-kms/scripts/perf_gate.sh
  else
    say "8) perf-gate script not found; skipping (expected path: crates/ron-kms/scripts/perf_gate.sh)"
  fi
else
  say "Skipping benches and perf gate (SKIP_BENCH=1)"
fi

say "All checks complete."
