#!/usr/bin/env bash
# RustyOnions — Gateway smoke test (HTTP gateway -> UDS overlay/index/storage)
# Packs+indexes bundles FIRST, then starts svc-index, svc-storage, svc-overlay, and the HTTP gateway.
# macOS Bash 3.2 compatible (no readarray/mapfile; BSD grep OK).

set -euo pipefail

# ---------------- Config ----------------
BIND_HOST="${BIND_HOST:-127.0.0.1}"
PORT="${PORT:-0}"                     # HTTP gateway port (0 = auto-pick)
OUT_DIR="${OUT_DIR:-.onions}"         # bundle root for bytes
INDEX_DB_BASE="${INDEX_DB:-}"         # if empty we use a per-run tmp index
ALGO="${ALGO:-blake3}"

# Tunables
HTTP_WAIT_SEC="${HTTP_WAIT_SEC:-15}"  # wait up to N seconds for HTTP to accept
QUIET="${QUIET:-0}"
KEEP_TMP="${KEEP_TMP:-0}"
STREAM_LOGS="${STREAM_LOGS:-$([ "$QUIET" = "1" ] && echo 0 || echo 1)}"  # 1=tail -f logs live

# Verbosity for binaries
RUST_LOG_SVCS="${RUST_LOG_SVCS:-info,svc_index=debug,svc_storage=debug,svc_overlay=debug}"
RUST_LOG_GW="${RUST_LOG_GW:-info,gateway=debug}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLDCTL="$ROOT_DIR/target/debug/tldctl"
GATEWAY="$ROOT_DIR/target/debug/gateway"
SVC_INDEX="$ROOT_DIR/target/debug/svc-index"
SVC_STORAGE="$ROOT_DIR/target/debug/svc-storage"
SVC_OVERLAY="$ROOT_DIR/target/debug/svc-overlay"

ARCHIVE_DIR="${ARCHIVE_DIR:-$ROOT_DIR/testing/_logs/last_run}"

log() { [ "$QUIET" = "1" ] && return 0; echo -e "$@"; }
need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1" >&2; exit 1; }; }

# ---------------- Preflight ----------------
[ -f "$ROOT_DIR/Cargo.toml" ] || { echo "Run inside repo (no Cargo.toml)"; exit 1; }
need curl
mkdir -p "$OUT_DIR" "$ARCHIVE_DIR"

log "[*] Building tldctl + services + gateway…"
cargo build -q -p tldctl -p svc-index -p svc-storage -p svc-overlay -p gateway

TMP_DIR="$(mktemp -d -t ron_gateway_uds.XXXXXX)"
RUN_DIR="$TMP_DIR/run"
LOG_DIR="$TMP_DIR/logs"
mkdir -p "$RUN_DIR" "$LOG_DIR"

GW_LOG="$LOG_DIR/gateway.log"
IDX_LOG="$LOG_DIR/svc-index.log"
STO_LOG="$LOG_DIR/svc-storage.log"
OVL_LOG="$LOG_DIR/svc-overlay.log"
: >"$GW_LOG"; : >"$IDX_LOG"; : >"$STO_LOG"; : >"$OVL_LOG"

TAIL_GW_PID=""; TAIL_IDX_PID=""; TAIL_STO_PID=""; TAIL_OVL_PID=""

trap '
  EC=$?
  { mkdir -p "'"$ARCHIVE_DIR"'" \
      && cp -f "'"$GW_LOG"'" "'"$IDX_LOG"'" "'"$STO_LOG"'" "'"$OVL_LOG"'" "'"$ARCHIVE_DIR"'/" >/dev/null 2>&1 || true; }
  [ -n "$TAIL_GW_PID" ]  && kill "$TAIL_GW_PID"  >/dev/null 2>&1 || true
  [ -n "$TAIL_IDX_PID" ] && kill "$TAIL_IDX_PID" >/dev/null 2>&1 || true
  [ -n "$TAIL_STO_PID" ] && kill "$TAIL_STO_PID" >/dev/null 2>&1 || true
  [ -n "$TAIL_OVL_PID" ] && kill "$TAIL_OVL_PID" >/dev/null 2>&1 || true
  [ -n "${GATEWAY_PID:-}" ] && kill "$GATEWAY_PID" >/dev/null 2>&1 || true
  [ -n "${IDX_PID:-}" ]  && kill "$IDX_PID"  >/dev/null 2>&1 || true
  [ -n "${STO_PID:-}" ]  && kill "$STO_PID"  >/dev/null 2>&1 || true
  [ -n "${OVL_PID:-}" ]  && kill "$OVL_PID"  >/dev/null 2>&1 || true
  if [ "$KEEP_TMP" = "1" ] || [ $EC -ne 0 ]; then
    echo "(Keeping TMP_DIR: '"$TMP_DIR"')"
  else
    rm -rf "'"$TMP_DIR"'"
  fi
  echo "(Gateway logs : '"$GW_LOG"' — copy: '"$ARCHIVE_DIR"'/gateway.log)"
  echo "(svc-index    : '"$IDX_LOG"' — copy: '"$ARCHIVE_DIR"'/svc-index.log)"
  echo "(svc-storage  : '"$STO_LOG"' — copy: '"$ARCHIVE_DIR"'/svc-storage.log)"
  echo "(svc-overlay  : '"$OVL_LOG"' — copy: '"$ARCHIVE_DIR"'/svc-overlay.log)"
  exit $EC
