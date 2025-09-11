#!/usr/bin/env bash
# Runs TSAN inside a Linux container that mirrors CI.
# Requires Docker. Produces the same kind of failures you see in GitHub Actions.

set -euo pipefail

here() { cd "$(dirname "$0")" && pwd; }
ROOT_DIR="$(cd "$(here)"/../../ && pwd)"

IMAGE="rust:nightly-slim"

# Compose a crates arg from testing/tsan_crates.txt (if present), else run workspace
CRATES_ARG=""
if [ -f "$ROOT_DIR/testing/tsan_crates.txt" ]; then
  # Build --package args
  while IFS= read -r line; do
    echo "$line" | grep -Eq '^\s*(#|$)' && continue
    CRATES_ARG="$CRATES_ARG -p $line"
  done < "$ROOT_DIR/testing/tsan_crates.txt"
else
  CRATES_ARG="--workspace"
fi

# Note: we install clang so libtsan is available; build-std requires rust-src.
docker run --rm -it \
  -v "$ROOT_DIR":/work \
  -w /work \
  "$IMAGE" \
  bash -lc '
    set -euo pipefail
    apt-get update && apt-get install -y --no-install-recommends clang ca-certificates pkg-config
    rustup component add rust-src
    export RUSTFLAGS="-Z sanitizer=thread"
    export RUSTDOCFLAGS="-Z sanitizer=thread"
    cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu '"$CRATES_ARG"' --tests -- --nocapture
  '
