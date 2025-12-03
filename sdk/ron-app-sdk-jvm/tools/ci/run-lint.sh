#!/usr/bin/env bash
# RO:WHAT — JVM SDK lint/style runner for ron-app-sdk-jvm.
# RO:WHY  — Placeholder for future Java/Kotlin lint wiring that does NOT
#           block CI or local dev while the SDK is still stabilizing.
# RO:INVARIANTS —
#   - Never fails the build today (no-op placeholder).
#   - Clearly documents future intent (checkstyle + ktlint + detekt).
#   - Safe to invoke from any working directory inside the repo.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$ROOT"

echo "[ron-app-sdk-jvm] lint checks are not wired yet."
echo
echo "  Planned future behavior:"
echo "    1) Java checkstyle for :core using config/checkstyle.xml"
echo "    2) ktlint for :kotlin and :facets via config/ktlint.gradle.kts"
echo "    3) detekt for Kotlin modules via config/detekt.yml"
echo
echo "  For now, this script is a no-op placeholder and exits successfully."
echo

exit 0
