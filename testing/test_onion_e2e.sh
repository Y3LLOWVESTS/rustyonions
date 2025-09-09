#!/usr/bin/env bash
# testing/test_onion_e2e.sh
# Old-good control-port + socat flow, but with isolated per-run cwd for server/client to avoid sled .data lock.

set -euo pipefail

LOG_PREFIX="[ron-e2e]"
say() { echo -e "${LOG_PREFIX} $*"; }
die() { echo -e "${LOG_PREFIX} [!] $*" >&2; print_paths "error"; exit 1; }

pick_free_port() {
  local p
  for _ in {1..200}; do
    p=$(( ( RANDOM % 20000 ) + 20000 ))
    if ! lsof -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1; then
      echo "$p"; return 0
    fi
  done
  return 1
}

# Will be filled after WORK_DIR is created
SERVER_LOG=""
TOR_LOG=""
CONTROL_DUMP=""
HS_EVENTS=""
PUT_OUT=""
GET_OUT=""
PAYLOAD=""
TUNNEL_PORT_FILE=""
PATHS_TXT=""
BUNDLE_PATH=""

# --- Pretty path summary (always show on exit / errors) ---
print_paths() {
  # $1 = context: "ok" | "error" (optional)
  [[ -z "${WORK_DIR:-}" || ! -d "${WORK_DIR:-}" ]] && return 0

  # Write a tiny index file with the key paths
  PATHS_TXT="$WORK_DIR/paths.txt"
  {
    echo "# RustyOnions onion e2e — key files (run: $(date -u +'%Y-%m-%dT%H:%M:%SZ'))"
    echo "workdir        = $WORK_DIR"
    [[ -n "$SERVER_LOG"    ]] && echo "server_log     = $SERVER_LOG"
    [[ -n "$TOR_LOG"       ]] && echo "tor_log        = $TOR_LOG"
    [[ -n "$CONTROL_DUMP"  ]] && echo "control_reply  = $CONTROL_DUMP"
    [[ -n "$HS_EVENTS"     ]] && echo "hs_events      = $HS_EVENTS"
    [[ -n "$PUT_OUT"       ]] && echo "put_out        = $PUT_OUT"
    [[ -n "$GET_OUT"       ]] && echo "get_out        = $GET_OUT"
  } > "$PATHS_TXT"

  say "[i] ===== Copy these paths ====="
  [[ -n "$SERVER_LOG"   ]] && echo "$SERVER_LOG"
  [[ -n "$TOR_LOG"      ]] && echo "$TOR_LOG"
  [[ -n "$CONTROL_DUMP" ]] && echo "$CONTROL_DUMP"
  [[ -n "$HS_EVENTS"    ]] && echo "$HS_EVENTS"
  [[ -n "$PUT_OUT"      ]] && echo "$PUT_OUT"
  [[ -n "$GET_OUT"      ]] && echo "$GET_OUT"
  echo "$PATHS_TXT"
  say "[i] ============================"

  if [[ "${BUNDLE_LOGS:-0}" == "1" ]]; then
    BUNDLE_PATH="$WORK_DIR/logs_bundle.tar.gz"
    tar -C "$WORK_DIR" -czf "$BUNDLE_PATH" \
      "$(basename "$SERVER_LOG")" \
      "$(basename "$TOR_LOG")" \
      "$(basename "$CONTROL_DUMP")" \
      "$(basename "$HS_EVENTS")" \
      "$(basename "$PUT_OUT")" \
      "$(basename "$GET_OUT")" \
      "$(basename "$PATHS_TXT")" >/dev/null 2>&1 || true
    [[ -f "$BUNDLE_PATH" ]] && say "[i] Log bundle: $BUNDLE_PATH"
  fi
}