' EXIT

# ---------------- Sample payloads ----------------
POST_TXT="$TMP_DIR/post.txt"
echo "Hello from RustyOnions gateway test (.post)" > "$POST_TXT"

IMG_AVIF="$TMP_DIR/pixel.avif"  # optional
if [ -n "${TEST_AVIF_PATH:-}" ] && [ -f "$TEST_AVIF_PATH" ]; then
  cp -f "$TEST_AVIF_PATH" "$IMG_AVIF"
fi

# ---------------- tldctl compatibility probe (exact flags) ------------------
PACK_HELP="$("$TLDCTL" pack --help 2>&1 || true)"
PACK_FLAGS="$(printf "%s\n" "$PACK_HELP" | grep -Eo -- '--[a-zA-Z0-9][a-zA-Z0-9-]*' | sort -u)"

has_flag() {
  printf "%s\n" "$PACK_FLAGS" | grep -Fxq -- "$1"
}

PACK_HAS_INPUT=0;    has_flag "--input"       && PACK_HAS_INPUT=1
PACK_HAS_FILE=0;     has_flag "--file"        && PACK_HAS_FILE=1
PACK_HAS_STORE=0;    has_flag "--store-root"  && PACK_HAS_STORE=1
PACK_HAS_OUT=0;      has_flag "--out"         && PACK_HAS_OUT=1
PACK_HAS_INDEX_DB=0; has_flag "--index-db"    && PACK_HAS_INDEX_DB=1
PACK_HAS_ALGO=0;     has_flag "--algo"        && PACK_HAS_ALGO=1
PACK_HAS_TLD=0;      has_flag "--tld"         && PACK_HAS_TLD=1

build_pack_argv_nul() {
  # emit NUL-delimited argv for: tldctl pack ...
  local INPUT_PATH="$1" TLD="$2"
  local args=( pack )

  if [ "$PACK_HAS_TLD" -eq 1 ]; then
    args+=( --tld "$TLD" )
  else
    args+=( "$TLD" )
  fi

  if [ "$PACK_HAS_INPUT" -eq 1 ]; then
    args+=( --input "$INPUT_PATH" )
  elif [ "$PACK_HAS_FILE" -eq 1 ]; then
    args+=( --file "$INPUT_PATH" )
  else
    echo "[!] tldctl pack: neither --input nor --file supported; cannot proceed." >&2
    exit 1
  fi

  [ "$PACK_HAS_ALGO" -eq 1 ] && args+=( --algo "$ALGO" )

  # IMPORTANT: never add a separate --index (unsupported in your build).
  if [ "$PACK_HAS_INDEX_DB" -eq 1 ]; then
    args+=( --index-db "$INDEX_DB_EFF" )
  fi

  if [ "$PACK_HAS_STORE" -eq 1 ]; then
    args+=( --store-root "$OUT_DIR" )
  elif [ "$PACK_HAS_OUT" -eq 1 ]; then
    args+=( --out "$OUT_DIR" )
  else
    echo "[!] tldctl pack: neither --store-root nor --out supported; cannot proceed." >&2
    exit 1
  fi

  printf '%s\0' "${args[@]}"
}

run_pack() {
  # run_pack <INPUT_PATH> <tld> <OUTFILE> -> echoes "<hex>.<tld>"
  local INPUT_PATH="$1" TLD="$2" OUTFILE="$3"

  local argv=()
  while IFS= read -r -d '' tok; do argv+=("$tok"); done < <(build_pack_argv_nul "$INPUT_PATH" "$TLD")

  log "[*] Running: $TLDCTL $(printf '%q ' "${argv[@]}")"
  if ! "$TLDCTL" "${argv[@]}" >"$OUTFILE" 2>&1; then
    echo "[!] tldctl pack($TLD) failed. Output:"; sed -n '1,200p' "$OUTFILE" || true
    return 1
  fi

  local addr
  addr="$(sed -n "s#^OK: .*/\\([^/]*\\)\\.${TLD}/Manifest\\.toml\$#\\1.${TLD}#p" "$OUTFILE")"
  if [ -z "$addr" ]; then
    echo "[!] Could not extract .$TLD address from pack output. See below:"
    sed -n '1,200p' "$OUTFILE" || true
    return 1
  fi

  printf "%s" "$addr"
  return 0
}

