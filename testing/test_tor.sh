#!/usr/bin/env bash
# Tor smoke test: launch an isolated Tor, watch bootstrap via the control port, then exit.
# macOS/Linux. Supports obfs4 (and optional snowflake) bridges, cleanup trap, and diagnostics.
#
# Env (all optional):
#   TOR_BOOTSTRAP_TIMEOUT   seconds to allow for bootstrap (default: 180)
#   TOR_COOKIE_TIMEOUT      seconds to wait for control_auth_cookie (default: 5)
#   STALL_SECS              seconds of no progress before showing a log peek (default: 60)
#   SOCKS_PORT              default: 19050
#   CTRL_PORT               default: 19051
#   AUTO_PORTS              1 = auto-pick free ports if busy (default: 0)
#   FORCE_TOR               1 = ignore existing Tor, start new one (default: 0)
#   KEEP_TOR                1 = on success, keep Tor running (skip cleanup) (default: 0)
#   QUIET                   1 = less chatter (default: 0)
#   TOR_DEBUG               1 = set Log level to debug (default: 0 -> notice)
#   TOR_BRIDGES             path to file with "Bridge ..." lines
#   TOR_BRIDGES_INLINE      newline-separated "Bridge ..." lines
#   TOR_NO_AUTH             1 = disable cookie auth (control port no-auth) (default: 0)
#   RUN_NODE_E2E            1 = (placeholder) hook to run node e2e after Tor is up (default: 0)
#
# Exit codes:
#   0 ok, 1 logic, 2 bootstrap timeout/stall, 3 port conflict, 4 missing dep, 5 bridge config error

set -euo pipefail

# -------- defaults --------
TOR_BOOTSTRAP_TIMEOUT="${TOR_BOOTSTRAP_TIMEOUT:-180}"
TOR_COOKIE_TIMEOUT="${TOR_COOKIE_TIMEOUT:-5}"
STALL_SECS="${STALL_SECS:-60}"
SOCKS_PORT="${SOCKS_PORT:-19050}"
CTRL_PORT="${CTRL_PORT:-19051}"
AUTO_PORTS="${AUTO_PORTS:-0}"
FORCE_TOR="${FORCE_TOR:-0}"
KEEP_TOR="${KEEP_TOR:-0}"
QUIET="${QUIET:-0}"
TOR_DEBUG="${TOR_DEBUG:-0}"
TOR_NO_AUTH="${TOR_NO_AUTH:-0}"
RUN_NODE_E2E="${RUN_NODE_E2E:-0}"

# -------- logging --------
log() { [[ "$QUIET" == "1" ]] || printf "%s\n" "$*"; }
ok()  { printf "✅ %s\n" "$*"; }
err() { printf "❌ %s\n" "$*" >&2; }
die() { err "$*"; exit 1; }

# -------- prereqs --------
need() { command -v "$1" >/dev/null 2>&1 || { err "missing: $1"; exit 4; }; }
need tor
need nc
need xxd
# obfs4proxy / tor-snowflake discovered lazily if bridges provided

# -------- dirs --------
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%s)-$RANDOM-$RANDOM"
OUT_DIR="$REPO_ROOT/.tor_test_logs/$RUN_ID"
mkdir -p "$OUT_DIR"
TMP_DIR="$(mktemp -d -t tor_test.XXXXXXXX)"
TOR_DATA_DIR="$TMP_DIR/data"
mkdir -p "$TOR_DATA_DIR"

TOR_LOG_START="$OUT_DIR/.tor_server.start.log"
TOR_LOG_BOOT="$OUT_DIR/.tor_server.bootstrap.log"
TOR_CFG_DIR="$TMP_DIR"
TORRCC="$TOR_CFG_DIR/config.testrun.torrc"

log "[tor] temp config: $TORRCC"
log "[tor] node log: $TOR_LOG_BOOT"

