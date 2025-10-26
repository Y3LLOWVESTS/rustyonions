#!/usr/bin/env bash
# RO:WHAT — sanitize env for hermetic CI runs
set -euo pipefail
unset RUST_LOG || true
unset RUST_BACKTRACE || true
