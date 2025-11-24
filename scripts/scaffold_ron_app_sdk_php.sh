#!/usr/bin/env bash
set -euo pipefail

# Root for the PHP SDK
ROOT="sdk/ron-app-sdk-php"

make_dir() {
  local dir="$1"
  mkdir -p "$dir"
}

make_file() {
  local file="$1"
  make_dir "$(dirname "$file")"
  if [ ! -f "$file" ]; then
    : > "$file"
  fi
}

echo "Scaffolding ron-app-sdk-php into $ROOT"

# Root-level files
make_dir "$ROOT"

make_file "$ROOT/README.md"
make_file "$ROOT/CHANGELOG.md"
make_file "$ROOT/LICENSE-MIT"
make_file "$ROOT/LICENSE-APACHE"
make_file "$ROOT/composer.json"
make_file "$ROOT/phpunit.xml.dist"
make_file "$ROOT/phpstan.neon.dist"
make_file "$ROOT/psalm.xml.dist"
make_file "$ROOT/.gitignore"
make_file "$ROOT/.gitattributes"
make_file "$ROOT/.editorconfig"
make_file "$ROOT/.php-cs-fixer.dist.php"
make_file "$ROOT/SDK_IDB.MD"
make_file "$ROOT/SDK_SECURITY.MD"
make_file "$ROOT/SDK_SCHEMA_IDB.MD"
make_file "$ROOT/CODECHECK.MD"
make_file "$ROOT/CODECOMMENTS.MD"
make_file "$ROOT/TODO.md"

# docs/
make_dir  "$ROOT/docs"
make_file "$ROOT/docs/OVERVIEW.md"
make_file "$ROOT/docs/ARCH.mmd"
make_file "$ROOT/docs/ARCH.svg"
make_file "$ROOT/docs/ERROR_MODEL.md"
make_file "$ROOT/docs/SCHEMA_NOTES.md"

# examples/
make_dir  "$ROOT/examples"
make_file "$ROOT/examples/hello.php"
make_file "$ROOT/examples/pagination.php"
make_file "$ROOT/examples/worker.php"

# src/
make_dir  "$ROOT/src"
make_file "$ROOT/src/RonClient.php"
make_file "$ROOT/src/ClientConfig.php"
make_file "$ROOT/src/Response.php"
make_file "$ROOT/src/Problem.php"
make_file "$ROOT/src/Pagination.php"

# src/Middleware/
make_dir  "$ROOT/src/Middleware"
make_file "$ROOT/src/Middleware/LoggingMiddleware.php"
make_file "$ROOT/src/Middleware/RetryMiddleware.php"
make_file "$ROOT/src/Middleware/IdempotencyMiddleware.php"

# src/Http/
make_dir  "$ROOT/src/Http"
make_file "$ROOT/src/Http/HttpClientInterface.php"
make_file "$ROOT/src/Http/GuzzleHttpClient.php"
make_file "$ROOT/src/Http/SymfonyHttpClient.php"

# src/Exception/
make_dir  "$ROOT/src/Exception"
make_file "$ROOT/src/Exception/RonException.php"
make_file "$ROOT/src/Exception/RonConfigException.php"
make_file "$ROOT/src/Exception/RonNetworkException.php"
make_file "$ROOT/src/Exception/RonTimeoutException.php"
make_file "$ROOT/src/Exception/RonAuthException.php"
make_file "$ROOT/src/Exception/RonProblemException.php"

# src/Util/
make_dir  "$ROOT/src/Util"
make_file "$ROOT/src/Util/Env.php"
make_file "$ROOT/src/Util/IdempotencyKey.php"
make_file "$ROOT/src/Util/Json.php"

# tests/
make_dir  "$ROOT/tests"
make_file "$ROOT/tests/bootstrap.php"

# tests/Unit/
make_dir  "$ROOT/tests/Unit"
make_file "$ROOT/tests/Unit/ClientConfigTest.php"
make_file "$ROOT/tests/Unit/RonClientBasicTest.php"
make_file "$ROOT/tests/Unit/ErrorMappingTest.php"
make_file "$ROOT/tests/Unit/PaginationTest.php"
make_file "$ROOT/tests/Unit/SecurityHeadersTest.php"

# tests/Integration/
make_dir  "$ROOT/tests/Integration"
make_file "$ROOT/tests/Integration/HelloWorldTest.php"
make_file "$ROOT/tests/Integration/PaginationLiveTest.php"

# .github/workflows/
make_dir  "$ROOT/.github/workflows"
make_file "$ROOT/.github/workflows/ci.yml"
make_file "$ROOT/.github/workflows/security.yml"

# tools/
make_dir  "$ROOT/tools"
make_file "$ROOT/tools/gen-schema-fixtures.php"
make_file "$ROOT/tools/smoke-app-plane.sh"

echo "Scaffold complete."
