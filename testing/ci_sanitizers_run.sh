#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLASSES="$ROOT/ci/crate-classes.toml"
[ -f "$CLASSES" ] || { echo "Missing ci/crate-classes.toml"; exit 1; }

CRATES=$(awk '/\[critical\]/{flag=1;next}/\[/{flag=0}flag' "$CLASSES" | grep -Eo '"[^"]+"' | tr -d '"')
echo "[*] Critical crates:"; echo "$CRATES" | sed 's/^/  - /'

export RUSTFLAGS="-Zsanitizer=thread"
export RUSTDOCFLAGS="-Zsanitizer=thread"
rustup target add x86_64-unknown-linux-gnu --toolchain nightly

FAILED=0
for c in $CRATES; do
  echo "[*] TSAN: $c"
  if ! cargo +nightly test -Z build-std --target x86_64-unknown-linux-gnu -p "$c"; then
    echo "[!] TSAN failed: $c"; FAILED=1
  fi
done
exit $FAILED