is_valid_addr() {
  local a="$1"
  [[ "$a" =~ ^(b3:)?[0-9a-f]{8,}\.[a-z0-9]+$ ]] && return 0 || return 1
}

# ---------------- Index location (per-run tmp unless INDEX_DB set) ----------
if [ -n "$INDEX_DB_BASE" ]; then
  INDEX_DB_EFF="$INDEX_DB_BASE"
else
  INDEX_DB_EFF="$TMP_DIR/index"
fi
mkdir -p "$INDEX_DB_EFF"
export RON_INDEX_DB="$INDEX_DB_EFF"   # share DB across tldctl/services/gateway

# ---------------- Pack + index (BEFORE servers start) ----------------------
log "[*] Creating bundles (pack + index)…"
POST_OUT="$TMP_DIR/pack_post.out"
IMG_OUT="$TMP_DIR/pack_img.out"

ADDR_POST="$(run_pack "$POST_TXT" post "$POST_OUT")" || exit 1
log "    .post  → $ADDR_POST"

# Verify object is in THIS DB
if ! RON_INDEX_DB="$INDEX_DB_EFF" "$TLDCTL" resolve "$ADDR_POST" >/dev/null 2>&1 \
   && ! RON_INDEX_DB="$INDEX_DB_EFF" "$TLDCTL" resolve "b3:$ADDR_POST" >/dev/null 2>&1; then
  echo "[!] Fresh index DB ($INDEX_DB_EFF) does not contain $ADDR_POST after pack."
  echo "    pack(post) stdout:"; sed -n '1,200p' "$POST_OUT" || true
  if RON_INDEX_DB="$INDEX_DB_EFF" "$TLDCTL" index scan --store-root "$OUT_DIR" >/dev/null 2>&1; then
    if ! RON_INDEX_DB="$INDEX_DB_EFF" "$TLDCTL" resolve "b3:$ADDR_POST" >/dev/null 2>&1; then
      exit 1
    fi
    echo "[*] Reindex succeeded; address now present in index DB."
  else
    exit 1
  fi
fi

# Optional .image
ADDR_IMAGE=""
if [ -s "$IMG_AVIF" ]; then
  if ADDR_IMAGE="$(run_pack "$IMG_AVIF" image "$IMG_OUT")"; then
    log "    .image → $ADDR_IMAGE"
  else
    echo "[!] .image pack did not produce an address; image checks will be skipped."
    echo "    pack(image) stdout:"; sed -n '1,200p' "$IMG_OUT" || true
  fi
else
  echo "[!] No AVIF sample found; skipping .image pack. To test .image, set TEST_AVIF_PATH=<path>.avif"
fi

# ---------------- Start UDS services ---------------------------------------
RON_INDEX_SOCK="$RUN_DIR/svc-index.sock"
RON_STORAGE_SOCK="$RUN_DIR/svc-storage.sock"
RON_OVERLAY_SOCK="$RUN_DIR/svc-overlay.sock"

log "[*] Starting svc-index/storage/overlay …"
( set -x; RUST_LOG="$RUST_LOG_SVCS" RON_INDEX_SOCK="$RON_INDEX_SOCK" RON_INDEX_DB="$INDEX_DB_EFF" "$SVC_INDEX" ) >> "$IDX_LOG" 2>&1 & IDX_PID=$!
( set -x; RUST_LOG="$RUST_LOG_SVCS" RON_STORAGE_SOCK="$RON_STORAGE_SOCK" "$SVC_STORAGE" ) >> "$STO_LOG" 2>&1 & STO_PID=$!
( set -x; RUST_LOG="$RUST_LOG_SVCS" RON_OVERLAY_SOCK="$RON_OVERLAY_SOCK" RON_INDEX_SOCK="$RON_INDEX_SOCK" RON_STORAGE_SOCK="$RON_STORAGE_SOCK" "$SVC_OVERLAY" ) >> "$OVL_LOG" 2>&1 & OVL_PID=$!

if [ "$STREAM_LOGS" = "1" ]; then
  tail -f "$IDX_LOG" & TAIL_IDX_PID=$!
  tail -f "$STO_LOG" & TAIL_STO_PID=$!
  tail -f "$OVL_LOG" & TAIL_OVL_PID=$!
