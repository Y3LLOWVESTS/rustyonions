#!/usr/bin/env bash
set -euo pipefail

tag="[ron-e2e]"
say(){ echo -e "${tag} $*"; }

# ---- Config knobs (env overrides) -------------------------------------------
NODE_BIN="${NODE_BIN:-target/debug/node}"
BIND_ADDR="${BIND_ADDR:-127.0.0.1:1777}"

# Use an existing system Tor (1) or spawn a disposable Tor (0)
TOR_USE_SYSTEM="${TOR_USE_SYSTEM:-0}"

# If spawning: choose/randomize ports (you may also preset these)
TOR_SOCKS_PORT="${TOR_SOCKS_PORT:-0}"   # 0 = random
TOR_CTRL_PORT="${TOR_CTRL_PORT:-0}"     # 0 = random

# Bootstrap + HS publish timeouts
E2E_TIMEOUT="${E2E_TIMEOUT:-240}"       # seconds to wait for bootstrap
HS_WAIT_SECS="${HS_WAIT_SECS:-120}"     # seconds to wait for HS_DESC UPLOADED

# Bridges: provide either TOR_BRIDGES_INLINE (string with lines), or TOR_BRIDGES_FILE (path)
: "${TOR_BRIDGES_INLINE:=}"
: "${TOR_BRIDGES_FILE:=}"

require() { command -v "$1" >/dev/null 2>&1 || { echo "Missing dependency: $1"; exit 1; }; }

# ---- Utility: random free port ----------------------------------------------
random_port() {
  python3 - "$@" <<'PY'
import socket, sys
s=socket.socket(); s.bind(("127.0.0.1",0))
print(s.getsockname()[1]); s.close()
PY
}

# ---- Start server ------------------------------------------------------------
start_server() {
  say "[*] Using binary: $NODE_BIN"
  require "$NODE_BIN" || true

  say "[*] Starting local server: $BIND_ADDR"
  RUST_LOG="${RUST_LOG:-info}" "$NODE_BIN" serve --bind "$BIND_ADDR" --transport tcp \
    >"$SERVER_LOG" 2>&1 &
  SERVER_PID=$!

  say "[*] Waiting for server to listen…"
  for i in {1..60}; do
    nc -z 127.0.0.1 "${BIND_ADDR##*:}" && return 0
    sleep 0.25
  done
  echo "[!] Server failed to listen:" >&2
  echo "----- server.log -----" >&2
  sed -e 's/^/[server] /' "$SERVER_LOG" >&2 || true
  exit 1
}

# ---- Tor bootstrap via control port -----------------------------------------
wait_bootstrap_ctrl() {
  local ctrl="$1" timeout="$2"
  local start ts
  start=$(date +%s)
  while :; do
    ts=$( (printf 'GETINFO status/bootstrap-phase\r\nQUIT\r\n' | nc -w 3 127.0.0.1 "$ctrl") || true )
    if echo "$ts" | grep -q 'PROGRESS=100'; then
      say "[*] Tor bootstrapped."
      return 0
    fi
    if (( $(date +%s) - start > timeout )); then
      echo "TIMEOUT" >&2
      return 1
    fi
    sleep 2
  done
}

