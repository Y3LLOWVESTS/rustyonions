#!/usr/bin/env bash
# RustyOnions — BLAKE3 pack/verify/resolve + overlay PUT/GET smoke test (macOS/POSIX friendly)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_TLDCTL="$ROOT/target/debug/tldctl"

# --- locate node binary (ronode or node) ---
if [ -x "$ROOT/target/debug/ronode" ]; then
  BIN_NODE="$ROOT/target/debug/ronode"
elif [ -x "$ROOT/target/debug/node" ]; then
  BIN_NODE="$ROOT/target/debug/node"
else
  echo "[*] Building node crate…"
  cargo build -q -p node
  if [ -x "$ROOT/target/debug/ronode" ]; then
    BIN_NODE="$ROOT/target/debug/ronode"
  else
    BIN_NODE="$ROOT/target/debug/node"
  fi
fi
echo "[*] Using node binary: $BIN_NODE"

OUT_ROOT="$ROOT/.testrun_store/objects"
IDX_DB="$ROOT/.testrun_data/index"
mkdir -p "$OUT_ROOT" "$(dirname "$IDX_DB")"

echo "[*] Building tldctl…"
cargo build -q -p tldctl

echo "[*] Creating sample file…"
echo "hello rusty onions" > /tmp/hello_ro.txt

echo "[*] Packing via tldctl…"
OUT="$("$BIN_TLDCTL" pack --in /tmp/hello_ro.txt --tld image --out-root "$OUT_ROOT" --index-db "$IDX_DB")"
ADDR="$(printf '%s\n' "$OUT" | sed -n '1p')"
BUNDLE_PATH="$(printf '%s\n' "$OUT" | sed -n '2p')"

echo "[*] Address: $ADDR"
echo "[*] Bundle path: $BUNDLE_PATH"
test -d "$(dirname "$BUNDLE_PATH")" || { echo "!! missing bundle dir"; exit 1; }

echo "[*] Verifying manifest + payload…"
"$BIN_TLDCTL" verify --addr "$ADDR" --out-root "$OUT_ROOT" --deep

echo "[*] Resolving via index…"
RESOLVED="$("$BIN_TLDCTL" resolve --addr "$ADDR" --index-db "$IDX_DB")"
echo "[*] Resolved path: $RESOLVED"

# ---------- helpers ----------
nc_check() { nc -z "$1" "$2" >/dev/null 2>&1; }  # BSD/macOS nc works with -z
wait_for_port() {
  local host="$1" port="$2" timeout="${3:-5}" elapsed=0
  while ! nc_check "$host" "$port"; do
    sleep 0.1
    elapsed=$((elapsed + 1))
    if [ "$elapsed" -ge $((timeout*10)) ]; then
      echo "!! timed out waiting for $host:$port to open"
      return 1
    fi
  done
}

find_free_port() {
  local start="${1:-1777}" end="${2:-1877}" p
  p="$start"
  while [ "$p" -le "$end" ]; do
    if ! nc_check 127.0.0.1 "$p"; then
      echo "$p"
      return 0
    fi
    p=$((p+1))
  done
  return 1
}
# -----------------------------

PORT="$(find_free_port 1777 1877 || true)"
if [ -z "${PORT:-}" ]; then
  echo "[*] No obvious free port found; defaulting to 1777"
  PORT=1777
fi

echo "[*] Starting overlay listener on 127.0.0.1:$PORT"
"$BIN_NODE" serve --bind "127.0.0.1:$PORT" --store-db "$ROOT/.testrun_data/sled-overlay-$PORT" &
SVPID=$!
trap 'kill $SVPID 2>/dev/null || true' EXIT

echo "[*] Waiting for listener to become ready…"
wait_for_port 127.0.0.1 "$PORT" 5

echo "[*] Overlay PUT/GET round-trip…"
echo "overlay round trip" > /tmp/ov_ro.txt
HASH="$("$BIN_NODE" put --to "127.0.0.1:$PORT" --path /tmp/ov_ro.txt)"
"$BIN_NODE" get --from "127.0.0.1:$PORT" --hash "$HASH" --out /tmp/ov_ro.out
diff -u /tmp/ov_ro.txt /tmp/ov_ro.out && echo "[OK] overlay K/V works (hash=$HASH)"

echo "[*] Done."