fi

# ---------------- Start HTTP gateway (points to overlay UDS) ----------------
if [ "$PORT" = "0" ]; then
  PORT="$(python3 - <<'PY' || true
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"; [ -n "$PORT" ] || PORT=31555
fi

log "[*] Starting HTTP gateway on http://$BIND_HOST:$PORT …"
( set -x; RUST_LOG="$RUST_LOG_GW" RON_OVERLAY_SOCK="$RON_OVERLAY_SOCK" "$GATEWAY" --bind "$BIND_HOST:$PORT" --index-db "$INDEX_DB_EFF" ) >> "$GW_LOG" 2>&1 & GATEWAY_PID=$!

if [ "$STREAM_LOGS" = "1" ]; then tail -f "$GW_LOG" & TAIL_GW_PID=$!; fi

# ---- Readiness: wait for HTTP TCP accept ----
tcp_up=0
HTTP_TRIES=$(( HTTP_WAIT_SEC * 20 )) # 50ms steps
for i in $(seq 1 "$HTTP_TRIES"); do
  if bash -c "exec 3<>/dev/tcp/$BIND_HOST/$PORT" 2>/dev/null; then exec 3>&- 3<&- || true; tcp_up=1; break; fi
  if ! kill -0 "$GATEWAY_PID" >/dev/null 2>&1; then
    echo "Gateway process exited early (see $GW_LOG)"
    sed -n '1,160p' "$GW_LOG" || true; tail -n 160 "$GW_LOG" || true
    exit 1
  fi
  sleep 0.05
done
[ "$tcp_up" -eq 1 ] || { echo "Gateway never accepted HTTP connections (see $GW_LOG)"; tail -n 200 "$GW_LOG" || true; exit 1; }
log "[*] Gateway is accepting at http://$BIND_HOST:$PORT"

# ---------------- Verify over HTTP (with retries) ----------------
make_url() {
  local addr="$1" rel="$2"
  is_valid_addr "$addr" || return 1
  printf "http://%s:%s/o/%s/%s" "$BIND_HOST" "$PORT" "$addr" "$rel"
}

try_get() {
  local url="$1" label="$2"
  for _ in $(seq 1 40); do
    code="$(curl -s -o /dev/null -w "%{http_code}" "$url" || echo 000)"
    [[ "$code" =~ ^2[0-9][0-9]$ ]] && { log "[*] $label OK ($code)"; return 0; }
    sleep 0.1
  done
  echo "[!] $label failed: $url"
  tail -n 120 "$GW_LOG" || true
  tail -n 120 "$OVL_LOG" || true
  return 1
}

MP_POST="$(make_url "$ADDR_POST" "Manifest.toml" || true)"
if [ -z "$MP_POST" ]; then
  echo "[!] Internal error: invalid .post address: '$ADDR_POST'"; exit 1
fi
log "[*] GET $MP_POST"
try_get "$MP_POST" "Manifest(post)" || exit 1

if [ -n "$ADDR_IMAGE" ] && is_valid_addr "$ADDR_IMAGE"; then
  MP_IMG="$(make_url "$ADDR_IMAGE" "Manifest.toml")"
  log "[*] GET $MP_IMG"
  try_get "$MP_IMG" "Manifest(img)" || true
else
  echo "[!] Skipping .image HTTP checks (no valid image address from pack)."
fi

# ---------------- Summary ----------------
echo
echo "=== Gateway Test Summary ==="
echo "Gateway   : http://$BIND_HOST:$PORT"
echo "OUT_DIR   : $OUT_DIR"
echo "INDEX_DB  : $INDEX_DB_EFF"
echo "ALGO      : $ALGO"
echo
echo "POST addr : $ADDR_POST"
[ -n "$ADDR_IMAGE" ] && echo "IMAGE addr: $ADDR_IMAGE" || echo "IMAGE addr: (none)"
echo
echo "Manifest (post): $MP_POST"
[ -n "${MP_IMG:-}" ] && echo "Manifest (img) : $MP_IMG" || true
echo
echo "(Gateway logs : $GW_LOG)      (copy: $ARCHIVE_DIR/gateway.log)"
echo "(svc-index    : $IDX_LOG)     (copy: $ARCHIVE_DIR/svc-index.log)"
echo "(svc-storage  : $STO_LOG)     (copy: $ARCHIVE_DIR/svc-storage.log)"
echo "(svc-overlay  : $OVL_LOG)     (copy: $ARCHIVE_DIR/svc-overlay.log)"
