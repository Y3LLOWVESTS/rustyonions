#!/usr/bin/env bash
# RustyOnions — Gateway smoke test (HTTP gateway -> OAP storage via svc-omnigate)
# Packs+indexes bundles FIRST, then starts svc-omnigate (OAP) and the HTTP gateway.

set -euo pipefail

# ---------------- Config ----------------
BIND_HOST="${BIND_HOST:-127.0.0.1}"
PORT="${PORT:-0}"                     # HTTP gateway port (0 = auto-pick)
OUT_DIR="${OUT_DIR:-.onions}"         # bundle root for bytes
INDEX_DB="${INDEX_DB:-.data/index}"   # index db path
ALGO="${ALGO:-blake3}"

# OAP listen base (host part); port is optional (auto-picked if omitted)
OAP_HOST="${OAP_HOST:-127.0.0.1}"
OAP_ADDR="${OAP_ADDR:-}"              # if empty we try to pick a free port; we will still parse the effective bind from logs
METRICS_ADDR="${METRICS_ADDR:-127.0.0.1:9909}" # svc-omnigate metrics

# Tunables
OAP_WAIT_SEC="${OAP_WAIT_SEC:-30}"    # wait up to N seconds for OAP to accept
HTTP_WAIT_SEC="${HTTP_WAIT_SEC:-15}"  # wait up to N seconds for HTTP to accept
QUIET="${QUIET:-0}"
KEEP_TMP="${KEEP_TMP:-0}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLDCTL="$ROOT_DIR/target/debug/tldctl"
GATEWAY="$ROOT_DIR/target/debug/gateway"
OMNI="$ROOT_DIR/target/debug/svc-omnigate"

log() { [ "$QUIET" = "1" ] && return 0; echo -e "$@"; }
need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1" >&2; exit 1; }; }

# ---------------- Preflight ----------------
[ -f "$ROOT_DIR/Cargo.toml" ] || { echo "Run inside repo (no Cargo.toml)"; exit 1; }
need curl
mkdir -p "$OUT_DIR" "$INDEX_DB"

log "[*] Building gateway, tldctl, and svc-omnigate…"
cargo build -q -p tldctl -p gateway -p svc-omnigate

TMP_DIR="$(mktemp -d -t ron_gateway.XXXXXX)"
trap '
  EC=$?
  if [ -n "${GATEWAY_PID:-}" ]; then kill "$GATEWAY_PID" >/dev/null 2>&1 || true; fi
  if [ -n "${OAPD_PID:-}" ]; then kill "$OAPD_PID" >/dev/null 2>&1 || true; fi
  if [ "$KEEP_TMP" = "1" ] || [ $EC -ne 0 ]; then
    echo "(Keeping TMP_DIR: $TMP_DIR)"
  else
    rm -rf "$TMP_DIR"
  fi
  echo "(Gateway logs: $TMP_DIR/gateway.log)"
  echo "(OAP logs    : $TMP_DIR/oapd.log)"
  exit $EC
' EXIT

# ---------------- Sample payloads ----------------
POST_TXT="$TMP_DIR/post.txt"
IMG_PNG="$TMP_DIR/pixel.png"
echo "Hello from RustyOnions gateway test (.post)" > "$POST_TXT"
base64 -d >"$IMG_PNG" <<'B64'
iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGMAAQAABQAB
JzQnWAAAAABJRU5ErkJggg==
B64

# ---------------- tldctl compatibility probe ----------------
PACK_HELP="$("$TLDCTL" pack --help 2>&1 || true)"
PACK_HAS_OUT=0
PACK_HAS_INDEX_DB=0
echo "$PACK_HELP" | grep -q -- '--out'       && PACK_HAS_OUT=1
echo "$PACK_HELP" | grep -q -- '--index-db'  && PACK_HAS_INDEX_DB=1

# Build common arg sets (NOTE: flags go *after* `pack`)
PACK_COMMON_POST=( pack --file "$POST_TXT" --tld post  --algo "$ALGO" --index )
PACK_COMMON_IMG=(  pack --file "$IMG_PNG"  --tld image --algo "$ALGO" --index )

# Append optional flags behind the subcommand if supported
if [ "$PACK_HAS_OUT" -eq 1 ]; then
  PACK_COMMON_POST+=( --out "$OUT_DIR" )
  PACK_COMMON_IMG+=(  --out "$OUT_DIR"  )
