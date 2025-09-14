#!/usr/bin/env bash
# FILE: testing/test_onion_e2e.sh
# Onion E2E over public Tor.
# macOS/Bash 3.2 friendly. Adds:
#  - PERSIST_HS=1 → reuse ED25519-V3 key across runs (speeds up HS availability)
#  - Active client-side HSFETCH to force descriptor retrieval
#  - Split-mode default (FAST_ONION=1): HS Tor (single-hop) + Client Tor (Socks)

set -euo pipefail

LOG_PREFIX="[ron-e2e]"
say() { echo -e "${LOG_PREFIX} $*"; }
die() { echo -e "${LOG_PREFIX} [!] $*" >&2; print_paths "error"; exit 1; }

# ---------- tiny helpers ----------
pick_free_port() {
  local p i
  for i in 1 2 3 4 5 6 7 8 9 10; do
    p=$(( (RANDOM % 20000) + 20000 ))
    if ! lsof -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1; then
      echo "$p"; return 0
    fi
  done
  for p in $(seq 20000 50000); do
    lsof -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1 || { echo "$p"; return 0; }
  done
  return 1
}

wait_tcp() { # host port timeout_sec
  local h="$1" p="$2" t="${3:-30}" start end
  start=$(date +%s)
  while ! nc -z "$h" "$p" >/dev/null 2>&1; do
    end=$(date +%s)
    [ $((end-start)) -ge "$t" ] && return 1
    sleep 0.25
  done
  return 0
}

dump_hs_debug() {
  if [[ -n "${TOR_LOG_HS:-}" && -f "${TOR_LOG_HS:-}" ]]; then
    echo "----- tor-hs.log (HS-related, tail) -----"
    grep -Ei 'hs_|hidden service|HSDir|descriptor|intro|upload|rend|client.*desc|No more HSDir|resolve failed' "$TOR_LOG_HS" | tail -n 300 | sed -e 's/^/[tor-hs] /' || true
  fi
  if [[ -n "${TOR_LOG_CLI:-}" && -f "${TOR_LOG_CLI:-}" ]]; then
    echo "----- tor-cli.log (client-side, tail) -----"
    grep -Ei 'hs_|hidden service|HSDir|descriptor|intro|upload|rend|client.*desc|No more HSDir|resolve failed|Socks listener|HSFETCH' "$TOR_LOG_CLI" | tail -n 300 | sed -e 's/^/[tor-cli] /' || true
  fi
  if [[ -n "${TOR_LOG:-}" && -f "${TOR_LOG:-}" ]]; then
    echo "----- tor.log (single-instance mode, tail) -----"
    grep -Ei 'hs_|hidden service|HSDir|descriptor|intro|upload|rend|client.*desc|No more HSDir|resolve failed|Socks listener|HSFETCH' "$TOR_LOG" | tail -n 300 | sed -e 's/^/[tor] /' || true
  fi
}

# ---- globals for pretty path dump ----
SERVER_LOG=""; TOR_LOG=""; TOR_LOG_HS=""; TOR_LOG_CLI=""
CONTROL_DUMP=""; HS_EVENTS=""; CONTROL_DUMP_HS=""; HS_EVENTS_HS=""
PUT_OUT=""; GET_OUT=""; PAYLOAD=""; TUNNEL_PORT_FILE=""; PATHS_TXT=""; BUNDLE_PATH=""
TAIL_PID=""; TAIL_PID_HS=""; TAIL_PID_CLI=""

