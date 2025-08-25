#!/usr/bin/env bash
set -euo pipefail

BIN_TLDCTL="${BIN_TLDCTL:-target/debug/tldctl}"
INDEX_DB="${INDEX_DB:-.data/index}"
STORE_ROOT="${STORE_ROOT:-.objects}"

echo "[*] build tldctl"
cargo build -p tldctl >/dev/null

mkdir -p "$INDEX_DB" "$STORE_ROOT"
echo "rel-test" > /tmp/rel.txt

# make fake-looking addrs for flags
PARENT="b3:$(printf 'a%.0s' {1..64}).text"
THREAD="b3:$(printf 'b%.0s' {1..64}).text"

OUT="$("$BIN_TLDCTL" pack \
  --tld text \
  --input /tmp/rel.txt \
  --index-db "$INDEX_DB" \
  --store-root "$STORE_ROOT" \
  --parent "$PARENT" \
  --thread "$THREAD" \
  --license CC-BY-4.0 \
  --ext image:width=800 \
  --ext image:height=600 \
  --ext seo:title='Hello World')"

ADDR="$(printf "%s\n" "$OUT" | grep -E '^b3:[0-9a-f]{64}\.[A-Za-z0-9._-]+$' | tail -n1)"
[ -n "$ADDR" ] || { echo "[FAIL] could not parse ADDR"; echo "$OUT"; exit 1; }
echo "[*] ADDR=$ADDR"

# derive on-disk path from your layout: .objects/objects/<tld>/<shard2>/<hex>.<tld>/Manifest.toml
TLD="${ADDR##*.}"
HEX="${ADDR#b3:}"; HEX="${HEX%%.*}"
SHARD="${HEX:0:2}"
MAN="${STORE_ROOT}/objects/${TLD}/${SHARD}/${HEX}.${TLD}/Manifest.toml"

if [ ! -f "$MAN" ]; then
  echo "[FAIL] Manifest.toml not found at $MAN"
  echo "STORE_ROOT=${STORE_ROOT}  TLD=${TLD}  SHARD=${SHARD}  HEX=${HEX}"
  exit 1
fi

echo "[*] Manifest at $MAN"
echo "----- Manifest.toml -----"
sed -n '1,200p' "$MAN"
echo "-------------------------"

# Assertions
grep -q '^\[relations\]' "$MAN" || { echo "[FAIL] missing [relations]"; exit 1; }
grep -q "^parent *= *\"$PARENT\"" "$MAN" || { echo "[FAIL] missing relations.parent"; exit 1; }
grep -q "^thread *= *\"$THREAD\"" "$MAN" || { echo "[FAIL] missing relations.thread"; exit 1; }
grep -q '^license *= *"CC-BY-4.0"' "$MAN" || { echo "[FAIL] missing license"; exit 1; }

grep -q '^\[ext\.image\]' "$MAN" || { echo "[FAIL] missing [ext.image]"; exit 1; }
grep -q '^width *= *"800"' "$MAN" || { echo "[FAIL] missing ext.image.width"; exit 1; }
grep -q '^height *= *"600"' "$MAN" || { echo "[FAIL] missing ext.image.height"; exit 1; }

grep -q '^\[ext\.seo\]' "$MAN" || { echo "[FAIL] missing [ext.seo]"; exit 1; }
grep -q '^title *= *"Hello World"' "$MAN" || { echo "[FAIL] missing ext.seo.title"; exit 1; }

echo "[PASS] smoke_relations: relations, license, and ext.* written."