fi
if [ "$PACK_HAS_INDEX_DB" -eq 1 ]; then
  PACK_COMMON_POST+=( --index-db "$INDEX_DB" )
  PACK_COMMON_IMG+=(  --index-db "$INDEX_DB"  )
fi

# Helper: resolve address from stdout or by scanning OUT_DIR
resolve_addr() {
  local tld="$1" stdout_file="$2"
  local addr
  addr="$(sed -n "s#^OK: .*\/\\([^/]*\\)\\.${tld}/Manifest\\.toml\$#\\1.${tld}#p" "$stdout_file" || true)"
  if [ -n "$addr" ]; then echo "$addr"; return 0; fi
  local latest
  latest="$(ls -t "$OUT_DIR"/*."$tld"/Manifest.toml 2>/dev/null | head -n1 || true)"
  if [ -n "$latest" ]; then basename "$(dirname "$latest")"; return 0; fi
  return 1
}

# ---------------- Pack + index (do this BEFORE servers start) ----------------
log "[*] Creating bundles (pack + index)…"
POST_OUT="$TMP_DIR/pack_post.out"
IMG_OUT="$TMP_DIR/pack_img.out"

"$TLDCTL" "${PACK_COMMON_POST[@]}" >"$POST_OUT" 2>&1 || true
"$TLDCTL" "${PACK_COMMON_IMG[@]}"  >"$IMG_OUT"  2>&1 || true

ADDR_POST="$(resolve_addr post  "$POST_OUT" || true)"
ADDR_IMAGE="$(resolve_addr image "$IMG_OUT"  || true)"

if [ -z "$ADDR_POST" ] || [ -z "$ADDR_IMAGE" ]; then
  echo "[!] Could not resolve addresses from pack output or $OUT_DIR/"
  echo "    pack(post) stdout:";  sed -n '1,80p' "$POST_OUT" || true
  echo "    pack(image) stdout:"; sed -n '1,80p' "$IMG_OUT"  || true
  exit 1
fi

log "    .post  → $ADDR_POST"
log "    .image → $ADDR_IMAGE"

# ---------------- svc-omnigate TLS & listen handling ----------------
OMNI_HELP="$("$OMNI" --help 2>&1 || true)"

# If caller didn't pass a full OAP_ADDR, pick a free port (we'll still parse logs for the actual bind)
if [ -z "$OAP_ADDR" ]; then
  OAP_PORT="$(python3 - <<'PY' || true
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"
  [ -n "$OAP_PORT" ] || OAP_PORT=9444
  OAP_ADDR="$OAP_HOST:$OAP_PORT"
fi

OMNI_ARGS=( --root "$OUT_DIR" --index-db "$INDEX_DB" --metrics "$METRICS_ADDR" )
OAP_TLS=1  # assume TLS unless we can disable

# Detect listen flag variant
if echo "$OMNI_HELP" | grep -q -- '--oap-listen'; then
  OMNI_ARGS+=( --oap-listen "$OAP_ADDR" )
elif echo "$OMNI_HELP" | grep -q -- '--oap'; then
  OMNI_ARGS+=( --oap "$OAP_ADDR" )
else
  export OAP_LISTEN="$OAP_ADDR"
  export RON_OAP_LISTEN="$OAP_ADDR"
fi

# TLS: prefer plaintext if supported, else generate self-signed and pass PATHS
if echo "$OMNI_HELP" | grep -q -- '--no-tls'; then
  OMNI_ARGS+=( --no-tls )
  OAP_TLS=0
else
  need openssl
  CERT="$TMP_DIR/cert.pem"
  KEY="$TMP_DIR/key.pem"
  openssl req -x509 -newkey rsa:2048 -nodes -days 2 \
    -subj "/CN=localhost" -keyout "$KEY" -out "$CERT" >/dev/null 2>&1

  if echo "$OMNI_HELP" | grep -q -- '--tls-cert'; then
    OMNI_ARGS+=( --tls-cert "$CERT" --tls-key "$KEY" )
  else
    # Env variants that expect FILE PATHS (not contents)
    export CERT_PEM="$CERT"
    export KEY_PEM="$KEY"
    export CERT_FILE="$CERT"
    export KEY_FILE="$KEY"
    export RUSTLS_CERTFILE="$CERT"
    export RUSTLS_KEYFILE="$KEY"
  fi
fi

# ---------------- Start svc-omnigate (OAP server) ----------------
log "[*] Starting svc-omnigate (OAP) on oap://$OAP_ADDR …"
RUST_LOG="${RUST_LOG:-info}" "$OMNI" "${OMNI_ARGS[@]}" > "$TMP_DIR/oapd.log" 2>&1 &
OAPD_PID=$!