cleanup() {
  set +e
  say "[*] Cleaning up…"
  [[ -n "${SOCAT_PID:-}" ]] && kill "$SOCAT_PID" >/devnull 2>&1 || true
  [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true
  if [[ -n "${TOR_PID:-}" ]]; then
    kill "$TOR_PID" >/dev/null 2>&1 || true
    sleep 1
    kill -9 "$TOR_PID" >/dev/null 2>&1 || true
  fi
  # Always show the key paths on exit so you can copy them
  print_paths "ok"

  if [[ -z "${KEEP_WORK:-}" && -n "${WORK_DIR:-}" && -d "${WORK_DIR:-}" ]]; then
    rm -rf "$WORK_DIR"
  else
    say "[*] Workdir preserved: $WORK_DIR"
  fi
}
trap cleanup EXIT

# ---------------- Config ----------------
if [[ -z "${BIND_ADDR:-}" ]]; then
  BIND_ADDR="127.0.0.1:$(pick_free_port)"
fi
SRV_PORT="${BIND_ADDR##*:}"

# Pick node binary & make absolute so cd won't break it
if [[ -z "${NODE_BIN:-}" ]]; then
  if [[ -x target/debug/node ]]; then NODE_BIN="target/debug/node"
  elif [[ -x target/debug/ronode ]]; then NODE_BIN="target/debug/ronode"
  else NODE_BIN="target/debug/node"; fi
fi
if [[ ! -x "$NODE_BIN" ]]; then
  say "[*] Building node binary (cargo build -p node)…"
  cargo build -p node >/dev/null
fi
NODE_BIN_ABS="$(cd "$(dirname "$NODE_BIN")"; pwd)/$(basename "$NODE_BIN")"

SOCKS_PORT="${SOCKS_PORT:-0}"
CTRL_PORT="${CTRL_PORT:-0}"
BOOTSTRAP_TIMEOUT="${BOOTSTRAP_TIMEOUT:-240}"
HS_WAIT_SECS="${HS_WAIT_SECS:-120}"
RUST_LOG="${RUST_LOG:-info}"

# ---------------- Workspace ----------------
WORK_DIR="$(mktemp -d -t ron_onion_e2e.XXXXXX)"
RUN_SRV_DIR="$WORK_DIR/run_srv"   # server cwd -> its own .data here
RUN_CLI_DIR="$WORK_DIR/run_cli"   # client cwd -> its own .data here
mkdir -p "$RUN_SRV_DIR" "$RUN_CLI_DIR"

SERVER_LOG="$WORK_DIR/server.log"
TOR_LOG="$WORK_DIR/tor.log"
CONTROL_DUMP="$WORK_DIR/control.reply"
HS_EVENTS="$WORK_DIR/hs_events.log"
PUT_OUT="$WORK_DIR/put.out"
GET_OUT="$WORK_DIR/get.out"
PAYLOAD="$WORK_DIR/payload.txt"
TUNNEL_PORT_FILE="$WORK_DIR/tunnel.port"

say "[*] Working dir: $WORK_DIR"
say "[*] Using binary: $NODE_BIN_ABS"
say "[*] Starting local server: $BIND_ADDR"

# ---------------- Start clear TCP server in isolated cwd ----------------
if lsof -iTCP:"$SRV_PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  die "Port $SRV_PORT is already in use. Set BIND_ADDR to another port."
fi

# Run server from RUN_SRV_DIR so its default .data is isolated there
(
  cd "$RUN_SRV_DIR"
  RUST_LOG="$RUST_LOG" "$NODE_BIN_ABS" serve --bind "$BIND_ADDR" --transport tcp >"$SERVER_LOG" 2>&1 &
  echo $! > "$WORK_DIR/server.pid"
)
SERVER_PID="$(cat "$WORK_DIR/server.pid")"

say "[*] Waiting for server to listen…"
( for _ in {1..120}; do
    nc -z 127.0.0.1 "$SRV_PORT" >/dev/null 2>&1 && exit 0
    sleep 0.25
  done
  exit 1
) || {
  say "[!] Server failed to listen:"
  echo "----- server.log -----"
  sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n +1
  die "Server did not start listening on $BIND_ADDR"
}

# ---------------- Start Tor (temp instance) ----------------
for dep in tor socat nc; do
  command -v "$dep" >/dev/null 2>&1 || die "$dep not found. Install it (e.g., brew install $dep)."
done

[[ "$SOCKS_PORT" == "0" ]] && SOCKS_PORT="$(pick_free_port)" || true
[[ "$CTRL_PORT"  == "0" ]] && CTRL_PORT="$(pick_free_port)"  || true
DATA_DIR="$WORK_DIR/tor_data"; mkdir -p "$DATA_DIR"

say "[*] Starting Tor (Socks=$SOCKS_PORT, Control=$CTRL_PORT)…"
tor \
  --SocksPort "127.0.0.1:$SOCKS_PORT" \
  --ControlPort "127.0.0.1:$CTRL_PORT" \
  --CookieAuthentication 0 \
  --ClientOnly 1 \
  --DataDirectory "$DATA_DIR" \
  --Log "notice file $TOR_LOG" \
  --RunAsDaemon 1

if [[ -f "$DATA_DIR/tor.pid" ]]; then
  TOR_PID="$(cat "$DATA_DIR/tor.pid")"
else
  TOR_PID="$(pgrep -f "DataDirectory $DATA_DIR" | head -n1 || true)"
fi
[[ -n "${TOR_PID:-}" ]] || die "Failed to determine Tor PID."

say "[*] Waiting for Tor bootstrap (timeout ${BOOTSTRAP_TIMEOUT}s)…"
(
  end=$((SECONDS + BOOTSTRAP_TIMEOUT))
  while (( SECONDS < end )); do
    if grep -q "Bootstrapped 100% (done)" "$TOR_LOG" 2>/dev/null; then
      echo ok; exit 0
    fi
    sleep 1
  done
  exit 1
) >/dev/null || die "Tor did not bootstrap in time."
say "[*] Tor bootstrapped."

# ---------------- Create onion service via control port ----------------
PORT="$SRV_PORT"
say "[*] Connecting to control port…"
ADD_REQ=$'AUTHENTICATE ""\r\n'"ADD_ONION NEW:ED25519-V3 Flags=DiscardPK Port=${PORT},127.0.0.1:${PORT}\r\nQUIT\r\n"
printf "%s" "$ADD_REQ" | nc -w 5 127.0.0.1 "$CTRL_PORT" | tee "$CONTROL_DUMP" >/dev/null

SERVICE_ID="$(awk -F= '/^250-ServiceID=/{print $2; exit}' "$CONTROL_DUMP" | tr -d '\r\n' || true)"
if [[ -z "$SERVICE_ID" ]]; then
  echo "----- control (ADD_ONION) response -----"
  cat "$CONTROL_DUMP"
  die "Failed to parse ServiceID from control response."
fi
ONION_HOST="$(printf "%s" "$SERVICE_ID" | sed -E 's/[^a-z0-9]+$//')"
say "[+] Onion created: ${ONION_HOST}.onion:${PORT}"

# ---------------- Wait for HS_DESC UPLOADED ----------------
say "[*] Waiting for HS_DESC UPLOADED (timeout ${HS_WAIT_SECS}s)…"
(
  exec 3<>"/dev/tcp/127.0.0.1/$CTRL_PORT" || exit 1
  printf 'AUTHENTICATE ""\r\nSETEVENTS HS_DESC\r\n' >&3
  end=$((SECONDS + HS_WAIT_SECS))
  while (( SECONDS < end )); do
    IFS= read -r -t 1 line <&3 || true
    if [[ -n "${line:-}" ]]; then
      echo "$line"
      if [[ "$line" =~ ^650\ HS_DESC\ UPLOADED\ ([a-z0-9]+) ]]; then
        sid="${BASHREMATCH[1]}"
        if [[ "$sid" == "$SERVICE_ID" || "$sid" == "$ONION_HOST" ]]; then
          echo "[OK]"; break
        fi
      fi
    fi
  done
  printf 'QUIT\r\n' >&3
) | tee "$HS_EVENTS" >/dev/null

grep -q "\[OK\]" "$HS_EVENTS" || {
  say "[!] Timed out waiting for HS_DESC UPLOADED."
  echo "----- control (ADD_ONION) response -----"; cat "$CONTROL_DUMP"
  echo "----- tor.log tail -----"; tail -n 80 "$TOR_LOG" | sed -e 's/^/[tor] /'
  die "Hidden service descriptor upload not confirmed"
}
say "[+] HS_DESC UPLOADED confirmed for $ONION_HOST"

# ---------------- Start local TCP tunnel via SOCKS ----------------
TUN_LOCAL_PORT="$(pick_free_port)"
say "[*] Starting socat tunnel: 127.0.0.1:${TUN_LOCAL_PORT} -> ${ONION_HOST}.onion:${PORT} via SOCKS 127.0.0.1:${SOCKS_PORT}"
socat TCP-LISTEN:"$TUN_LOCAL_PORT",fork,reuseaddr SOCKS4A:127.0.0.1:"${ONION_HOST}.onion:${PORT}",socksport="$SOCKS_PORT" >/dev/null 2>&1 &
SOCAT_PID=$!
echo "$TUN_LOCAL_PORT" > "$TUNNEL_PORT_FILE"

for _ in {1..40}; do
  nc -z 127.0.0.1 "$TUN_LOCAL_PORT" >/dev/null 2>&1 && break || true
  sleep 0.25
done

# ---------------- E2E PUT/GET over onion ----------------
echo "Hello over onions at $(date)" > "$PAYLOAD"

say "[*] PUT over onion…"
set +e
( cd "$RUN_CLI_DIR"; "$NODE_BIN_ABS" put --to "127.0.0.1:${TUN_LOCAL_PORT}" --transport tcp "$PAYLOAD" ) >"$PUT_OUT" 2>&1
PUT_RC=$?
set -e
if [[ $PUT_RC -ne 0 ]]; then
  say "[!] PUT failed (exit $PUT_RC)."
  echo "----- server.log -----"; sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n +1
  echo "----- put.out -----";    sed -e 's/^/[put] /' "$PUT_OUT" | tail -n +1
  die "PUT failed"
fi

CID="$(grep -Eo 'cid=([A-Za-z0-9:/+=_-]+)' "$PUT_OUT" | head -n1 | cut -d= -f2 || true)"
if [[ -z "$CID" ]]; then CID="$(tail -n1 "$PUT_OUT" | tr -d '[:space:]')"; fi
if [[ -z "$CID" ]]; then
  say "[!] PUT succeeded but no CID/hash was printed."
  echo "----- put.out -----"; sed -e 's/^/[put] /' "$PUT_OUT" | tail -n +1
  die "No CID/hash parsed from PUT output"
fi
say "[+] PUT cid/hash: $CID"

say "[*] GET over onion…"
set +e
( cd "$RUN_CLI_DIR"; "$NODE_BIN_ABS" get --to "127.0.0.1:${TUN_LOCAL_PORT}" --transport tcp "$CID" ) >"$GET_OUT" 2>&1
GET_RC=$?
set -e
if [[ $GET_RC -ne 0 ]]; then
  say "[!] GET failed (exit $GET_RC)."
  echo "----- server.log -----"; sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n +1
  echo "----- get.out -----";    sed -e 's/^/[get] /' "$GET_OUT" | tail -n +1
  die "GET failed"
fi

if ! cmp -s "$PAYLOAD" "$GET_OUT"; then
  say "[!] Mismatch between sent payload and GET output."
  echo "----- server.log (tail) -----"
  tail -n 80 "$SERVER_LOG" | sed -e 's/^/[server] /'
  die "Payload/GET mismatch"
fi

say "[✅] E2E over onion PASSED"
say "[i] Onion: ${ONION_HOST}.onion:${PORT}"
say "[i] Local tunnel: 127.0.0.1:${TUN_LOCAL_PORT}  (socat → Tor SOCKS ${SOCKS_PORT})"
say "[i] Logs:"
say "    server_log: $SERVER_LOG"
say "    tor_log:    $TOR_LOG"
say "    control:    $CONTROL_DUMP"
say "    hs_events:  $HS_EVENTS"
say "    put_out:    $PUT_OUT"
say "    get_out:    $GET_OUT"
say "[i] Workdir: $WORK_DIR (KEEP_WORK=1 to preserve)"
