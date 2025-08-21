#!/usr/bin/env bash
# RustyOnions — TLD scaffold smoke test
# Creates tiny sample payloads, hashes them into <sha>.<tld>, packs bundles
# with Manifest.toml, and verifies key fields.
#
# Usage:
#   testing/test_tlds.sh                 # default quick run
#   OUT_DIR=.onions  ALGO=sha256  QUIET=1  testing/test_tlds.sh
#
# Env:
#   OUT_DIR    Directory to place bundles (default: .onions)
#   ALGO       Hash algo: sha256 | blake3  (default: sha256)
#   QUIET      Set to 1 for less output
#   KEEP_TMP   Set to 1 to keep temp working dir

set -euo pipefail

# ---------- Config ----------
OUT_DIR="${OUT_DIR:-.onions}"
ALGO="${ALGO:-sha256}"
QUIET="${QUIET:-0}"
KEEP_TMP="${KEEP_TMP:-0}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLDCTL="$ROOT_DIR/target/debug/tldctl"

log() { [ "$QUIET" = "1" ] && return 0; echo -e "$@"; }

# ---------- Pre-flight ----------
if [ ! -f "$ROOT_DIR/Cargo.toml" ]; then
  echo "Run from inside the repo (can't find Cargo.toml at $ROOT_DIR)" >&2
  exit 1
fi

# Make sure the CLI is built
log "[*] Building tldctl..."
cargo build -q -p tldctl

# Workdir
TMP_DIR="$(mktemp -d -t ron_tlds.XXXXXX)"
trap 'EC=$?; [ "$KEEP_TMP" = "1" ] || rm -rf "$TMP_DIR"; exit $EC' EXIT

# Sample payloads
POST_TXT="$TMP_DIR/post.txt"
COMMENT_TXT="$TMP_DIR/comment.txt"
IMG_PNG="$TMP_DIR/pixel.png"

cat >"$POST_TXT" <<'TXT'
RustyOnions hello world — this is a sample post to test .post addressing.
TXT

cat >"$COMMENT_TXT" <<'TXT'
This is a comment replying to the sample post — testing .comment addressing.
TXT

# tiny 1x1 PNG (transparent)
base64 -d >"$IMG_PNG" <<'B64'
iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGMAAQAABQAB
JzQnWAAAAABJRU5ErkJggg==
B64

# Ensure output dir
mkdir -p "$OUT_DIR"

# ---------- Helpers ----------
hash_addr() {
  local file="$1" tld="$2"
  "$TLDCTL" hash --file "$file" --tld "$tld" --algo "$ALGO"
}

pack_bundle() {
  local file="$1" tld="$2"
  "$TLDCTL" --out "$OUT_DIR" pack --file "$file" --tld "$tld" --algo "$ALGO" >/dev/null
}

manifest_path_for() {
  local addr="$1"
  echo "$OUT_DIR/$addr/Manifest.toml"
}

file_size() { stat -f%z "$1" 2>/dev/null || stat -c%s "$1"; }

# Extract simple TOML scalars with grep/sed (good enough for our fields)
toml_get() {
  local key="$1" file="$2"
  # Matches: key = "value"   OR   key = value
  sed -n -E "s/^[[:space:]]*$key[[:space:]]*=[[:space:]]*\"?([^\"#]+)\"?.*$/\1/p" "$file" | head -n1 | tr -d '[:space:]'
}

# For nested hash.digest we can just grab the first digest occurrence
toml_get_digest() {
  local file="$1"
  sed -n -E 's/^[[:space:]]*digest[[:space:]]*=[[:space:]]*\"?([0-9a-fA-F]+)\"?.*$/\1/p' "$file" | head -n1 | tr '[:upper:]' '[:lower:]'
}

# ---------- Flow ----------
log "[*] Hashing payloads to addresses..."
ADDR_POST="$(hash_addr "$POST_TXT" post)"
ADDR_COMMENT="$(hash_addr "$COMMENT_TXT" comment)"
ADDR_IMAGE="$(hash_addr "$IMG_PNG" image)"

log "    .post    -> $ADDR_POST"
log "    .comment -> $ADDR_COMMENT"
log "    .image   -> $ADDR_IMAGE"

log "[*] Packing bundles..."
pack_bundle "$POST_TXT" post
pack_bundle "$COMMENT_TXT" comment
pack_bundle "$IMG_PNG" image

MP_POST="$(manifest_path_for "$ADDR_POST")"
MP_COMMENT="$(manifest_path_for "$ADDR_COMMENT")"
MP_IMAGE="$(manifest_path_for "$ADDR_IMAGE")"

for MP in "$MP_POST" "$MP_COMMENT" "$MP_IMAGE"; do
  [ -f "$MP" ] || { echo "Missing Manifest at $MP" >&2; exit 2; }
done

log "[*] Verifying Manifests..."

verify_manifest() {
  local addr="$1" mp="$2" payload="$3" expected_tld="$4"
  local hex="${addr%%.*}"
  local tld="${addr##*.}"

  # Address line matches
  local m_addr="$(toml_get address "$mp")"
  [ "$m_addr" = "$addr" ] || { echo "address mismatch: $m_addr != $addr ($mp)"; return 3; }

  # TLD & size
  local m_tld="$(toml_get tld "$mp")"
  [ "$m_tld" = "$expected_tld" ] || { echo "tld mismatch: $m_tld != $expected_tld ($mp)"; return 3; }

  local m_size="$(toml_get size "$mp")"
  local f_size="$(file_size "$payload")"
  [ "$m_size" = "$f_size" ] || { echo "size mismatch: $m_size != $f_size ($mp)"; return 3; }

  # Digest equals the address hex
  local m_digest="$(toml_get_digest "$mp")"
  if [ -z "$m_digest" ]; then
    echo "missing digest in $mp"; return 3
  fi
  [ "$m_digest" = "$hex" ] || { echo "digest mismatch: $m_digest != $hex ($mp)"; return 3; }

  return 0
}

verify_manifest "$ADDR_POST"    "$MP_POST"    "$POST_TXT"    "post"
verify_manifest "$ADDR_COMMENT" "$MP_COMMENT" "$COMMENT_TXT" "comment"
verify_manifest "$ADDR_IMAGE"   "$MP_IMAGE"   "$IMG_PNG"     "image"

log "[*] OK — all manifests look consistent."

# ---------- Summary ----------
echo
echo "=== TLD Test Summary ==="
printf "OUT_DIR: %s\nALGO: %s\n\n" "$OUT_DIR" "$ALGO"
printf "POST    %s\n"    "$ADDR_POST"
printf "COMMENT %s\n"    "$ADDR_COMMENT"
printf "IMAGE   %s\n"    "$ADDR_IMAGE"
echo
echo "Bundle paths:"
echo "  $OUT_DIR/$ADDR_POST/"
echo "  $OUT_DIR/$ADDR_COMMENT/"
echo "  $OUT_DIR/$ADDR_IMAGE/"
