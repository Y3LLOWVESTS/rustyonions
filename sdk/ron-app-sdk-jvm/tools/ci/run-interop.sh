#!/usr/bin/env bash
# RO:WHAT —
#   Interop test runner for ron-app-sdk-jvm.
#
# RO:WHY —
#   Provides a single entry point for running JVM SDK interop tests against
#   a real RON-CORE Micronode/Macronode gateway.
#
# RO:INVARIANTS —
#   - MUST be safe to call even when no gateway is running.
#   - MUST print clear guidance instead of blowing up in CI/local dev.
#   - Does NOT rely on a multi-module Gradle graph; runs interop-tests
#     as a standalone Gradle project via -p interop-tests.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$ROOT"

echo "[ron-app-sdk-jvm] Interop test runner"
echo "  ROOT=$ROOT"

# 1) Ensure we have a gateway address; without it, there is nothing meaningful to test.
if [[ -z "${RON_SDK_GATEWAY_ADDR:-}" ]]; then
  echo "[ron-app-sdk-jvm] RON_SDK_GATEWAY_ADDR is not set."
  echo "  - Skipping interop tests. To run them, export e.g.:"
  echo "      RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090"
  echo "      RON_SDK_INSECURE_HTTP=1"
  exit 0
fi

echo "[ron-app-sdk-jvm] Running interop tests against: ${RON_SDK_GATEWAY_ADDR}"

# 2) Run the interop-tests module as an independent Gradle project.
if [[ $# -gt 0 ]]; then
  ./gradlew -p interop-tests test "$@"
else
  ./gradlew -p interop-tests test
fi
