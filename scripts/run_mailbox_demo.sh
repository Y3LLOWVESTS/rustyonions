#!/usr/bin/env bash
# Mailbox MVP end-to-end demo WITH BUILD and FORCE_RESTART support.
# - Builds binaries up-front
# - Generates local TLS under testing/tls if missing
# - By default reuses a healthy existing server
# - Set FORCE_RESTART=1 to kill any existing server on the port and start fresh
# - Sends two idempotent messages (same msg_id), receives & ACKs, shows metrics

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TLS_DIR="$ROOT/testing/tls"
ADDR="127.0.0.1:9443"
ADMIN="127.0.0.1:9096"
TOPIC="${TOPIC:-chat}"
TEXT1="${TEXT1:-hello vest #1}"
TEXT2="${TEXT2:-hello vest #1 again}"
IDEMPOTENCY_KEY="${IDEMPOTENCY_KEY:-demo-idem-1}"
FORCE_RESTART="${FORCE_RESTART:-0}"

BIN_SERVER="$ROOT/target/debug/svc-omnigate"
BIN_SEND="$ROOT/target/debug/examples/mailbox_send"
BIN_RECV="$ROOT/target/debug/examples/mailbox_recv"

log() { printf "\033[1;32m[mailbox]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[error]\033[0m %s\n" "$*" >&2; }

ensure_openssl() {
  command -v openssl >/dev/null 2>&1 || { err "openssl not found in PATH"; exit 1; }
}

ensure_tls() {
  mkdir -p "$TLS_DIR"
  [[ -f "$TLS_DIR/openssl-server.cnf" ]] || cat >"$TLS_DIR/openssl-server.cnf" <<'EOF'
[req]
distinguished_name = dn
[dn]

[req_ext]
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1  = 127.0.0.1
EOF
  [[ -f "$TLS_DIR/server-ext.cnf" ]] || cat >"$TLS_DIR/server-ext.cnf" <<'EOF'
authorityKeyIdentifier=keyid,issuer
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=@alt_names

[alt_names]
DNS.1=localhost
IP.1=127.0.0.1
EOF
  if [[ ! -f "$TLS_DIR/ca.key" || ! -f "$TLS_DIR/ca.crt" ]]; then
    log "creating dev CA"
    openssl genrsa -out "$TLS_DIR/ca.key" 2048 >/dev/null 2>&1
    openssl req -x509 -new -key "$TLS_DIR/ca.key" -sha256 -days 3650 \
      -out "$TLS_DIR/ca.crt" -subj "/CN=rustyonions-dev-ca" >/dev/null 2>&1
  fi
  if [[ ! -f "$TLS_DIR/server.key" || ! -f "$TLS_DIR/server.crt" ]]; then
    log "creating localhost server certificate"
    openssl genrsa -out "$TLS_DIR/server.key" 2048 >/dev/null 2>&1
    openssl req -new -key "$TLS_DIR/server.key" -out "$TLS_DIR/server.csr" \
      -subj "/CN=localhost" -config "$TLS_DIR/openssl-server.cnf" >/dev/null 2>&1
    openssl x509 -req -in "$TLS_DIR/server.csr" -CA "$TLS_DIR/ca.crt" -CAkey "$TLS_DIR/ca.key" \
      -CAcreateserial -out "$TLS_DIR/server.crt" -days 825 -sha256 \
      -extfile "$TLS_DIR/server-ext.cnf" >/dev/null 2>&1
  fi
}

build_binaries() {
  log "building svc-omnigate and ron-app-sdk examples (one-time, up-front)"
  cargo build -p svc-omnigate -p ron-app-sdk --examples
  [[ -x "$BIN_SERVER" ]] || { err "build did not produce $BIN_SERVER"; exit 1; }
  [[ -x "$BIN_SEND"   ]] || { err "build did not produce $BIN_SEND";   exit 1; }
  [[ -x "$BIN_RECV"   ]] || { err "build did not produce $BIN_RECV";   exit 1; }
}

port_in_use() {
  local port="$1"
  if command -v lsof >/dev/null 2>&1; then
    lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1
  else
    nc -z 127.0.0.1 "$port" >/dev/null 2>&1
  fi
}

pids_on_port() {
  local port="$1"
  if command -v lsof >/dev/null 2>&1; then
    lsof -nP -tiTCP:"$port" -sTCP:LISTEN || true
  else
    # Fallback best-effort
    ps aux | grep "svc-omnigate" | grep -v grep | awk '{print $2}'
  fi
}

