#!/usr/bin/env bash
# Runs ThreadSanitizer on selected crates locally (macOS or Linux).
# - Bash 3.2 compatible
# - Uses nightly + build-std (instrument std with TSAN)
# - Reads optional crate list from testing/tsan_crates.txt (one crate per line, # comments allowed)
# - Falls back to --workspace when file is absent
# - Preserves your environment; just sets the sanitizer flags needed

set -euo pipefail

here() { cd "$(dirname "$0")" && pwd; }
ROOT_DIR="$(cd "$(here)"/../../ && pwd)"
cd "$ROOT_DIR"

OS="$(uname -s)"
ARCH="$(uname -m)"

# ---- pick target triple ------------------------------------------------------
if [ "$OS" = "Darwin" ]; then
  case "$ARCH" in
    arm64) TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *) echo "[run_tsan_local] Unsupported mac arch: $ARCH" >&2; exit 2 ;;
  esac
else
  TARGET="x86_64-unknown-linux-gnu"
fi

# ---- ensure toolchain bits ---------------------------------------------------
if ! command -v rustup >/dev/null 2>&1; then
  echo "[run_tsan_local] rustup not found. Install Rust first." >&2
  exit 2
fi

# nightly + rust-src for -Zbuild-std
rustup toolchain install nightly >/dev/null 2>&1 || true
rustup component add rust-src --toolchain nightly >/dev/null 2>&1 || true

# clang is needed for sanitizer runtimes; on macOS Xcode clang usually suffices
if ! command -v clang >/dev/null 2>&1; then
  echo "[run_tsan_local] clang not found in PATH. Install LLVM/Clang (e.g., brew install llvm) and re-run." >&2
  exit 2
fi

# ---- build crate list --------------------------------------------------------
PKGS=()
CRATES_FILE="$ROOT_DIR/testing/tsan_crates.txt"
if [ -f "$CRATES_FILE" ]; then
  while IFS= read -r line; do
    # skip blank/comment lines
    echo "$line" | grep -Eq '^\s*(#|$)' && continue
    PKGS+=("-p" "$line")
  done < "$CRATES_FILE"
fi
if [ ${#PKGS[@]} -eq 0 ]; then
  # default to critical crates commonly used in this repo; tweak if needed
  # If you keep tsan_crates.txt in the repo, this fallback won’t be used.
  PKGS=(-p ron-kernel -p overlay -p gateway || true)
fi

# ---- run tests with TSAN -----------------------------------------------------
export RUSTFLAGS="-Z sanitizer=thread ${RUSTFLAGS-}"
export RUSTDOCFLAGS="-Z sanitizer=thread ${RUSTDOCFLAGS-}"

# Optional TSAN tuning (uncomment to keep going on first error, or add suppressions):
# export TSAN_OPTIONS="halt_on_error=1 report_signal_unsafe=0 suppressions=$ROOT_DIR/testing/tools/tsan.supp"

set -x
cargo +nightly test \
  -Zbuild-std \
  --target "$TARGET" \
  "${PKGS[@]}" \
  --tests \
  -- --nocapture
