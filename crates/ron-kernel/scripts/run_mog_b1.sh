#!/usr/bin/env bash
# RO:WHAT — Run SoA micro benches and tests (B1).
# RO:WHY  — Repeatable ritual for quick signal before Bus wiring.
# RO:INTERACTS — benches/bus_soa.rs; tests/soa_smoke.rs; Cargo features.
# RO:INVARIANTS — non-destructive; no feature bleed into other benches.

set -euo pipefail

echo "[MOG B1] build+test (feature=bus_soa)"
cargo test -p ron-kernel --features bus_soa -- tests:: # narrow run
cargo test -p ron-kernel --features bus_soa

echo "[MOG B1] benches (feature=bus_soa)"
cargo bench -p ron-kernel --features bus_soa --bench bus_soa
