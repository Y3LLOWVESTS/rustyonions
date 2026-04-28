#!/usr/bin/env bash
# RO:WHAT — Send a policy-reload signal to a running svc-wallet process.
# RO:WHY — Ops/GOV; Concerns: RES/GOV. Keeps policy reload workflow explicit.
# RO:INTERACTS — local process table, svc-wallet binary.
# RO:INVARIANTS — no magic sleeps; no stale svc-wallet2 process name; harmless if service is absent.
# RO:METRICS — future reload success/failure counters can be checked after signal.
# RO:CONFIG — none.
# RO:SECURITY — no secrets echoed.
# RO:TEST — manual ops smoke.

set -euo pipefail

PID="$(pgrep -f 'target/.*/svc-wallet|svc-wallet' || true)"

if [[ -n "$PID" ]]; then
  kill -HUP "$PID"
  echo "Sent SIGHUP to svc-wallet ($PID)"
else
  echo "svc-wallet not running"
fi