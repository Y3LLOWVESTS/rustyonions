#!/usr/bin/env bash
# RustyOnions — Gateway smoke test (no Sled lock contention)
# Packs+indexes bundles FIRST, then starts the gateway to fetch them.

set -euo pipefail

# ---------------- Config ----------------
BIND_HOST="${BIND_HOST:-127.0.0.1}"
PORT="${PORT:-0}"                 # 0 = auto-pick
OUT_DIR="${OUT_DIR:-.onions}"
INDEX_DB="${INDEX_DB:-.data/index}"
ALGO="${ALGO:-sha256}"
QUIET="${QUIET:-0}"
KEEP_TMP="${KEEP_TMP:-0}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLDCTL="$ROOT_DIR/target/debug/tldctl"
GATEWAY="$ROOT_DIR/target/debug/gateway"

log() { [ "$QUIET" = "1" ] && return 0; echo -e "$@"; }
need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1" >&2; exit 1; }; }

# ---------------- Preflight ----------------
[ -f "$ROOT_DIR/Cargo.toml" ] || { echo "Run inside repo (no Cargo.toml)"; exit 1; }
need curl
mkdir -p "$OUT_DIR" "$INDEX_DB"

log "[*] Building gateway and tldctl…"
cargo build -q -p tldctl -p gateway

TMP_DIR="$(mktemp -d -t ron_gateway.XXXXXX)"
trap 'EC=$?; [ "$KEEP_TMP" = "1" ] || rm -rf "$TMP_DIR"; if [ -n "${GATEWAY_PID:-}" ]; then kill "$GATEWAY_PID" >/dev/null 2>&1 || true; fi; exit $EC' EXIT

# ---------------- Sample payloads ----------------
POST_TXT="$TMP_DIR/post.txt"
IMG_PNG="$TMP_DIR/pixel.png"
echo "Hello from RustyOnions gateway test (.post)" > "$POST_TXT"
base64 -d >"$IMG_PNG" <<'B64'
iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGMAAQAABQAB
JzQnWAAAAABJRU5ErkJggg==
B64

# ---------------- Pack + index (do this BEFORE gateway starts) ----------------
log "[*] Creating bundles (pack + index)…"
ADDR_POST="$("$TLDCTL" --out "$OUT_DIR" --index-db "$INDEX_DB" \
  pack --file "$POST_TXT" --tld post --algo "$ALGO" --index \
  | sed -n 's/^OK: .*\/\([^/]*\)\.post\/Manifest\.toml$/\1.post/p')"
[ -n "$ADDR_POST" ] || ADDR_POST="$("$TLDCTL" hash --file "$POST_TXT" --tld post --algo "$ALGO")"

ADDR_IMAGE="$("$TLDCTL" --out "$OUT_DIR" --index-db "$INDEX_DB" \
  pack --file "$IMG_PNG" --tld image --algo "$ALGO" --index \
  | sed -n 's/^OK: .*\/\([^/]*\)\.image\/Manifest\.toml$/\1.image/p')"
[ -n "$ADDR_IMAGE" ] || ADDR_IMAGE="$("$TLDCTL" hash --file "$IMG_PNG" --tld image --algo "$ALGO")"

log "    .post  → $ADDR_POST"
log "    .image → $ADDR_IMAGE"

# ---------------- Start gateway AFTER indexing ----------------
# Choose a port if PORT=0
if [ "$PORT" = "0" ]; then
  PORT="$(python3 - <<'PY' || true
import socket
s=socket.socket()
s.bind(("127.0.0.1",0))
print(s.getsockname()[1])
s.close()
PY
)"
  [ -n "$PORT" ] || PORT=31555
fi

log "[*] Starting gateway on http://$BIND_HOST:$PORT …"
"$GATEWAY" --bind "$BIND_HOST:$PORT" --index-db "$INDEX_DB" --root "$OUT_DIR" \
  > "$TMP_DIR/gateway.log" 2>&1 &
GATEWAY_PID=$!

# Wait for /healthz
HEALTH_URL="http://$BIND_HOST:$PORT/healthz"
for i in {1..60}; do
  if curl -fsS "$HEALTH_URL" >/dev/null 2>&1; then
    log "[*] Gateway is up."
    break
  fi
  sleep 0.2
  [ $i -eq 60 ] && { echo "Gateway failed to start (see $TMP_DIR/gateway.log)"; exit 1; }
done

# ---------------- Verify over HTTP ----------------
MP_POST="http://$BIND_HOST:$PORT/o/$ADDR_POST/Manifest.toml"
MP_IMG="http://$BIND_HOST:$PORT/o/$ADDR_IMAGE/Manifest.toml"
PB_IMG="http://$BIND_HOST:$PORT/o/$ADDR_IMAGE/payload.bin"

log "[*] GET $MP_POST"
curl -sS -f "$MP_POST" >/dev/null || { echo "Failed to fetch $MP_POST"; exit 1; }

log "[*] GET $MP_IMG"
curl -sS -f "$MP_IMG" >/dev/null || { echo "Failed to fetch $MP_IMG"; exit 1; }

log "[*] HEAD $PB_IMG"
HDRS="$(curl -sS -I "$PB_IMG")" || { echo "Failed to fetch headers for payload.bin"; exit 1; }
CT="$(printf "%s\n" "$HDRS" | awk -F': ' 'tolower($1)=="content-type"{print $2}' | tr -d '\r')"
CL="$(printf "%s\n" "$HDRS" | awk -F': ' 'tolower($1)=="content-length"{print $2}' | tr -d '\r')"

# ---------------- Summary ----------------
echo
echo "=== Gateway Test Summary ==="
echo "Gateway   : http://$BIND_HOST:$PORT"
echo "OUT_DIR   : $OUT_DIR"
echo "INDEX_DB  : $INDEX_DB"
echo "ALGO      : $ALGO"
echo
echo "POST addr : $ADDR_POST"
echo "IMAGE addr: $ADDR_IMAGE"
echo
echo "Manifest (post): $MP_POST"
echo "Manifest (img) : $MP_IMG"
echo "Payload  (img) : $PB_IMG"
echo "Payload headers:"
echo "  Content-Type  : ${CT:-<missing>}"
echo "  Content-Length: ${CL:-<missing>}"
echo
echo "(Gateway logs: $TMP_DIR/gateway.log)"