# Determine EFFECTIVE OAP address from logs (handles binaries that force 9443/9444)
OAP_ADDR_EFF="$OAP_ADDR"
parse_oap_addr_from_log() {
  awk '
    /OAP listener on / {print $NF; found=1}
    /starting on /     {print $NF; found=1}
  ' "$TMP_DIR/oapd.log" | tail -n1
}

# Wait for OAP TCP accept on the effective address
oap_up=0
OAP_TRIES=$(( OAP_WAIT_SEC * 20 ))   # 50ms steps
for i in $(seq 1 "$OAP_TRIES"); do
  if eff="$(parse_oap_addr_from_log)"; then
    if [ -n "$eff" ]; then OAP_ADDR_EFF="$eff"; fi
  fi
  if bash -c "exec 3<>/dev/tcp/${OAP_ADDR_EFF%:*}/${OAP_ADDR_EFF#*:}" 2>/dev/null; then
    exec 3>&- 3<&- || true
    oap_up=1
    break
  fi
  if ! kill -0 "$OAPD_PID" >/dev/null 2>&1; then
    echo "svc-omnigate exited early (see $TMP_DIR/oapd.log)"
    echo "------ oapd.log (head) ------"; sed -n '1,120p' "$TMP_DIR/oapd.log" || true; echo "------------------------------"
    echo "------ oapd.log (tail) ------"; tail -n 200 "$TMP_DIR/oapd.log" || true; echo "------------------------------"
    exit 1
  fi
  sleep 0.05
done
if [ "$oap_up" -ne 1 ]; then
  echo "svc-omnigate never accepted OAP connections (see $TMP_DIR/oapd.log)"
  echo "  requested : $OAP_ADDR"
  echo "  effective?: $OAP_ADDR_EFF"
  echo "------ oapd.log (head) ------"; sed -n '1,120p' "$TMP_DIR/oapd.log" || true; echo "------------------------------"
  echo "------ oapd.log (tail) ------"; tail -n 200 "$TMP_DIR/oapd.log" || true; echo "------------------------------"
  exit 1
fi
log "[*] OAP is accepting on $OAP_ADDR_EFF"

# ---------------- Start HTTP gateway (points to OAP) ----------------
# Pick a port if PORT=0
if [ "$PORT" = "0" ]; then
  PORT="$(python3 - <<'PY' || true
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"; [ -n "$PORT" ] || PORT=31555
fi

log "[*] Starting HTTP gateway on http://$BIND_HOST:$PORT …"
GW_HELP="$("$GATEWAY" --help 2>&1 || true)"

# Base args
GW_ARGS=( --bind "$BIND_HOST:$PORT" --index-db "$INDEX_DB" )

# Upstream address: include scheme when TLS is on (helps many builds)
UPSTREAM_ADDR="$OAP_ADDR_EFF"
if [ "$OAP_TLS" -eq 1 ]; then
  UPSTREAM_ADDR="oaps://$OAP_ADDR_EFF"
fi

# Provide upstream via flag if available, else by env
if echo "$GW_HELP" | grep -q -- '--oap'; then
  GW_ARGS+=( --oap "$UPSTREAM_ADDR" )
else
  export RON_OAP_UPSTREAM="$UPSTREAM_ADDR"
  export OAP_UPSTREAM="$UPSTREAM_ADDR"
fi

# TLS client configuration for the gateway if OAP is TLS
GW_TLS_ARGS=()
if [ "$OAP_TLS" -eq 1 ]; then
  # Prefer a CA flag if present
  if echo "$GW_HELP" | grep -q -- '--oap-tls-ca'; then
    GW_TLS_ARGS+=( --oap-tls-ca "$CERT" )
  elif echo "$GW_HELP" | grep -q -- '--tls-ca'; then
    GW_TLS_ARGS+=( --tls-ca "$CERT" )
  elif echo "$GW_HELP" | grep -q -- '--ca' && ! echo "$GW_HELP" | grep -q -- '--http'; then
    GW_TLS_ARGS+=( --ca "$CERT" )
  else
    # Try "insecure" style flags (first one that exists)
    if echo "$GW_HELP" | grep -q -- '--oap-insecure'; then
      GW_TLS_ARGS+=( --oap-insecure )
    elif echo "$GW_HELP" | grep -q -- '--oap-tls-no-verify'; then
      GW_TLS_ARGS+=( --oap-tls-no-verify )
    elif echo "$GW_HELP" | grep -q -- '--tls-insecure'; then
      GW_TLS_ARGS+=( --tls-insecure )
    elif echo "$GW_HELP" | grep -q -- '--insecure'; then
      GW_TLS_ARGS+=( --insecure )
    fi
    # And set common env fallbacks (some builds only read env)
    export OAP_TLS_CA="$CERT"
    export RON_OAP_TLS_CA="$CERT"
    export OAP_CA_PEM="$CERT"
    export OAP_TLS_INSECURE=1
  fi
