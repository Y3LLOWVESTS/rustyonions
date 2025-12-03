#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Scaffold file tree for ron-app-sdk-jvm.
# RO:WHY  — Create all dirs/files so we can start filling in code + Gradle configs.

# Base directory for the SDK (default: sdk/ron-app-sdk-jvm)
BASE_DIR="${1:-sdk/ron-app-sdk-jvm}"

echo "Scaffolding ron-app-sdk-jvm into: $BASE_DIR"

if [ -e "$BASE_DIR" ]; then
  echo "Error: $BASE_DIR already exists. Refusing to overwrite."
  exit 1
fi

mkdir -p "$BASE_DIR"

cd "$BASE_DIR"

# --- Root files --------------------------------------------------------------

touch README.md
touch SDK_IDB.MD
touch SDK_SECURITY.MD
touch SDK_SCHEMA_IDB.MD
touch settings.gradle.kts
touch build.gradle.kts
touch .editorconfig
touch .gitignore

# --- Gradle wrapper / versions ----------------------------------------------

mkdir -p gradle/wrapper
touch gradle/libs.versions.toml
touch gradle/wrapper/gradle-wrapper.jar
touch gradle/wrapper/gradle-wrapper.properties

# --- Config (lint / static analysis) ----------------------------------------

mkdir -p config
touch config/checkstyle.xml
touch config/detekt.yml
touch config/ktlint.gradle.kts

# --- Docs (Mermaid diagrams) -----------------------------------------------

mkdir -p docs
touch docs/arch.mmd
touch docs/sequence.mmd
touch docs/state.mmd

# --- core module ------------------------------------------------------------

mkdir -p core/src/main/java/dev/roncore/sdk
mkdir -p core/src/main/java/dev/roncore/sdk/config
mkdir -p core/src/main/java/dev/roncore/sdk/auth
mkdir -p core/src/main/java/dev/roncore/sdk/http
mkdir -p core/src/main/java/dev/roncore/sdk/internal
mkdir -p core/src/test/java/dev/roncore/sdk

touch core/build.gradle.kts

# Core Java API
touch core/src/main/java/dev/roncore/sdk/RonClient.java
touch core/src/main/java/dev/roncore/sdk/AppResponse.java
touch core/src/main/java/dev/roncore/sdk/RonProblem.java
touch core/src/main/java/dev/roncore/sdk/RonException.java
touch core/src/main/java/dev/roncore/sdk/Page.java
touch core/src/main/java/dev/roncore/sdk/RonSdkVersion.java

# Config
touch core/src/main/java/dev/roncore/sdk/config/RonConfig.java
touch core/src/main/java/dev/roncore/sdk/config/EnvConfigLoader.java

# Auth
touch core/src/main/java/dev/roncore/sdk/auth/TokenProvider.java
touch core/src/main/java/dev/roncore/sdk/auth/StaticTokenProvider.java
touch core/src/main/java/dev/roncore/sdk/auth/RefreshingTokenProvider.java

# HTTP
touch core/src/main/java/dev/roncore/sdk/http/HttpClientAdapter.java
touch core/src/main/java/dev/roncore/sdk/http/OkHttpClientAdapter.java
touch core/src/main/java/dev/roncore/sdk/http/HttpRequestContext.java

# Internal helpers
touch core/src/main/java/dev/roncore/sdk/internal/JsonMapper.java
touch core/src/main/java/dev/roncore/sdk/internal/RetryPolicy.java
touch core/src/main/java/dev/roncore/sdk/internal/BackoffStrategy.java
touch core/src/main/java/dev/roncore/sdk/internal/ResponseSizeLimiter.java

# Core tests
touch core/src/test/java/dev/roncore/sdk/RonClientBasicTest.java
touch core/src/test/java/dev/roncore/sdk/ErrorParsingTest.java
touch core/src/test/java/dev/roncore/sdk/ConfigEnvLoaderTest.java
touch core/src/test/java/dev/roncore/sdk/RetryPolicyTest.java