# ---- Spawn Tor or reuse system Tor ------------------------------------------
start_tor() {
  if [[ "$TOR_USE_SYSTEM" == "1" ]]; then
    # Reuse system Tor listeners (assumes SocksPort/ControlPort already running).
    say "[*] Reusing system Tor"
    # Try common control ports; override with TOR_CTRL_PORT if you know it.
    if [[ "$TOR_CTRL_PORT" == "0" ]]; then
      for p in 9051 9151 9150 9152; do
        if nc -z 127.0.0.1 "$p" 2>/dev/null; then
          TOR_CTRL_PORT="$p"; break
        fi
      done
    fi
    if [[ "$TOR_CTRL_PORT" == "0" ]]; then
      echo "[!] Could not find a running Tor control port; set TOR_CTRL_PORT." >&2
      exit 1
    fi
    # Guess a SocksPort if not set
    if [[ "$TOR_SOCKS_PORT" == "0" ]]; then
      for p in 9050 9150; do
        if nc -z 127.0.0.1 "$p" 2>/dev/null; then
          TOR_SOCKS_PORT="$p"; break
        fi
      done
    fi
    if [[ "$TOR_SOCKS_PORT" == "0" ]]; then
      echo "[!] Could not find a running Tor SocksPort; set TOR_SOCKS_PORT." >&2
      exit 1
    fi
    wait_bootstrap_ctrl "$TOR_CTRL_PORT" "$E2E_TIMEOUT" || exit 1
    return 0
  fi

  # Spawn a temporary Tor
  TOR_DIR="$(mktemp -d -t ron_onion_e2e.XXXXXX)"
  TOR_LOG="$TOR_DIR/tor.log"
  mkdir -p "$TOR_DIR/tor_data"
  chmod 700 "$TOR_DIR/tor_data"

  [[ "$TOR_SOCKS_PORT" == "0" ]] && TOR_SOCKS_PORT="$(random_port)"
  [[ "$TOR_CTRL_PORT" == "0" ]] && TOR_CTRL_PORT="$(random_port)"

  say "[*] Starting Tor (Socks=$TOR_SOCKS_PORT, Control=$TOR_CTRL_PORT)…"

  # Build torrc
  {
    echo "Log notice file $TOR_LOG"
    echo "SocksPort 127.0.0.1:$TOR_SOCKS_PORT"
    echo "ControlPort 127.0.0.1:$TOR_CTRL_PORT"
    echo "DataDirectory $TOR_DIR/tor_data"
    # Optional bridges
    if [[ -n "$TOR_BRIDGES_INLINE" || -n "${TOR_BRIDGES_FILE:-}" ]]; then
      echo "UseBridges 1"
      if [[ -n "$TOR_BRIDGES_INLINE" ]]; then
        echo "$TOR_BRIDGES_INLINE" | sed 's/^[[:space:]]*$/#/; s/^/Bridge /'
      fi
      if [[ -n "${TOR_BRIDGES_FILE:-}" && -f "$TOR_BRIDGES_FILE" ]]; then
        sed 's/^/Bridge /' "$TOR_BRIDGES_FILE"
      fi
      echo "ClientTransportPlugin obfs4 exec /usr/local/bin/obfs4proxy || /opt/homebrew/bin/obfs4proxy || /usr/bin/obfs4proxy"
    fi
  } > "$TOR_DIR/torrc"

  tor -f "$TOR_DIR/torrc" >/dev/null 2>&1 &
  TOR_PID=$!

  say "[*] Waiting for Tor bootstrap (timeout ${E2E_TIMEOUT}s)…"
  if ! wait_bootstrap_ctrl "$TOR_CTRL_PORT" "$E2E_TIMEOUT"; then
    echo "----- tor.log (tail) -----" >&2
    sed -e 's/^/[tor] /' "$TOR_LOG" | tail -n 200 >&2 || true
    exit 1
  fi
}

# ---- Control: create onion + wait HS_DESC UPLOADED ---------------------------
create_onion_and_wait() {
  local ctrl="$1" listen_port="$2" hs_wait="$3"
  local CTRL_IN="$WORK_DIR/ctrl.in"
  local CTRL_OUT="$WORK_DIR/ctrl.out"
  mkfifo "$CTRL_IN" "$CTRL_OUT"

  # One persistent control connection via nc
  ( nc 127.0.0.1 "$ctrl" <"$CTRL_IN" >"$CTRL_OUT" ) &
  local NC_PID=$!

  # Authenticate (no password by default; set PW env to use one)
  if [[ -n "${PW:-}" ]]; then
    printf 'AUTHENTICATE "%s"\r\n' "$PW" >"$CTRL_IN"
  else
    printf 'AUTHENTICATE\r\n' >"$CTRL_IN"
  fi

  # Subscribe to HS_DESC events BEFORE creating onion
  printf 'SETEVENTS HS_DESC\r\n' >"$CTRL_IN"

  # Create ephemeral onion
  printf 'ADD_ONION NEW:ED25519-V3 Flags=DiscardPK Port=%d,127.0.0.1:%d\r\n' \
    "$listen_port" "$listen_port" >"$CTRL_IN"

  # Parse service id from the streaming response
  local sid=""
  local start=$(date +%s)
  while read -r -t 2 line <"$CTRL_OUT"; do
    [[ -z "$line" ]] && continue
    if [[ "$line" =~ ^250-ServiceID=([a-z0-9]+)$ ]]; then
      sid="${BASH_REMATCH[1]}"
      echo "$line"
      break
    fi
    echo "$line"
    if (( $(date +%s) - start > 15 )); then
      echo "[!] Timed out waiting ADD_ONION reply" >&2
      break
    fi
  done

  if [[ -z "$sid" ]]; then
    echo "[!] Failed to parse ServiceID from control response." >&2
    # dump residual
    timeout 1 cat "$CTRL_OUT" || true
    kill "$NC_PID" 2>/dev/null || true
    return 1
  fi

  echo "$sid" > "$WORK_DIR/service_id"
  say "[+] Onion created: ${sid}.onion:${listen_port}"

  # Wait for HS_DESC UPLOADED for this sid
  say "[*] Waiting for HS_DESC UPLOADED (timeout ${hs_wait}s)…"
  local deadline=$(( $(date +%s) + hs_wait ))
  while :; do
    if read -r -t 5 line <"$CTRL_OUT"; then
      echo "$line"
      if [[ "$line" =~ ^650\ HS_DESC\ UPLOADED\ ([a-z0-9]+) ]]; then
        if [[ "${BASH_REMATCH[1]}" == "$sid" ]]; then
          say "[+] HS_DESC UPLOADED confirmed for $sid"
          printf 'QUIT\r\n' >"$CTRL_IN" || true
          kill "$NC_PID" 2>/dev/null || true
          return 0
        fi
      fi
    fi
    if (( $(date +%s) > deadline )); then
      echo "[!] Timed out waiting for HS_DESC UPLOADED." >&2
      echo "----- control (ADD_ONION) response -----" >&2
      printf 'QUIT\r\n' >"$CTRL_IN" || true
      timeout 1 cat "$CTRL_OUT" >&2 || true
      kill "$NC_PID" 2>/dev/null || true
      return 1
    fi
  done
}

