#!/usr/bin/env bash
set -euo pipefail

FLAGS=(--warm-up-time 8 --measurement-time 15 --sample-size 300)

RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms --bench sign_bench -- "${FLAGS[@]}"
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms --bench verify_bench -- "${FLAGS[@]}"
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms --bench batch_verify -- "${FLAGS[@]}"
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms --bench parallel_throughput -- "${FLAGS[@]}"