# --- kotlin module ----------------------------------------------------------

mkdir -p kotlin/src/main/kotlin/dev/roncore/sdk/kotlin
mkdir -p kotlin/src/test/kotlin/dev/roncore/sdk/kotlin

touch kotlin/build.gradle.kts

touch kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/Ron.kt
touch kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/RonConfigDsl.kt
touch kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/RonExtensions.kt
touch kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/Streaming.kt

touch kotlin/src/test/kotlin/dev/roncore/sdk/kotlin/RonDslTest.kt
touch kotlin/src/test/kotlin/dev/roncore/sdk/kotlin/RonCoroutineTest.kt

# --- facets module ----------------------------------------------------------

mkdir -p facets/src/main/kotlin/dev/roncore/sdk/facets
mkdir -p facets/src/test/kotlin/dev/roncore/sdk/facets

touch facets/build.gradle.kts

touch facets/src/main/kotlin/dev/roncore/sdk/facets/FacetKind.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/FacetSecurity.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/FacetMeta.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/FacetLimits.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/Integrity.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/RouteDefinition.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/FacetDefinition.kt
touch facets/src/main/kotlin/dev/roncore/sdk/facets/FacetTomlWriter.kt

touch facets/src/test/kotlin/dev/roncore/sdk/facets/FacetTomlWriterTest.kt
touch facets/src/test/kotlin/dev/roncore/sdk/facets/FacetSchemaInteropTest.kt

# --- interop-tests module ---------------------------------------------------

mkdir -p interop-tests/src/test/java/dev/roncore/sdk/interop

touch interop-tests/build.gradle.kts

touch interop-tests/src/test/java/dev/roncore/sdk/interop/InteropSmokeTest.java
touch interop-tests/src/test/java/dev/roncore/sdk/interop/ErrorEnvelopeInteropTest.java
touch interop-tests/src/test/java/dev/roncore/sdk/interop/PaginationInteropTest.java

# --- examples ---------------------------------------------------------------

# Java CLI example
mkdir -p examples/java-cli/src/main/java/dev/roncore/sdk/examples
touch examples/java-cli/build.gradle.kts
touch examples/java-cli/src/main/java/dev/roncore/sdk/examples/HelloCli.java

# Spring Boot example
mkdir -p examples/spring-boot/src/main/java/dev/roncore/sdk/examples/spring
touch examples/spring-boot/build.gradle.kts
touch examples/spring-boot/src/main/java/dev/roncore/sdk/examples/spring/RonSdkConfig.java
touch examples/spring-boot/src/main/java/dev/roncore/sdk/examples/spring/HelloController.java

# Kotlin Ktor example
mkdir -p examples/kotlin-ktor/src/main/kotlin/dev/roncore/sdk/examples/ktor
touch examples/kotlin-ktor/build.gradle.kts
touch examples/kotlin-ktor/src/main/kotlin/dev/roncore/sdk/examples/ktor/KtorApp.kt
touch examples/kotlin-ktor/src/main/kotlin/dev/roncore/sdk/examples/ktor/Routes.kt

# Android sample
mkdir -p examples/android-sample/src/main/java/dev/roncore/sdk/examples/android
touch examples/android-sample/build.gradle.kts
touch examples/android-sample/src/main/AndroidManifest.xml
touch examples/android-sample/src/main/java/dev/roncore/sdk/examples/android/MainActivity.kt
touch examples/android-sample/src/main/java/dev/roncore/sdk/examples/android/HelloViewModel.kt
touch examples/android-sample/src/main/java/dev/roncore/sdk/examples/android/RonApp.kt

# --- tools ------------------------------------------------------------------

mkdir -p tools/codegen
mkdir -p tools/ci

touch tools/codegen/openapi-config.yml
touch tools/codegen/regenerate-dtos.sh

touch tools/ci/run-lint.sh
touch tools/ci/run-tests.sh
touch tools/ci/run-interop.sh

echo "Scaffold complete."
