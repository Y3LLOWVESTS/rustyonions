#!/usr/bin/env bash
# RO:WHAT — Run svc-wallet locally with dev-safe defaults.
# RO:WHY — Ops/DX; Concerns: RES/DX. Gives a repeatable local smoke path.
# RO:INTERACTS — cargo, svc-wallet binary, /healthz, /readyz, /metrics.
# RO:INVARIANTS — no secrets echoed; amnesia defaults stay on; no svc-wallet2 drift.
# RO:METRICS — service exposes wallet_* on /metrics after startup.
# RO:CONFIG — uses SVC_WALLET_ADDR and RUST_LOG.
# RO:SECURITY — dummy dev bearer only in caller-side curl examples, not here.
# RO:TEST — manual smoke plus CI build.

set -euo pipefail

export RUST_LOG="${RUST_LOG:-svc_wallet=info,tower_http=info}"
export SVC_WALLET_ADDR="${SVC_WALLET_ADDR:-127.0.0.1:8088}"

cargo run -p svc-wallet