# ---- Client PUT/GET via torsocks --------------------------------------------
client_put_get() {
  require torsocks
  local sid file put_out get_out
  sid="$(cat "$WORK_DIR/service_id")"
  file="$WORK_DIR/payload.txt"
  put_out="$WORK_DIR/put.out"
  get_out="$WORK_DIR/get.out"
  echo "hello_rusty_onions_$(date +%s)" > "$file"

  say "[*] PUT over onion…"
  if ! torsocks -o TorPort=127.0.0.1:"$TOR_SOCKS_PORT" \
      "$NODE_BIN" put --to "${sid}.onion:${BIND_ADDR##*:}" --transport tcp "$file" \
      >"$put_out" 2>&1; then
    echo "[!] PUT failed" >&2
    return 1
  fi

  # Expect the last line to be the hash
  local hash
  hash="$(tail -n1 "$put_out" | tr -d '\r\n' | sed 's/.* //')"
  if [[ -z "$hash" ]]; then
    echo "[!] Could not parse hash from PUT output" >&2
    cat "$put_out" >&2
    return 1
  fi
  echo "$hash" > "$WORK_DIR/hash"

  say "[*] GET over onion…"
  if ! torsocks -o TorPort=127.0.0.1:"$TOR_SOCKS_PORT" \
      "$NODE_BIN" get --from "${sid}.onion:${BIND_ADDR##*:}" --transport tcp "$hash" \
      >"$get_out" 2>&1; then
    echo "[!] GET failed" >&2
    return 1
  fi

  local got_path
  got_path="$(grep -Eo 'wrote to .*$' "$get_out" | sed 's/wrote to //')"
  if [[ -z "$got_path" || ! -f "$got_path" ]]; then
    echo "[!] Could not find downloaded file path in GET output" >&2
    cat "$get_out" >&2
    return 1
  fi

  if cmp -s "$file" "$got_path"; then
    say "[✓] E2E OK — payload verified"
    return 0
  else
    echo "[!] Payload mismatch" >&2
    return 1
  fi
}

# ---- Cleanup ----------------------------------------------------------------
cleanup() {
  [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" 2>/dev/null || true
  [[ -n "${TOR_PID:-}"    ]] && kill "$TOR_PID"    2>/dev/null || true
}
trap cleanup EXIT

# ---- Main -------------------------------------------------------------------
WORK_DIR="$(mktemp -d -t ron_onion_e2e.XXXXXX)"
SERVER_LOG="$WORK_DIR/server.log"
say "[*] Working dir: $WORK_DIR"

start_server
start_tor

# If we spawned Tor, TOR_SOCKS_PORT/TOR_CTRL_PORT are set. For system Tor, TOR_SOCKS_PORT/TOR_CTRL_PORT must have been found above.
if ! create_onion_and_wait "$TOR_CTRL_PORT" "${BIND_ADDR##*:}" "$HS_WAIT_SECS"; then
  echo "----- tor.log tail -----" >&2
  [[ -n "${TOR_LOG:-}" && -f "${TOR_LOG:-}" ]] && sed -e 's/^/[tor] /' "$TOR_LOG" | tail -n 200 >&2 || true
  exit 1
fi

if ! client_put_get; then
  echo "----- server.log -----" >&2
  sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n 200 >&2 || true
  echo "----- put.out -----" >&2
  [[ -f "$WORK_DIR/put.out" ]] && sed -e 's/^/[put] /' "$WORK_DIR/put.out" >&2 || true
  echo "----- get.out -----" >&2
  [[ -f "$WORK_DIR/get.out" ]] && sed -e 's/^/[get] /' "$WORK_DIR/get.out" >&2 || true
  exit 1
fi

say "[*] Cleaning up…"
exit 0