fi

"$GATEWAY" "${GW_ARGS[@]}" "${GW_TLS_ARGS[@]}" > "$TMP_DIR/gateway.log" 2>&1 &
GATEWAY_PID=$!

# ---- Readiness: wait for HTTP TCP accept ----
tcp_up=0
HTTP_TRIES=$(( HTTP_WAIT_SEC * 20 )) # 50ms steps
for i in $(seq 1 "$HTTP_TRIES"); do
  if bash -c "exec 3<>/dev/tcp/$BIND_HOST/$PORT" 2>/dev/null; then
    exec 3>&- 3<&- || true
    tcp_up=1; break
  fi
  if ! kill -0 "$GATEWAY_PID" >/dev/null 2>&1; then
    echo "Gateway process exited early (see $TMP_DIR/gateway.log)"
    echo "------ gateway.log (head) ------"; sed -n '1,120p' "$TMP_DIR/gateway.log" || true; echo "------------------------------"
    echo "------ gateway.log (tail) ------"; tail -n 200 "$TMP_DIR/gateway.log" || true; echo "------------------------------"
    exit 1
  fi
  sleep 0.05
done
[ "$tcp_up" -eq 1 ] || { echo "Gateway never accepted HTTP connections (see $TMP_DIR/gateway.log)"; tail -n 200 "$TMP_DIR/gateway.log" || true; exit 1; }
log "[*] TCP listener is accepting"

# ---------------- Verify over HTTP (with retries) ----------------
MP_POST="http://$BIND_HOST:$PORT/o/$ADDR_POST/Manifest.toml"
MP_IMG="http://$BIND_HOST:$PORT/o/$ADDR_IMAGE/Manifest.toml"
PB_IMG="http://$BIND_HOST:$PORT/o/$ADDR_IMAGE/payload.bin"

try_get() {
  local url="$1" label="$2"
  for i in {1..40}; do
    code="$(curl -s -o /dev/null -w "%{http_code}" "$url" || echo 000)"
    if [[ "$code" =~ ^2[0-9][0-9]$ ]]; then
      log "[*] $label OK ($code)"; return 0
    fi
    sleep 0.1
  done
  echo "[!] $label failed: $url"
  echo "------ gateway.log (tail) ------"; tail -n 200 "$TMP_DIR/gateway.log" || true; echo "------------------------------"
  echo "------ oapd.log (tail) ------"; tail -n 200 "$TMP_DIR/oapd.log" || true; echo "------------------------------"
  curl -v "$url" || true
  return 1
}

try_head() {
  local url="$1" label="$2"
  for i in {1..40}; do
    code="$(curl -sI -o /dev/null -w "%{http_code}" "$url" || echo 000)"
    if [[ "$code" =~ ^2[0-9][0-9]$ ]]; then
      log "[*] $label OK ($code)"; return 0
    fi
    sleep 0.1
  done
  echo "[!] $label failed: $url"
  echo "------ gateway.log (tail) ------"; tail -n 200 "$TMP_DIR/gateway.log" || true; echo "------------------------------"
  echo "------ oapd.log (tail) ------"; tail -n 200 "$TMP_DIR/oapd.log" || true; echo "------------------------------"
  curl -vI "$url" || true
  return 1
}

log "[*] GET $MP_POST"
try_get "$MP_POST" "Manifest(post)"

log "[*] GET $MP_IMG"
try_get "$MP_IMG" "Manifest(img)"

log "[*] HEAD $PB_IMG"
try_head "$PB_IMG" "Payload(img)"

# ---------------- Summary ----------------
echo
echo "=== Gateway Test Summary ==="
echo "Gateway   : http://$BIND_HOST:$PORT"
echo "OAP (eff) : ${UPSTREAM_ADDR}"
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
echo
echo "(Gateway logs: $TMP_DIR/gateway.log)"
echo "(OAP logs    : $TMP_DIR/oapd.log)"
