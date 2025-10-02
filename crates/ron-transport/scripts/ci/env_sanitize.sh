#!/usr/bin/env bash
set -euo pipefail
export AMNESIA=1
export TMPDIR="${TMPDIR:-/tmp}"
echo "[ci] AMNESIA=1 and TMPDIR set for no-disk tests."