# -------- port helpers --------
check_port_free() {
  if command -v lsof >/dev/null 2>&1; then
    ! lsof -nP -iTCP:"$1" -sTCP:LISTEN >/dev/null 2>&1
    return
  fi
  # fallback /dev/tcp (may not work everywhere)
  if (echo >/dev/tcp/127.0.0.1/"$1") >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

pick_free_pair() {
  local s="$1" c d
  for ((d=0; d<200; d++)); do
    s=$(( s + d ))
    c=$(( s + 1 ))
    if check_port_free "$s" && check_port_free "$c"; then
      echo "$s $c"
      return 0
    fi
  done
  return 1
}

# quick control check
ctrl_cmd_nc() {
  local cmd="$1" auth="$2"
  printf "%b" "${auth}${cmd}\r\nQUIT\r\n" \
    | nc -w 2 127.0.0.1 "$CTRL_PORT" 2>/dev/null \
    | tr -d '\r'
}

existing_tor_ok=0
if [[ "$FORCE_TOR" != "1" ]]; then
  if ! check_port_free "$CTRL_PORT"; then
    # Try no-auth ping first (will work if Tor allows), then with AUTHENTICATE
    local_resp="$(ctrl_cmd_nc "GETINFO version" "")" || local_resp=""
    if ! grep -q "^250-version=" <<<"$local_resp"; then
      local_resp="$(printf "AUTHENTICATE\r\nGETINFO version\r\nQUIT\r\n" \
        | nc -w 2 127.0.0.1 "$CTRL_PORT" 2>/dev/null | tr -d '\r' || true)"
    fi
    if grep -q "^250-version=" <<<"$local_resp"; then
      existing_tor_ok=1
      ok "Found a healthy Tor on ControlPort $CTRL_PORT (reuse)."
    else
      if [[ "$AUTO_PORTS" == "1" ]]; then
        read -r SOCKS_PORT CTRL_PORT < <(pick_free_pair 19050) || die "AUTO_PORTS couldn't find free ports"
        log "[tor] AUTO_PORTS picked SOCKS:$SOCKS_PORT CTRL:$CTRL_PORT"
      else
        die "Another process is listening on ControlPort $CTRL_PORT but didn't answer like Tor. Use FORCE_TOR=1, AUTO_PORTS=1, or change CTRL_PORT."
      fi
    fi
  fi
fi

STARTED_TOR=0
TOR_PID=""

# -------- torrc generation --------
make_torrc() {
  local loglevel="notice"
  [[ "$TOR_DEBUG" == "1" ]] && loglevel="debug"

  {
    echo "ClientOnly 1"
    echo "SOCKSPort 127.0.0.1:$SOCKS_PORT"
    echo "ControlPort 127.0.0.1:$CTRL_PORT"
    echo "DataDirectory $TOR_DATA_DIR"
    echo "AvoidDiskWrites 1"
    echo "UseMicrodescriptors 1"
    echo "Log $loglevel file $TOR_LOG_BOOT"
    if [[ "$TOR_NO_AUTH" == "1" ]]; then
      echo "CookieAuthentication 0"
    else
      echo "CookieAuthentication 1"
      echo "CookieAuthFile $TOR_DATA_DIR/control_auth_cookie"
    fi
  } >"$TORRCC"

  # Bridges
  local bridges_tmp="$TMP_DIR/bridges.txt"
  : >"$bridges_tmp"

  if [[ -n "${TOR_BRIDGES:-}" && -f "${TOR_BRIDGES:-}" ]]; then
    grep -E '^\s*Bridge\s' "$TOR_BRIDGES" | sed 's/^[[:space:]]*//' >>"$bridges_tmp" || true
  fi
  if [[ -n "${TOR_BRIDGES_INLINE:-}" ]]; then
    while IFS= read -r line; do
      [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
      [[ "$line" =~ ^[[:space:]]*Bridge[[:space:]] ]] && echo "$line" >>"$bridges_tmp"
    done <<<"$TOR_BRIDGES_INLINE"
  fi

  if [[ -s "$bridges_tmp" ]]; then
    echo "UseBridges 1" >>"$TORRCC"
    # obfs4?
    if grep -qE '^Bridge[[:space:]]+obfs4[[:space:]]' "$bridges_tmp"; then
      OBFS4PROXY_BIN="$(command -v obfs4proxy || true)"
      [[ -z "$OBFS4PROXY_BIN" ]] && { err "obfs4 bridges provided but obfs4proxy not found (brew install obfs4proxy)"; exit 4; }
      echo "ClientTransportPlugin obfs4 exec $OBFS4PROXY_BIN" >>"$TORRCC"
      log "[tor] Using obfs4proxy at: $OBFS4PROXY_BIN"
    fi
    # snowflake? (optional)
    if grep -qE '^Bridge[[:space:]]+snowflake[[:space:]]' "$bridges_tmp"; then
      SNOWFLAKE_BIN="$(command -v tor-snowflake || true)"
      [[ -z "$SNOWFLAKE_BIN" ]] && { err "snowflake bridges provided but tor-snowflake not found"; exit 5; }
      echo "ClientTransportPlugin snowflake exec $SNOWFLAKE_BIN" >>"$TORRCC"
      log "[tor] Using tor-snowflake at: $SNOWFLAKE_BIN"
    fi
    cat "$bridges_tmp" >>"$TORRCC"

    # validate we configured at least one supported transport if bridges present
    if ! grep -qE '^ClientTransportPlugin[[:space:]]+(obfs4|snowflake)[[:space:]]+exec' "$TORRCC"; then
      err "Bridges provided but none are supported (obfs4 or snowflake)."
      exit 5
    fi
  fi
}

# -------- launch tor --------
launch_tor() {
  make_torrc

  if [[ "$existing_tor_ok" == "1" ]]; then
    return
  fi

  # ports
  if ! check_port_free "$SOCKS_PORT" || ! check_port_free "$CTRL_PORT"; then
    if [[ "$AUTO_PORTS" == "1" ]]; then
      read -r SOCKS_PORT CTRL_PORT < <(pick_free_pair 19050) || { err "AUTO_PORTS couldn't find free ports"; exit 3; }
      # Re-write torrc with new ports
      make_torrc
      log "[tor] AUTO_PORTS picked SOCKS:$SOCKS_PORT CTRL:$CTRL_PORT"
    else
      err "Port in use (SOCKS:$SOCKS_PORT or CTRL:$CTRL_PORT)."
      exit 3
    fi
  fi

  log "[tor] starting temporary Tor (SocksPort $SOCKS_PORT, ControlPort $CTRL_PORT)…"
  ( tor -f "$TORRCC" >"$TOR_LOG_START" 2>&1 ) &
  TOR_PID=$!
  STARTED_TOR=1
  sleep 0.3
  kill -0 "$TOR_PID" 2>/dev/null || { err "Tor failed to start. See $TOR_LOG_START"; exit 2; }
}

# -------- control helpers --------
ctrl_auth_cmd=""
prepare_ctrl_auth() {
  if [[ "$TOR_NO_AUTH" == "1" ]]; then
    ctrl_auth_cmd=""
    return
  fi
  local cookie="$TOR_DATA_DIR/control_auth_cookie"
  local t0 tnow
  t0="$(date +%s)"
  while [[ ! -s "$cookie" ]]; do
    sleep 0.1
    tnow="$(date +%s)"
    if (( tnow - t0 > TOR_COOKIE_TIMEOUT )); then
      err "Timed out waiting for control_auth_cookie (>${TOR_COOKIE_TIMEOUT}s)."
      exit 2
    fi
  done
  local hex
  hex="$(xxd -ps "$cookie" | tr -d '\n')"
  ctrl_auth_cmd="AUTHENTICATE $hex\r\n"
}

# cache for bootstrap GETINFO to avoid redundant prints (minor)
_cached_bootstrap=""
ctrl_get_bootstrap() {
  # Only cache for 500ms windows to avoid stale reads
  local now_ns cached_at_ns
  now_ns="$(date +%s%N)"
  if [[ -n "${_cached_bootstrap:-}" && -n "${_cached_bootstrap_at:-}" ]]; then
    cached_at_ns="$_cached_bootstrap_at"
    # if less than ~0.5s old, reuse
    if (( now_ns - cached_at_ns < 500000000 )); then
      printf "%s\n" "$_cached_bootstrap"
      return
    fi
  fi
  local resp
  resp="$(printf "%b" "${ctrl_auth_cmd}GETINFO status/bootstrap-phase\r\nQUIT\r\n" \
      | nc -w 2 127.0.0.1 "$CTRL_PORT" 2>/dev/null | tr -d '\r')"
  _cached_bootstrap="$resp"
  _cached_bootstrap_at="$now_ns"
  printf "%s\n" "$resp"
}

# -------- cleanup --------
cleanup() {
  if [[ "$KEEP_TOR" == "1" ]]; then
    # Leave Tor + temp data; just keep tmp dir intact
    return
  fi
  if [[ "$STARTED_TOR" == "1" && -n "${TOR_PID}" ]]; then
    kill "$TOR_PID" 2>/dev/null || true
    sleep 0.2
  fi
  rm -rf "$TMP_DIR" 2>/dev/null || true
}
trap cleanup EXIT

# -------- bootstrap monitor --------
monitor_bootstrap() {
  prepare_ctrl_auth

  local deadline=$(( $(date +%s) + TOR_BOOTSTRAP_TIMEOUT ))
  local last_progress=-1
  local stall_count=0

  while :; do
    local now prog_line progress summary
    now="$(date +%s)"
    if (( now > deadline )); then
      err "Tor did not bootstrap within ${TOR_BOOTSTRAP_TIMEOUT}s."
      echo "----- recent bootstrap log (last 100 lines) -----"
      tail -n 100 "$TOR_LOG_BOOT" || true
      echo "Hint: If stuck ~50%, try bridges (TOR_BRIDGES / TOR_BRIDGES_INLINE) or set TOR_DEBUG=1."
      exit 2
    fi

    prog_line="$(ctrl_get_bootstrap || true)"
    progress="$(sed -nE 's/.*PROGRESS=([0-9]+).*/\1/p' <<<"$prog_line" | tail -n1)"
    summary="$(sed -nE 's/.*SUMMARY=\"?([^\",]+).*$/\1/p' <<<"$prog_line" | tail -n1)"

    if [[ -z "$progress" ]]; then
      sleep 0.5
      continue
    fi

    if [[ "$progress" == "$last_progress" ]]; then
      stall_count=$((stall_count+1))
      if (( stall_count == STALL_SECS )); then
        log "⚠️  Bootstrap appears stalled at ${progress}%. Recent log:"
        tail -n 20 "$TOR_LOG_BOOT" || true
        log "Hint: Check network or use bridges (obfs4/snowflake). See TOR_BRIDGES* envs."
      fi
    else
      stall_count=0
      last_progress="$progress"
      log "[tor] Bootstrap ${progress%%%}% - ${summary:-...}"
    fi

    if (( progress >= 100 )); then
      ok "Tor bootstrapped (100%)."
      break
    fi

    sleep 1
  done
}

# -------- main --------
launch_tor
monitor_bootstrap

if [[ "$KEEP_TOR" == "1" ]]; then
  ok "KEEP_TOR=1: keeping Tor running."
  ok "SOCKS: 127.0.0.1:$SOCKS_PORT   CTRL: 127.0.0.1:$CTRL_PORT"
  ok "Logs and artifacts in: $OUT_DIR"
else
  ok "Logs and artifacts saved in: $OUT_DIR"
fi

# (Optional) placeholder for node e2e
if [[ "$RUN_NODE_E2E" == "1" ]]; then
  log "[node] RUN_NODE_E2E is not implemented in this script yet (placeholder)."
  # Skeleton for future:
  # cargo run -p node -- --config "$REPO_ROOT/config.sample.toml" serve --transport tor &
  # NODE_PID=$!
  # sleep 5
  # # Do a PUT/GET here...
  # kill "$NODE_PID" 2>/dev/null || true
fi