# --- Pretty path summary (always show on exit / errors) ---
print_paths() {
  if [[ -z "${WORK_DIR:-}" || ! -d "${WORK_DIR:-}" ]]; then return 0; fi
  PATHS_TXT="$WORK_DIR/paths.txt"
  {
    echo "# RustyOnions onion e2e — key files (run: $(date -u +'%Y-%m-%dT%H:%M:%SZ'))"
    echo "workdir        = $WORK_DIR"
    [[ -n "$SERVER_LOG"      ]] && echo "server_log     = $SERVER_LOG"
    [[ -n "$TOR_LOG"         ]] && echo "tor_log        = $TOR_LOG"
    [[ -n "$TOR_LOG_HS"      ]] && echo "tor_hs_log     = $TOR_LOG_HS"
    [[ -n "$TOR_LOG_CLI"     ]] && echo "tor_cli_log    = $TOR_LOG_CLI"
    [[ -n "$CONTROL_DUMP"    ]] && echo "control_reply  = $CONTROL_DUMP"
    [[ -n "$CONTROL_DUMP_HS" ]] && echo "control_hs     = $CONTROL_DUMP_HS"
    [[ -n "$HS_EVENTS"       ]] && echo "hs_events      = $HS_EVENTS"
    [[ -n "$HS_EVENTS_HS"    ]] && echo "hs_events_hs   = $HS_EVENTS_HS"
    [[ -n "$PUT_OUT"         ]] && echo "put_out        = $PUT_OUT"
    [[ -n "$GET_OUT"         ]] && echo "get_out        = $GET_OUT"
  } > "$PATHS_TXT"

  say "[i] ===== Copy these paths ====="
  [[ -n "$SERVER_LOG"      ]] && echo "$SERVER_LOG"
  [[ -n "$TOR_LOG"         ]] && echo "$TOR_LOG"
  [[ -n "$TOR_LOG_HS"      ]] && echo "$TOR_LOG_HS"
  [[ -n "$TOR_LOG_CLI"     ]] && echo "$TOR_LOG_CLI"
  [[ -n "$CONTROL_DUMP"    ]] && echo "$CONTROL_DUMP"
  [[ -n "$CONTROL_DUMP_HS" ]] && echo "$CONTROL_DUMP_HS"
  [[ -n "$HS_EVENTS"       ]] && echo "$HS_EVENTS"
  [[ -n "$HS_EVENTS_HS"    ]] && echo "$HS_EVENTS_HS"
  [[ -n "$PUT_OUT"         ]] && echo "$PUT_OUT"
  [[ -n "$GET_OUT"         ]] && echo "$GET_OUT"
  echo "$PATHS_TXT"
  say "[i] ============================"

  if [[ "${BUNDLE_LOGS:-0}" == "1" ]]; then
    BUNDLE_PATH="$WORK_DIR/logs_bundle.tar.gz"
    local files=()
    for f in "$SERVER_LOG" "$TOR_LOG" "$TOR_LOG_HS" "$TOR_LOG_CLI" \
             "$CONTROL_DUMP" "$CONTROL_DUMP_HS" "$HS_EVENTS" "$HS_EVENTS_HS" \
             "$PUT_OUT" "$GET_OUT" "$PATHS_TXT"; do
      if [[ -n "$f" && -f "$f" ]]; then files+=( "$(basename "$f")" ); fi
    done
    if (( ${#files[@]} > 0 )); then
      ( cd "$WORK_DIR" && tar -czf "$BUNDLE_PATH" "${files[@]}" ) >/dev/null 2>&1 || true
      [[ -f "$BUNDLE_PATH" ]] && say "[i] Log bundle: $BUNDLE_PATH"
    fi
  fi
}

cleanup() {
  set +e
  say "[*] Cleaning up…"
  [[ -n "${SOCAT_PID:-}"    ]] && kill "$SOCAT_PID"    >/dev/null 2>&1 || true
  [[ -n "${SERVER_PID:-}"   ]] && kill "$SERVER_PID"   >/dev/null 2>&1 || true
  [[ -n "${TAIL_PID:-}"     ]] && kill "$TAIL_PID"     >/dev/null 2>&1 || true
  [[ -n "${TAIL_PID_HS:-}"  ]] && kill "$TAIL_PID_HS"  >/dev/null 2>&1 || true
  [[ -n "${TAIL_PID_CLI:-}" ]] && kill "$TAIL_PID_CLI" >/dev/null 2>&1 || true
  if [[ -n "${TOR_PID:-}"      ]]; then kill "$TOR_PID"      >/dev/null 2>&1 || true; sleep 0.25; kill -9 "$TOR_PID"      >/dev/null 2>&1 || true; fi
  if [[ -n "${TOR_PID_HS:-}"   ]]; then kill "$TOR_PID_HS"   >/dev/null 2>&1 || true; sleep 0.25; kill -9 "$TOR_PID_HS"   >/dev/null 2>&1 || true; fi
  if [[ -n "${TOR_PID_CLI:-}"  ]]; then kill "$TOR_PID_CLI"  >/dev/null 2>&1 || true; sleep 0.25; kill -9 "$TOR_PID_CLI"  >/dev/null 2>&1 || true; fi
  print_paths "ok"
  if [[ -z "${KEEP_WORK:-}" && -n "${WORK_DIR:-}" && -d "${WORK_DIR:-}" ]]; then
    rm -rf "$WORK_DIR"
  else
    say "[*] Workdir preserved: $WORK_DIR"
  fi
}
trap cleanup EXIT

# ---------------- Config ----------------
# Defaults tuned for reliability on public Tor:
FAST_ONION="${FAST_ONION:-1}"           # default split mode
BOOTSTRAP_TIMEOUT="${BOOTSTRAP_TIMEOUT:-600}"
HS_WAIT_SECS="${HS_WAIT_SECS:-420}"
CONTROL_WAIT_SECS="${CONTROL_WAIT_SECS:-30}"
PUT_RETRIES="${PUT_RETRIES:-20}"
PUT_RETRY_DELAY="${PUT_RETRY_DELAY:-10}"
TOR_LOG_LEVEL="${TOR_LOG_LEVEL:-notice}"
FOLLOW_LOGS="${FOLLOW_LOGS:-0}"
RUST_LOG="${RUST_LOG:-info}"
TOR_BIN="${TOR_BIN:-tor}"
USE_SYSTEM_TORRC="${USE_SYSTEM_TORRC:-0}"
INCLUDE_CIRC_EVENTS="${INCLUDE_CIRC_EVENTS:-1}"
HS_EVENT_STRICT="${HS_EVENT_STRICT:-0}" # don’t fail if we don’t see UPLOADED
# NEW: persistent HS key between runs
PERSIST_HS="${PERSIST_HS:-1}"
PERSIST_DIR_DEFAULT="$HOME/.rustyonions/e2e_hs"
PERSIST_DIR="${PERSIST_DIR:-$PERSIST_DIR_DEFAULT}"
mkdir -p "$PERSIST_DIR" >/dev/null 2>&1 || true
HS_KEY_FILE="$PERSIST_DIR/ed25519.key"   # stores "ED25519-V3:<base64>"

# Address for our clear TCP server
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

# ---------------- Workspace ----------------
WORK_DIR="$(mktemp -d -t ron_onion_e2e.XXXXXX)"
RUN_SRV_DIR="$WORK_DIR/run_srv"
RUN_CLI_DIR="$WORK_DIR/run_cli"
mkdir -p "$RUN_SRV_DIR" "$RUN_CLI_DIR"

SERVER_LOG="$WORK_DIR/server.log"
CONTROL_DUMP="$WORK_DIR/control.reply"
CONTROL_DUMP_HS="$WORK_DIR/control_hs.reply"
HS_EVENTS="$WORK_DIR/hs_events.log"
HS_EVENTS_HS="$WORK_DIR/hs_events_hs.log"
PUT_OUT="$WORK_DIR/put.out"
GET_OUT="$WORK_DIR/get.out"
PAYLOAD="$WORK_DIR/payload.txt"
TUNNEL_PORT_FILE="$WORK_DIR/tunnel.port"

say "[*] Working dir: $WORK_DIR"
say "[*] Using binary: $NODE_BIN_ABS"
say "[*] Starting local server: $BIND_ADDR"

# ---------------- Start clear TCP server ----------------
if lsof -iTCP:"$SRV_PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  die "Port $SRV_PORT is already in use. Set BIND_ADDR to another port."
fi

(
  cd "$RUN_SRV_DIR"
  RUST_LOG="$RUST_LOG" "$NODE_BIN_ABS" serve --bind "$BIND_ADDR" --transport tcp >"$SERVER_LOG" 2>&1 &
  echo $! > "$WORK_DIR/server.pid"
)
SERVER_PID="$(cat "$WORK_DIR/server.pid")"

say "[*] Waiting for server to listen…"
if ! wait_tcp 127.0.0.1 "$SRV_PORT" 30; then
  say "[!] Server failed to listen:"
  echo "----- server.log -----"
  sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n +1
  die "Server did not start listening on $BIND_ADDR"
fi

# ---------------- Tor launchers ----------------
ensure_deps() {
  for dep in socat nc; do
    command -v "$dep" >/dev/null 2>&1 || die "$dep not found. Install it (e.g., brew install $dep)."
  done
  command -v "$TOR_BIN" >/dev/null 2>&1 || die "tor not found at '$TOR_BIN'. Set TOR_BIN=/path/to/tor or add it to PATH."
}
ensure_deps

SOCKS_PORT="${SOCKS_PORT:-0}"
CTRL_PORT="${CTRL_PORT:-0}"
CTRL_PORT_HS="${CTRL_PORT_HS:-0}"
CTRL_PORT_CLI="${CTRL_PORT_CLI:-0}"

start_single_tor() {
  [[ "$SOCKS_PORT" == "0" ]] && SOCKS_PORT="$(pick_free_port)" || true
  [[ "$CTRL_PORT"  == "0" ]] && CTRL_PORT="$(pick_free_port)"  || true
  DATA_DIR="$WORK_DIR/tor_data"; mkdir -p "$DATA_DIR"
  TOR_LOG="$WORK_DIR/tor.log"
  local torrc="$WORK_DIR/torrc.empty"; : > "$torrc"

  local flags=(
    --SocksPort "127.0.0.1:$SOCKS_PORT"
    --ControlPort "127.0.0.1:$CTRL_PORT"
    --ClientOnly 1
    --DormantCanceledByStartup 1
    --DataDirectory "$DATA_DIR"
    --Log "$TOR_LOG_LEVEL file $TOR_LOG"
    --RunAsDaemon 1
  )
  if [[ "$USE_SYSTEM_TORRC" != "1" ]]; then
    flags+=( --defaults-torrc /dev/null -f "$torrc" --ignore-missing-torrc )
  fi
  if [[ -n "${TOR_EXTRA_FLAGS:-}" ]]; then EXTRA=( $TOR_EXTRA_FLAGS ); flags+=( "${EXTRA[@]}" ); fi

  say "[*] Starting Tor (single) Socks=$SOCKS_PORT Control=$CTRL_PORT using '$TOR_BIN'…"
  "$TOR_BIN" "${flags[@]}"

  if [[ -f "$DATA_DIR/tor.pid" ]]; then TOR_PID="$(cat "$DATA_DIR/tor.pid")"
  else TOR_PID="$(pgrep -f "DataDirectory $DATA_DIR" | head -n1 || true)"; fi
  [[ -n "${TOR_PID:-}" ]] || die "Failed to determine Tor PID."

  [[ "$FOLLOW_LOGS" == "1" ]] && ( tail -n +1 -F "$TOR_LOG" 2>/dev/null | sed -e 's/^/[tor] /' ) & TAIL_PID=$!

  say "[*] Waiting for Tor bootstrap (timeout ${BOOTSTRAP_TIMEOUT}s)…"
  local deadline=$((SECONDS + BOOTSTRAP_TIMEOUT))
  while (( SECONDS < deadline )); do
    grep -q "Bootstrapped 100% (done)" "$TOR_LOG" 2>/dev/null && { say "[*] Tor bootstrapped."; break; }
    sleep 0.25
  done
  grep -q "Bootstrapped 100% (done)" "$TOR_LOG" 2>/dev/null || die "Tor did not bootstrap in time."
}

start_split_tor() {
  # HS Tor (single-hop HS)
  [[ "$CTRL_PORT_HS" == "0" ]] && CTRL_PORT_HS="$(pick_free_port)" || true
  DATA_DIR_HS="$WORK_DIR/tor_hs_data"; mkdir -p "$DATA_DIR_HS"
  TOR_LOG_HS="$WORK_DIR/tor-hs.log"
  local torrc_hs="$WORK_DIR/torrc.hs.empty"; : > "$torrc_hs"

  # Client Tor (Socks)
  [[ "$SOCKS_PORT"    == "0" ]] && SOCKS_PORT="$(pick_free_port)"    || true
  [[ "$CTRL_PORT_CLI" == "0" ]] && CTRL_PORT_CLI="$(pick_free_port)" || true
  DATA_DIR_CLI="$WORK_DIR/tor_cli_data"; mkdir -p "$DATA_DIR_CLI"
  TOR_LOG_CLI="$WORK_DIR/tor-cli.log"
  local torrc_cli="$WORK_DIR/torrc.cli.empty"; : > "$torrc_cli"

  local flags_hs=(
    --HiddenServiceNonAnonymousMode 1
    --HiddenServiceSingleHopMode 1
    --SocksPort 0 --TransPort 0 --NATDPort 0 --DNSPort 0
    --ControlPort "127.0.0.1:$CTRL_PORT_HS"
    --ClientOnly 1
    --DormantCanceledByStartup 1
    --DataDirectory "$DATA_DIR_HS"
    --Log "$TOR_LOG_LEVEL file $TOR_LOG_HS"
    --RunAsDaemon 1
  )
  local flags_cli=(
    --SocksPort "127.0.0.1:$SOCKS_PORT"
    --ControlPort "127.0.0.1:$CTRL_PORT_CLI"
    --ClientOnly 1
    --DormantCanceledByStartup 1
    --DataDirectory "$DATA_DIR_CLI"
    --Log "$TOR_LOG_LEVEL file $TOR_LOG_CLI"
    --RunAsDaemon 1
  )
  if [[ "$USE_SYSTEM_TORRC" != "1" ]]; then
    flags_hs+=( --defaults-torrc /dev/null -f "$torrc_hs" --ignore-missing-torrc )
    flags_cli+=( --defaults-torrc /dev/null -f "$torrc_cli" --ignore-missing-torrc )
  fi
  if [[ -n "${TOR_EXTRA_FLAGS:-}" ]]; then EXTRA=( $TOR_EXTRA_FLAGS ); flags_hs+=( "${EXTRA[@]}" ); flags_cli+=( "${EXTRA[@]}" ); fi

  say "[*] Starting Tor-HS Control=$CTRL_PORT_HS (single-hop HS)…"
  "$TOR_BIN" "${flags_hs[@]}"
  say "[*] Starting Tor-CLI Socks=$SOCKS_PORT Control=$CTRL_PORT_CLI …"
  "$TOR_BIN" "${flags_cli[@]}"

  if [[ -f "$DATA_DIR_HS/tor.pid" ]]; then TOR_PID_HS="$(cat "$DATA_DIR_HS/tor.pid")"
  else TOR_PID_HS="$(pgrep -f "DataDirectory $DATA_DIR_HS" | head -n1 || true)"; fi
  [[ -n "${TOR_PID_HS:-}" ]] || die "Failed to determine Tor-HS PID."

  if [[ -f "$DATA_DIR_CLI/tor.pid" ]]; then TOR_PID_CLI="$(cat "$DATA_DIR_CLI/tor.pid")"
  else TOR_PID_CLI="$(pgrep -f "DataDirectory $DATA_DIR_CLI" | head -n1 || true)"; fi
  [[ -n "${TOR_PID_CLI:-}" ]] || die "Failed to determine Tor-CLI PID."

  [[ "$FOLLOW_LOGS" == "1" ]] && ( tail -n +1 -F "$TOR_LOG_HS"  2>/dev/null | sed -e 's/^/[tor-hs] /'  ) & TAIL_PID_HS=$!
  [[ "$FOLLOW_LOGS" == "1" ]] && ( tail -n +1 -F "$TOR_LOG_CLI" 2>/dev/null | sed -e 's/^/[tor-cli] /' ) & TAIL_PID_CLI=$!

  say "[*] Waiting for Tor-HS bootstrap (timeout ${BOOTSTRAP_TIMEOUT}s)…"
  local d1=$((SECONDS + BOOTSTRAP_TIMEOUT))
  while (( SECONDS < d1 )); do
    grep -q "Bootstrapped 100% (done)" "$TOR_LOG_HS" 2>/dev/null && { say "[*] Tor-HS bootstrapped."; break; }
    sleep 0.25
  done
  grep -q "Bootstrapped 100% (done)" "$TOR_LOG_HS" 2>/dev/null || die "Tor-HS did not bootstrap in time."

  say "[*] Waiting for Tor-CLI bootstrap (timeout ${BOOTSTRAP_TIMEOUT}s)…"
  local d2=$((SECONDS + BOOTSTRAP_TIMEOUT))
  while (( SECONDS < d2 )); do
    grep -q "Bootstrapped 100% (done)" "$TOR_LOG_CLI" 2>/dev/null && { say "[*] Tor-CLI bootstrapped."; break; }
    sleep 0.25
  done
  grep -q "Bootstrapped 100% (done)" "$TOR_LOG_CLI" 2>/dev/null || die "Tor-CLI did not bootstrap in time."
}

# ---------------- Start Tor ----------------
if [[ "$FAST_ONION" == "1" ]]; then start_split_tor; else start_single_tor; fi

# ---------------- Tor control helpers ----------------
ctrl_open() { # host port -> opens FD 3
  local host="$1" port="$2"
  exec 3<>"/dev/tcp/${host}/${port}" || return 1
  read -r -t 2 _banner <&3 || true
  printf 'AUTHENTICATE ""\r\n' >&3
  local end=$((SECONDS + CONTROL_WAIT_SECS)) line ok=0
  while (( SECONDS < end )); do
    IFS= read -r -t 2 -u 3 line || true
    [[ -z "${line:-}" ]] && continue
    [[ "$line" =~ ^250 ]]; ok=$?
    if [[ "$line" =~ ^250[[:space:]]OK ]]; then ok=0; break; fi
  done
  return $ok
}
ctrl_close() { printf 'QUIT\r\n' >&3 2>/dev/null || true; exec 3>&- 2>/dev/null || true; }
ctrl_readln() { local line; IFS= read -r -t 2 -u 3 line || return 124; printf "%s\n" "${line%$'\r'}"; return 0; }

# ---------------- Create or restore onion ----------------
PORT="$SRV_PORT"
say "[*] Connecting to control port…"

# choose HS control: split -> CTRL_PORT_HS ; single -> CTRL_PORT
if [[ "$FAST_ONION" == "1" ]]; then
  CTRL_HOST="127.0.0.1"; CTRL_USE_PORT="$CTRL_PORT_HS"
  CONTROL_DUMP_USE="$CONTROL_DUMP_HS"; HS_EVENTS_USE="$HS_EVENTS_HS"
else
  CTRL_HOST="127.0.0.1"; CTRL_USE_PORT="$CTRL_PORT"
  CONTROL_DUMP_USE="$CONTROL_DUMP"; HS_EVENTS_USE="$HS_EVENTS"
fi

ctrl_open "$CTRL_HOST" "$CTRL_USE_PORT" || die "Failed to open HS control port"

svc_id=""
saved_key=""
if [[ "$PERSIST_HS" == "1" && -f "$HS_KEY_FILE" ]]; then
  saved_key="$(cat "$HS_KEY_FILE" | tr -d '[:space:]')"
fi

if [[ -n "$saved_key" ]]; then
  printf 'ADD_ONION ED25519-V3:%s Port=%s,127.0.0.1:%s\r\n' "$saved_key" "$PORT" "$PORT" >&3
else
  printf 'ADD_ONION NEW:ED25519-V3 Port=%s,127.0.0.1:%s\r\n' "$PORT" "$PORT" >&3
fi

add_dead=$((SECONDS + CONTROL_WAIT_SECS))
while (( SECONDS < add_dead )); do
  line="$(ctrl_readln || true)"; [[ -n "$line" ]] && printf "%s\n" "$line" | tee -a "$CONTROL_DUMP_USE" >/dev/null
  if [[ "$line" =~ ^250-ServiceID=([a-z2-7]{56})$ ]]; then svc_id="${BASH_REMATCH[1]}"; fi
  if [[ -z "$saved_key" && "$line" =~ ^250-PrivateKey=ED25519-V3:([A-Za-z0-9+/=]+)$ ]]; then
    newkey="${BASH_REMATCH[1]}"; echo "ED25519-V3:${newkey}" > "$HS_KEY_FILE" 2>/dev/null || true
  fi
  [[ "$line" =~ ^250[[:space:]]OK ]] && break
done
[[ -n "$svc_id" ]] || die "Failed to parse ServiceID from control response (see $CONTROL_DUMP_USE)"
ONION_HOST="$svc_id"
say "[+] Onion created: ${ONION_HOST}.onion:${PORT}"

ctrl_close

# ---------------- Wait for HS_DESC UPLOADED ----------------
say "[*] Waiting for HS_DESC UPLOADED (timeout ${HS_WAIT_SECS}s)…"
(
  if [[ "$FAST_ONION" == "1" ]]; then CP="$CTRL_PORT_HS"; else CP="$CTRL_PORT"; fi
  exec 3<>"/dev/tcp/127.0.0.1/$CP" || exit 1
  read -r -t 2 _b <&3 || true
  printf 'AUTHENTICATE ""\r\n' >&3
  end=$((SECONDS + CONTROL_WAIT_SECS)); ok=0
  while (( SECONDS < end )); do
    IFS= read -r -t 1 line <&3 || true
    [[ -z "${line:-}" ]] && continue
    echo "${line%$'\r'}"
    [[ "$line" =~ ^250[[:space:]]OK ]] && { ok=1; break; }
  done
  (( ok == 1 )) || { echo "[ERR] auth-no-250"; exit 0; }

  EVENTS="HS_DESC"; [[ "${INCLUDE_CIRC_EVENTS:-1}" == "1" ]] && EVENTS="$EVENTS CIRC"
  echo "SETEVENTS $EVENTS" >&3
  ok=0; end=$((SECONDS + CONTROL_WAIT_SECS))
  while (( SECONDS < end )); do
    IFS= read -r -t 1 line <&3 || true
    [[ -z "${line:-}" ]] && continue
    echo "${line%$'\r'}"
    [[ "$line" =~ ^250[[:space:]]OK ]] && { ok=1; break; }
    [[ "$line" =~ ^552[[:space:]]+Unrecognized[[:space:]]+event ]] && { echo "[ERR] setevents-unsupported"; break; }
  done
  (( ok == 1 )) || { echo "[ERR] setevents-no-250"; exit 0; }

  end=$((SECONDS + HS_WAIT_SECS)); ok=0
  while (( SECONDS < end )); do
    IFS= read -r -t 1 line <&3 || true
    [[ -z "${line:-}" ]] && continue
    echo "${line%$'\r'}"
    if [[ "$line" == *"HS_DESC"* && "$line" == *"UPLOADED"* && "$line" == *"$ONION_HOST"* ]]; then ok=1; echo "[OK]"; break; fi
    if [[ "$line" =~ ^650[[:space:]]+HS_DESC[[:space:]]+UPLOADED[[:space:]]+([a-z2-7]{56}) ]]; then
      sid="${BASH_REMATCH[1]}"; [[ "$sid" == "$ONION_HOST" ]] && { ok=1; echo "[OK]"; break; }
    fi
  done
  printf 'QUIT\r\n' >&3
  exit $ok
) | tee "$HS_EVENTS_USE" >/dev/null

if ! grep -q "\[OK\]" "$HS_EVENTS_USE"; then
  say "[!] HS_DESC UPLOADED not seen in window; will proceed and actively fetch on client side."
fi

# ---------------- Actively fetch descriptor on client side ----------------
fetch_desc_ok=0
if [[ "$FAST_ONION" == "1" ]]; then
  CF_HOST="127.0.0.1"; CF_PORT="$CTRL_PORT_CLI"
else
  CF_HOST="127.0.0.1"; CF_PORT="$CTRL_PORT"
fi

if ctrl_open "$CF_HOST" "$CF_PORT"; then
  printf 'SETEVENTS HS_DESC\r\n' >&3
  # drain to 250 OK
  end=$((SECONDS + CONTROL_WAIT_SECS)); while (( SECONDS < end )); do line="$(ctrl_readln || true)"; [[ "$line" =~ ^250\ OK ]] && break; done
  printf 'HSFETCH %s\r\n' "$ONION_HOST" >&3
  end=$((SECONDS + 60)); while (( SECONDS < end )); do
    line="$(ctrl_readln || true)"; [[ -z "${line:-}" ]] && continue
    if [[ "$line" =~ ^650[[:space:]]+HS_DESC[[:space:]]+RECEIVED[[:space:]]+([a-z2-7]{56}) ]]; then
      sid="${BASH_REMATCH[1]}"; [[ "$sid" == "$ONION_HOST" ]] && { fetch_desc_ok=1; break; }
    fi
    # Bail out if client reports no HSDirs left (rare but informative)
    if echo "$line" | grep -q "No more HSDir available to query"; then break; fi
  done
  ctrl_close
fi

if [[ "$fetch_desc_ok" -ne 1 ]]; then
  say "[!] Client-side descriptor fetch not confirmed; will still attempt tunnel/PUT (retries enabled)."
fi

# ---------------- Start local TCP tunnel via SOCKS ----------------
TUN_LOCAL_PORT="$(pick_free_port)"
if [[ "$FAST_ONION" == "1" ]]; then
  say "[*] Starting socat tunnel: 127.0.0.1:${TUN_LOCAL_PORT} -> ${ONION_HOST}.onion:${PORT} via SOCKS 127.0.0.1:${SOCKS_PORT} (split)"
else
  say "[*] Starting socat tunnel: 127.0.0.1:${TUN_LOCAL_PORT} -> ${ONION_HOST}.onion:${PORT} via SOCKS 127.0.0.1:${SOCKS_PORT}"
fi
socat TCP-LISTEN:"$TUN_LOCAL_PORT",fork,reuseaddr \
      SOCKS4A:127.0.0.1:"${ONION_HOST}.onion:${PORT}",socksport="$SOCKS_PORT" >/dev/null 2>&1 &
SOCAT_PID=$!
echo "$TUN_LOCAL_PORT" > "$TUNNEL_PORT_FILE"

for _ in 1 2 3 4 5 6 7 8 9 10 11 12; do
  nc -z 127.0.0.1 "$TUN_LOCAL_PORT" >/dev/null 2>&1 && break
  sleep 0.5
done

# ---------------- E2E PUT/GET over onion ----------------
echo "Hello over onions at $(date)" > "$PAYLOAD"

if "$NODE_BIN_ABS" put --help 2>&1 | grep -q -- '--path <PATH>'; then
  PUT_CMD_BASE=( "$NODE_BIN_ABS" put --path "$PAYLOAD" --to "127.0.0.1:${TUN_LOCAL_PORT}" )
else
  PUT_CMD_BASE=( "$NODE_BIN_ABS" put --to "127.0.0.1:${TUN_LOCAL_PORT}" "$PAYLOAD" )
fi

attempt_put() {
  local tries="$1" delay="$2" i rc
  : >"$PUT_OUT"
  for i in $(seq 1 "$tries"); do
    say "[*] PUT over onion… (attempt $i/$tries)"
    set +e
    ( cd "$RUN_CLI_DIR"; "${PUT_CMD_BASE[@]}" ) >>"$PUT_OUT" 2>&1
    rc=$?
    set -e
    if [[ $rc -eq 0 ]]; then return 0; fi
    tail -n 14 "$PUT_OUT" | sed -e 's/^/[put] /'
    sleep "$delay"
  done
  return 1
}

if ! attempt_put "$PUT_RETRIES" "$PUT_RETRY_DELAY"; then
  say "[!] PUT failed after retries."
  dump_hs_debug
  echo "----- server.log -----"; sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n +1
  echo "----- put.out -----";    sed -e 's/^/[put] /' "$PUT_OUT" | tail -n +1
  die "PUT failed"
fi

CID="$(grep -Eo 'cid=([A-Za-z0-9:/+=_-]+)' "$PUT_OUT" | head -n1 | cut -d= -f2 || true)"
[[ -z "$CID" ]] && CID="$(tail -n1 "$PUT_OUT" | tr -d '[:space:]')"
[[ -n "$CID" ]] || { echo "----- put.out -----"; sed -e 's/^/[put] /' "$PUT_OUT" | tail -n +1; die "No CID/hash parsed from PUT output"; }
say "[+] PUT cid/hash: $CID"

if "$NODE_BIN_ABS" get --help 2>&1 | grep -q 'get --to <TO>'; then
  GET_CMD=( "$NODE_BIN_ABS" get --to "127.0.0.1:${TUN_LOCAL_PORT}" "$CID" )
elif "$NODE_BIN_ABS" get --help 2>&1 | grep -q -- '--cid'; then
  GET_CMD=( "$NODE_BIN_ABS" get --cid "$CID" --to "127.0.0.1:${TUN_LOCAL_PORT}" )
else
  GET_CMD=( "$NODE_BIN_ABS" get "127.0.0.1:${TUN_LOCAL_PORT}" "$CID" )
fi

say "[*] GET over onion…"
set +e
( cd "$RUN_CLI_DIR"; "${GET_CMD[@]}" ) >"$GET_OUT" 2>&1
GET_RC=$?
set -e
if [[ $GET_RC -ne 0 ]]; then
  say "[!] GET failed (exit $GET_RC)."
  dump_hs_debug
  echo "----- server.log -----"; sed -e 's/^/[server] /' "$SERVER_LOG" | tail -n +1
  echo "----- get.out -----";    sed -e 's/^/[get] /' "$GET_OUT" | tail -n +1
  die "GET failed"
fi

if ! cmp -s "$PAYLOAD" "$GET_OUT"; then
  say "[!] Mismatch between sent payload and GET output."
  dump_hs_debug
  echo "----- server.log (tail) -----"; tail -n 120 "$SERVER_LOG" | sed -e 's/^/[server] /'
  die "Payload/GET mismatch"
fi

say "[✅] E2E over onion PASSED"
say "[i] Onion: ${ONION_HOST}.onion:${PORT}"
if [[ "$FAST_ONION" == "1" ]]; then
  say "[i] Split mode: HS Control=$CTRL_PORT_HS, Client Socks=$SOCKS_PORT Control=$CTRL_PORT_CLI"
else
  say "[i] Single Tor: Socks=$SOCKS_PORT Control=$CTRL_PORT"
fi
say "[i] Local tunnel: 127.0.0.1:${TUN_LOCAL_PORT}  (socat → Tor SOCKS ${SOCKS_PORT})"
say "[i] HS key file: $HS_KEY_FILE  (delete to rotate onion)"
say "[i] Logs saved under: $WORK_DIR  (set KEEP_WORK=1 to preserve)"
