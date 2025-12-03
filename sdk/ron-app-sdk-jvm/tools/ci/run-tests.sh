#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

echo "[ron-app-sdk-jvm] running JVM SDK tests..."
echo "  ROOT=$ROOT"
echo

echo "[ron-app-sdk-jvm] ::projects (sanity)"
./gradlew projects

echo "[ron-app-sdk-jvm] ::core:test"
./gradlew :core:test

echo "[ron-app-sdk-jvm] ::kotlin:test"
./gradlew :kotlin:test

echo "[ron-app-sdk-jvm] ::facets:test"
./gradlew :facets:test

echo "[ron-app-sdk-jvm] ::interop-tests:test"
./gradlew :interop-tests:test

echo "[ron-app-sdk-jvm] ::examples:java-cli:build"
./gradlew :examples:java-cli:build

echo
echo "[ron-app-sdk-jvm] all JVM SDK tests passed."