wait_port_free() {
  local port="$1" tries=40
  for ((i=1;i<=tries;i++)); do
    if ! port_in_use "$port"; then return 0; fi
    sleep 0.25
  done
  return 1
}

health_ready_ok() {
  curl -s --max-time 1 "http://$ADMIN/healthz" >/dev/null 2>&1 &&
  curl -s --max-time 1 "http://$ADMIN/readyz"  >/dev/null 2>&1
}

wait_ready() {
  local tries=80
  for ((i=1;i<=tries;i++)); do
    health_ready_ok && return 0
    sleep 0.25
  done
  return 1
}

kill_existing_if_forced() {
  if [[ "$FORCE_RESTART" == "1" ]]; then
    local p1 p2
    p1="$(pids_on_port "${ADDR##*:}")"
    p2="$(pids_on_port "${ADMIN##*:}")"
    if [[ -n "${p1:-}" || -n "${p2:-}" ]]; then
      log "FORCE_RESTART=1 → stopping existing server on $ADDR/$ADMIN"
      [[ -n "${p1:-}" ]] && kill $p1 2>/dev/null || true
      [[ -n "${p2:-}" ]] && kill $p2 2>/dev/null || true
      wait_port_free "${ADDR##*:}" || { err "port $ADDR still busy"; exit 1; }
      wait_port_free "${ADMIN##*:}" || { err "port $ADMIN still busy"; exit 1; }
    fi
  fi
}

EXTERNAL_SERVER=0
start_server() {
  log "starting svc-omnigate on $ADDR (admin $ADMIN)"
  export OVERLAY_ADDR="$ADDR"
  export ADMIN_ADDR="$ADMIN"
  export TILES_ROOT="$ROOT/testing/tiles"   # not used by mailbox; harmless default
  export CERT_PEM="$TLS_DIR/server.crt"
  export KEY_PEM="$TLS_DIR/server.key"
  mkdir -p "$ROOT/testing/tiles"

  if port_in_use "${ADDR##*:}"; then
    if [[ "$FORCE_RESTART" == "1" ]]; then
      kill_existing_if_forced
    else
      if wait_ready; then
        EXTERNAL_SERVER=1
        log "existing server detected and healthy; reusing it"
        return 0
      else
        err "port $ADDR in use but admin not healthy; set FORCE_RESTART=1 or stop the other process"
        exit 1
      fi
    fi
  fi

  set +e
  "$BIN_SERVER" &
  SERVER_PID=$!
  set -e
  trap 'if [[ "$EXTERNAL_SERVER" -eq 0 ]]; then log "stopping server"; kill $SERVER_PID >/dev/null 2>&1 || true; wait $SERVER_PID 2>/dev/null || true; fi' EXIT

  if ! wait_ready; then
    err "server not ready at $ADMIN"
    exit 1
  fi
  log "server is ready"
}

send_once() {
  local text="$1"
  local idem="$2"
  RON_ADDR="$ADDR" RON_SNI=localhost RON_EXTRA_CA="$TLS_DIR/ca.crt" \
  TOPIC="$TOPIC" TEXT="$text" IDEMPOTENCY_KEY="$idem" \
  "$BIN_SEND"
}

recv_and_ack() {
  RON_ADDR="$ADDR" RON_SNI=localhost RON_EXTRA_CA="$TLS_DIR/ca.crt" \
  TOPIC="$TOPIC" MAX="${1:-10}" \
  "$BIN_RECV"
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
  build_binaries
  kill_existing_if_forced
  start_server

  log "send #1 (idempotent key: $IDEMPOTENCY_KEY)"
  out1="$(send_once "$TEXT1" "$IDEMPOTENCY_KEY" || true)"
  echo "$out1"
  id1="$(echo "$out1" | awk '/^msg_id:/ {print $2}')"
  [[ -n "${id1:-}" ]] || { err "failed to parse msg_id from first send"; exit 2; }

  log "send #2 (same idempotent key; should return SAME msg_id)"
  out2="$(send_once "$TEXT2" "$IDEMPOTENCY_KEY" || true)"
  echo "$out2"
  id2="$(echo "$out2" | awk '/^msg_id:/ {print $2}')"
  [[ -n "${id2:-}" ]] || { err "failed to parse msg_id from second send"; exit 3; }

  if [[ "$id1" != "$id2" ]]; then
    err "idempotency failed: ids differ: $id1 vs $id2"
    exit 4
  else
    log "idempotency OK (msg_id $id1)"
  fi

  log "receive & ACK (should include exactly one message for the topic)"
  recv_and_ack 10 || true

  log "second receive (should be empty)"
  recv_and_ack 10 || true

  show_metrics
}

main "$@"
