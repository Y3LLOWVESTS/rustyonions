#!/usr/bin/env bash

# RO:WHAT  — Single-command smoke test for the RON-CORE app plane via ron-app-sdk-php.
# RO:WHY   — CI/dev helper to quickly verify that /app/hello responds successfully.
# RO:INTERACTS — examples/hello.php, RON_SDK_* env vars, svc-gateway app plane.
# RO:INVARIANTS —
#   * Never prints secrets (tokens/caps) to stdout/stderr.
#   * Exits non-zero on failure.
#   * Works from repo root or from sdk/ron-app-sdk-php.

set -euo pipefail

# Resolve SDK root directory (script may be invoked from anywhere).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${SDK_ROOT}"

if [ ! -f "vendor/autoload.php" ]; then
  echo "[RON] vendor/autoload.php not found. Did you run 'composer install' in sdk/ron-app-sdk-php?" >&2
  exit 1
fi

# Show minimal context (but never secrets).
GATEWAY="${RON_SDK_GATEWAY_ADDR:-<unset>}"
echo "[RON] Smoke test: sdk=ron-app-sdk-php"
echo "[RON] Using RON_SDK_GATEWAY_ADDR=${GATEWAY}"

# Run the hello example; capture output but let exit code drive success.
set +e
OUTPUT="$(php examples/hello.php 2>&1)"
STATUS=$?
set -e

if [ "${STATUS}" -ne 0 ]; then
  echo "[RON] Smoke test FAILED: hello.php exited with status ${STATUS}" >&2
  echo "[RON] Output:" >&2
  echo "${OUTPUT}" >&2
  exit "${STATUS}"
fi

# Treat SDK-level error output as failure too.
if printf '%s\n' "${OUTPUT}" | grep -qE '^\[RON\] (Problem|Auth error|Network error)'; then
  echo "[RON] Smoke test FAILED: hello.php reported an SDK-level error." >&2
  echo "[RON] Output:" >&2
  echo "${OUTPUT}" >&2
  exit 2
fi

if [ -z "${OUTPUT}" ]; then
  echo "[RON] Smoke test FAILED: hello.php produced no output." >&2
  exit 3
fi

echo "[RON] Smoke test PASSED."
echo "${OUTPUT}"
