#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLDCTL="$ROOT_DIR/target/debug/tldctl"
OUT_DIR="${OUT_DIR:-.onions}"
DB_PATH="${DB_PATH:-.data/index}"

echo "[*] Build tldctl…"
cargo build -q -p tldctl

mkdir -p "$OUT_DIR" "$DB_PATH"

# Use existing sample from test_tlds.sh if present; else make one quickly
PAY="$ROOT_DIR/.tmp.post.txt"
echo "Index me" > "$PAY"

ADDR="$("$TLDCTL" hash --file "$PAY" --tld post)"
echo "[*] Address: $ADDR"

echo "[*] Pack + index…"
"$TLDCTL" --out "$OUT_DIR" --index-db "$DB_PATH" pack --file "$PAY" --tld post --index >/dev/null

echo "[*] Resolve…"
BUNDLE_DIR="$("$TLDCTL" --index-db "$DB_PATH" resolve "$ADDR")"
echo "Resolved path: $BUNDLE_DIR"

test -d "$BUNDLE_DIR" && echo "[OK] Directory exists."
