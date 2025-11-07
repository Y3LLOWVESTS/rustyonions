#!/usr/bin/env bash
set -euo pipefail

CRATE="ron-kms"
RUSTFLAGS="${RUSTFLAGS:-"-C target-cpu=native"}"
export RUSTFLAGS
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-4}"

# Gates (microseconds). Tweak as needed.
PAR32_MAX_US="${PAR32_MAX_US:-800}"   # 0.80 ms
PAR64_MAX_US="${PAR64_MAX_US:-1000}"  # 1.00 ms

say(){ printf "\n\033[1;36m[perf-gate]\033[0m %s\n" "$*"; }
die(){ printf "\033[1;31m[perf-gate] FAIL:\033[0m %s\n" "$*"; exit 1; }
ok(){  printf "\033[1;32m[perf-gate] OK:\033[0m %s\n" "$*"; }

run_and_parse() {
  local size="$1"
  local out
  out="$(cargo bench -p "$CRATE" --features "dalek-batch,parallel-batch" \
         --bench batch_verify -- \
         --measurement-time 6 --sample-size 35 2>/dev/null | tee /dev/stderr)"

  # Find the "verify_batch/<size>  time: [a b c]" line and extract median (b)
  local line median unit
  line="$(echo "$out" | awk "/^verify_batch\/$size[[:space:]]+time:/{print; exit}")"
  median="$(echo "$line" | sed -E 's/.*time:\s*\[[^ ]+\s+([^ ]+)\s+[^ ]+\].*/\1/')"
  unit="$(echo "$median" | sed -E 's/[0-9.\-]//g')"
  median="$(echo "$median" | sed -E 's/[^0-9.]//g')"

  # Convert to microseconds
  local us
  case "$unit" in
    "us"|"µs") us="$(awk -v v="$median" 'BEGIN{printf "%.0f", v}')" ;;
    "ms")     us="$(awk -v v="$median" 'BEGIN{printf "%.0f", v*1000}')" ;;
    "s")      us="$(awk -v v="$median" 'BEGIN{printf "%.0f", v*1000000}')" ;;
    *)        die "Unknown time unit for size $size: '$unit' (line: $line)";;
  esac
  echo "$us"
}

say "Running gates for parallel multiscalar..."
us32="$(run_and_parse 32)"
us64="$(run_and_parse 64)"

[[ "$us32" -le "$PAR32_MAX_US" ]] || die "verify_batch/32: ${us32}µs > ${PAR32_MAX_US}µs"
[[ "$us64" -le "$PAR64_MAX_US" ]] || die "verify_batch/64: ${us64}µs > ${PAR64_MAX_US}µs"

ok "verify_batch/32=${us32}µs (<= ${PAR32_MAX_US}µs)"
ok "verify_batch/64=${us64}µs (<= ${PAR64_MAX_US}µs)"
