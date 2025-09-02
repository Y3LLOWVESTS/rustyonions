#!/usr/bin/env bash
# Stream a sample tile end-to-end via OAP/1 (tcp+tls) using svc-omnigate and ron-app-sdk/examples/tiles_get.
# Safe to re-run; it (re)creates missing certs and the test tile.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLS_DIR="$ROOT/testing/tls"
TILES_DIR="$ROOT/testing/tiles"
ADDR="127.0.0.1:9443"
ADMIN="127.0.0.1:9096"
TILE_SUBPATH="12/654/1583.webp"           # on disk under testing/tiles/...
CLIENT_TILE_PATH="/12/654/1583.webp"      # client request path relative to TILES_ROOT (leading slash ok)
OUT_FILE="$ROOT/out.webp"
BYTES=200000                               # size of sample tile

log() { printf "\033[1;32m[run]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[err]\033[0m %s\n" "$*" >&2; }

ensure_openssl() {
  command -v openssl >/dev/null 2>&1 || { err "openssl not found"; exit 1; }
}

ensure_tls() {
  mkdir -p "$TLS_DIR"
  # Config files (idempotent)
  if [[ ! -f "$TLS_DIR/openssl-server.cnf" ]]; then
    cat >"$TLS_DIR/openssl-server.cnf" <<'EOF'
[req]
distinguished_name = dn
[dn]

[req_ext]
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1  = 127.0.0.1
EOF
  fi

  if [[ ! -f "$TLS_DIR/server-ext.cnf" ]]; then
    cat >"$TLS_DIR/server-ext.cnf" <<'EOF'
authorityKeyIdentifier=keyid,issuer
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=@alt_names

[alt_names]
DNS.1=localhost
IP.1=127.0.0.1
EOF
  fi

  # Dev CA
  if [[ ! -f "$TLS_DIR/ca.key" || ! -f "$TLS_DIR/ca.crt" ]]; then
    log "creating dev CA"
    openssl genrsa -out "$TLS_DIR/ca.key" 2048 >/dev/null 2>&1
    openssl req -x509 -new -key "$TLS_DIR/ca.key" -sha256 -days 3650 \
      -out "$TLS_DIR/ca.crt" -subj "/CN=rustyonions-dev-ca" >/dev/null 2>&1
  fi

  # Server leaf
  if [[ ! -f "$TLS_DIR/server.key" || ! -f "$TLS_DIR/server.crt" ]]; then
    log "creating server cert (localhost, 127.0.0.1)"
    openssl genrsa -out "$TLS_DIR/server.key" 2048 >/dev/null 2>&1
    openssl req -new -key "$TLS_DIR/server.key" -out "$TLS_DIR/server.csr" \
      -subj "/CN=localhost" -config "$TLS_DIR/openssl-server.cnf" >/dev/null 2>&1
    openssl x509 -req -in "$TLS_DIR/server.csr" -CA "$TLS_DIR/ca.crt" -CAkey "$TLS_DIR/ca.key" \
      -CAcreateserial -out "$TLS_DIR/server.crt" -days 825 -sha256 \
      -extfile "$TLS_DIR/server-ext.cnf" >/dev/null 2>&1
  fi
}

ensure_tile() {
  local path="$TILES_DIR/$TILE_SUBPATH"
  if [[ ! -f "$path" ]]; then
    log "creating sample tile: $path ($BYTES bytes)"
    mkdir -p "$(dirname "$path")"
    head -c "$BYTES" /dev/urandom > "$path"
  fi
}

wait_ready() {
  local tries=40
  for ((i=1;i<=tries;i++)); do
    if curl -s --max-time 1 "http://$ADMIN/healthz" >/dev/null; then
      if curl -s --max-time 1 "http://$ADMIN/readyz" >/dev/null; then
        return 0
      fi
    fi
    sleep 0.25
  done
  return 1
}

start_server() {
  log "starting svc-omnigate on $ADDR (admin $ADMIN)"
  export OVERLAY_ADDR="$ADDR"
  export ADMIN_ADDR="$ADMIN"
  export TILES_ROOT="$TILES_DIR"
  export CERT_PEM="$TLS_DIR/server.crt"
  export KEY_PEM="$TLS_DIR/server.key"

  # Run in background; kill on exit.
  set +e
  (cd "$ROOT" && cargo run -p svc-omnigate) &
  SERVER_PID=$!
  set -e
  trap 'log "stopping server"; kill $SERVER_PID >/dev/null 2>&1 || true; wait $SERVER_PID 2>/dev/null || true' EXIT

  if ! wait_ready; then
    err "server not ready at $ADMIN"
    exit 1
  fi
  log "server is ready"
}

run_client() {
  log "running tiles_get example (saving to $OUT_FILE)"
  RON_ADDR="$ADDR" \
  RON_SNI="localhost" \
  RON_EXTRA_CA="$TLS_DIR/ca.crt" \
  TILE_PATH="/$CLIENT_TILE_PATH" \
  OUT="$OUT_FILE" \
  cargo run -p ron-app-sdk --example tiles_get
}

show_metrics() {
  log "healthz:"
  curl -i "http://$ADMIN/healthz" || true
  echo
  log "readyz:"
  curl -i "http://$ADMIN/readyz" || true
  echo
  log "metrics:"
  curl -s "http://$ADMIN/metrics" || true
  echo
}

main() {
  ensure_openssl
  ensure_tls
  ensure_tile
  start_server
  run_client
  show_metrics

  # Verify we didn’t save a tiny JSON error
  if [[ -f "$OUT_FILE" ]]; then
    sz=$(wc -c < "$OUT_FILE" | tr -d ' ')
    if (( sz <= 30 )); then
      err "output file is very small ($sz bytes) — likely an error payload (e.g., not_found). Check paths."
      exit 2
    else
      log "stream succeeded: $OUT_FILE ($sz bytes)"
    fi
  else
    err "no output file produced"
    exit 3
  fi
}

main "$@"
