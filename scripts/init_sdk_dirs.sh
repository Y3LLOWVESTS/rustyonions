#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Create language-specific SDK folders under /sdk.
# RO:WHY  — Keep SDK layout consistent across languages (rs/ts/py/etc).
# RO:INVARIANTS —
#   - Script is run from the repo root (where /sdk should live).
#   - All directory names follow ron-app-sdk-<langcode>.
#   - Script is idempotent (safe to run multiple times).

ROOT_DIR="$(pwd)"
SDK_DIR="${ROOT_DIR}/sdk"

echo "[sdk-init] root=${ROOT_DIR}"
echo "[sdk-init] ensuring ${SDK_DIR} exists"
mkdir -p "${SDK_DIR}"

SDK_FOLDERS=(
  "ron-app-sdk-rs"      # Rust
  "ron-app-sdk-ts"      # TypeScript
  "ron-app-sdk-js"      # JavaScript
  "ron-app-sdk-py"      # Python
  "ron-app-sdk-go"      # Go
  "ron-app-sdk-java"    # Java
  "ron-app-sdk-csharp"  # C#
  "ron-app-sdk-kt"      # Kotlin
  "ron-app-sdk-swift"   # Swift
  "ron-app-sdk-php"     # PHP
  "ron-app-sdk-rb"      # Ruby
  "ron-app-sdk-cpp"     # C++
  "ron-app-sdk-c"       # C
  "ron-app-sdk-hs"      # Haskell
  "ron-app-sdk-ex"      # Elixir
  "ron-app-sdk-scala"   # Scala
  "ron-app-sdk-dart"    # Dart
  "ron-app-sdk-zig"     # Zig
  "ron-app-sdk-pony"    # Pony
  "ron-app-sdk-lua"     # Lua
  "ron-app-sdk-clj"     # Clojure
)

for folder in "${SDK_FOLDERS[@]}"; do
  target="${SDK_DIR}/${folder}"
  if [ -d "${target}" ]; then
    echo "[sdk-init] exists: ${target}"
  else
    echo "[sdk-init] creating: ${target}"
    mkdir -p "${target}"
  fi
done

echo "[sdk-init] done."
