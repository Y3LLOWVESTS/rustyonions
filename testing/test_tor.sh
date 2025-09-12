#!/usr/bin/env bash
# RustyOnions — Tor bootstrap + control channel smoke test (cookie auth)
# macOS/Bash 3.2 compatible; no sleeps >= 0.5s; supports live log follow.
#
# How to test
#
# bash testing/test_tor.sh
#

set -Eeuo pipefail

# --------------------------- Config ------------------------------------------
: "${QUIET:=0}"                   # 0 = print logs, 1 = quiet
: "${STAY_UP:=0}"                 # 1 = leave Tor running after script
: "${FOLLOW_LOGS:=0}"             # 1 = tail -F tor.log while running
: "${TOR_BOOTSTRAP_TIMEOUT:=300}" # bumped to reduce spurious timeouts
: "${SOCKS_PORT:=auto}"
: "${CTRL_PORT:=auto}"
: "${TOR_BRIDGES:=}"              # path to bridges file (optional)
: "${TOR_BRIDGES_INLINE:=}"       # inline "Bridge ..." lines (optional)
: "${OBFS4_PROXY:=}"              # path to obfs4proxy (optional)
: "${RO_DATA_DIR:=.data}"         # parity

# --------------------------- UI ----------------------------------------------
log()  { [ "$QUIET" = "1" ] || printf "[*] %s\n" "$*"; }
warn() { printf "[!] %s\n" "$*" >&2; }
die()  { warn "$*"; exit 1; }

# --------------------------- Preflight ---------------------------------------
need() { command -v "$1" >/dev/null 2>&1 || die "Missing dependency: $1"; }
need tor
need nc
if ! command -v xxd >/dev/null 2>&1; then
  need hexdump
  HEXDUMP_FALLBACK=1
else
  HEXDUMP_FALLBACK=0
fi

# --------------------------- Helpers (bounded waits) --------------------------
is_port_free() { # host port
  local h="$1" p="$2"
  if nc -z "$h" "$p" >/dev/null 2>&1; then
    return 1  # occupied
  else
    return 0  # free
  fi
}

find_free_port_range() { # start end -> echo free port
  local start="$1" end="$2" p
  for p in $(seq "$start" "$end"); do
    if is_port_free 127.0.0.1 "$p"; then echo "$p"; return 0; fi
  done
  return 1
}

find_free_port_exclude() { # start end exclude -> echo free port != exclude
  local start="$1" end="$2" exclude="$3" p
  for p in $(seq "$start" "$end"); do
    [ "$p" = "$exclude" ] && continue
    if is_port_free 127.0.0.1 "$p"; then echo "$p"; return 0; fi
  done
  return 1
}

wait_for_file() { # path timeout_sec
  local f="$1" t="$2" start end
  start=$(date +%s)
  while [ ! -f "$f" ]; do
    end=$(date +%s)
    if [ $((end-start)) -ge "$t" ]; then return 1; fi
    sleep 0.2
  done
  return 0
}

wait_tcp() { # host port timeout_sec
  local h="$1" p="$2" t="${3:-90}" start end
  start=$(date +%s)
  while true; do
    if nc -z "$h" "$p" 2>/dev/null; then return 0; fi
    end=$(date +%s)
    if [ $((end-start)) -ge "$t" ]; then return 1; fi
    sleep 0.2
  done
}

wait_pid_gone() { # pid timeout_sec
  local pid="$1" t="${2:-5}" start end
  start=$(date +%s)
  while kill -0 "$pid" 2>/dev/null; do
    end=$(date +%s)
    if [ $((end-start)) -ge "$t" ]; then return 1; fi
    sleep 0.2
  done
  return 0
}

# --------------------------- Workspace ---------------------------------------
WORK_ROOT="$(mktemp -d -t tor_test.XXXXXXXX)"
CFG_DIR="$WORK_ROOT/testrun"; mkdir -p "$CFG_DIR"
TOR_DATA_DIR="$WORK_ROOT/tor"; mkdir -p "$TOR_DATA_DIR"
TOR_LOG="$WORK_ROOT/tor.log"
TOR_RUNLOG="$WORK_ROOT/tor_run.log"
COOKIE_FILE="$TOR_DATA_DIR/control_auth_cookie"
TAIL_PID=""

on_error() {
  warn "Error occurred; last log lines:"
  if [ -f "$TOR_RUNLOG" ]; then echo "--- tor_run.log ---" >&2; tail -n 80 "$TOR_RUNLOG" >&2 || true; fi
  if [ -f "$TOR_LOG" ]; then echo "--- tor.log ---" >&2; tail -n 120 "$TOR_LOG" >&2 || true; fi
}
trap on_error ERR

# --------------------------- Ports (auto, robust) ----------------------------
PORT_MIN=19050
PORT_MAX=19999

if [ "$SOCKS_PORT" = "auto" ]; then
  SOCKS_PORT="$(find_free_port_range "$PORT_MIN" "$PORT_MAX")" || die "No free SocksPort found"
else
  is_port_free 127.0.0.1 "$SOCKS_PORT" || die "SocksPort $SOCKS_PORT is in use"
fi

if [ "$CTRL_PORT" = "auto" ]; then
  CTRL_PORT="$(find_free_port_exclude "$PORT_MIN" "$PORT_MAX" "$SOCKS_PORT")" || die "No free ControlPort found"
else
  [ "$CTRL_PORT" != "$SOCKS_PORT" ] || die "SocksPort and ControlPort must differ"
  is_port_free 127.0.0.1 "$CTRL_PORT" || die "ControlPort $CTRL_PORT is in use"
fi

# --------------------------- torrc -------------------------------------------
TORRC="$WORK_ROOT/torrc"
cat >"$TORRC" <<EOF
DataDirectory $TOR_DATA_DIR
SocksPort 127.0.0.1:$SOCKS_PORT
ControlPort 127.0.0.1:$CTRL_PORT
CookieAuthentication 1
CookieAuthFile $COOKIE_FILE
CookieAuthFileGroupReadable 1
Log notice file $TOR_LOG
EOF

if [ -n "$TOR_BRIDGES" ] && [ -f "$TOR_BRIDGES" ]; then
  echo "UseBridges 1" >>"$TORRC"
  cat "$TOR_BRIDGES" >>"$TORRC"
fi
if [ -n "$TOR_BRIDGES_INLINE" ]; then
  echo "UseBridges 1" >>"$TORRC"
  printf "%s\n" "$TOR_BRIDGES_INLINE" >>"$TORRC"
fi
if [ -n "$OBFS4_PROXY" ]; then
  echo "ClientTransportPlugin obfs4 exec $OBFS4_PROXY" >>"$TORRC"
fi

# --------------------------- Start Tor ----------------------------------------
log "Starting tor (Socks=$SOCKS_PORT, Control=$CTRL_PORT)…"
log "Work root : $WORK_ROOT"
log "tor.log   : $TOR_LOG"
log "run log   : $TOR_RUNLOG"
log "hint      : tail -f '$TOR_LOG'   # live bootstrap"

( tor -f "$TORRC" ) >"$TOR_RUNLOG" 2>&1 &
TOR_PID=$!

if [ "$FOLLOW_LOGS" = "1" ]; then
  tail -n +1 -F "$TOR_LOG" >&2 &
  TAIL_PID=$!
fi

kill -0 "$TOR_PID" 2>/dev/null || die "Tor failed to start (see $TOR_RUNLOG)"
wait_for_file "$TOR_LOG" 10 || warn "tor.log not created yet (will continue)"
wait_tcp 127.0.0.1 "$CTRL_PORT" 90 || warn "ControlPort not ready yet (continuing to bootstrap wait)"

cleanup() {
  if [ -n "$TAIL_PID" ]; then kill "$TAIL_PID" 2>/dev/null || true; fi
  if [ "$STAY_UP" = "1" ]; then
    log "STAY_UP=1 — leaving Tor running."
    log "  ControlPort : 127.0.0.1:$CTRL_PORT"
    log "  SocksPort   : 127.0.0.1:$SOCKS_PORT"
    log "  tor.log     : $TOR_LOG"
    log "  run log     : $TOR_RUNLOG"
  else
    if kill -0 "$TOR_PID" 2>/dev/null; then
      kill "$TOR_PID" 2>/dev/null || true
      wait_pid_gone "$TOR_PID" 5 || kill -9 "$TOR_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK_ROOT" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# --------------------------- Bootstrap wait -----------------------------------
BOOT_DEADLINE=$(( $(date +%s) + TOR_BOOTSTRAP_TIMEOUT ))
last=""
log "Waiting for Tor bootstrap (timeout ${TOR_BOOTSTRAP_TIMEOUT}s)…"

while :; do
  if [ -f "$TOR_LOG" ] && grep -q "Bootstrapped 100%" "$TOR_LOG" 2>/dev/null; then
    log "Tor bootstrapped."
    break
  fi
  if [ -f "$TOR_LOG" ]; then
    cur="$(grep -E 'Bootstrapped [0-9]+' "$TOR_LOG" | tail -n 1 || true)"
    if [ -n "$cur" ] && [ "$cur" != "$last" ]; then
      printf "[*] %s\n" "$cur"
      last="$cur"
    fi
  fi
  now_epoch="$(date +%s)"
  if [ "$now_epoch" -ge "$BOOT_DEADLINE" ]; then
    die "Timed out waiting for bootstrap. See $TOR_LOG and $TOR_RUNLOG"
  fi
  sleep 0.2
done

# --------------------------- Cookie + Control ---------------------------------
if ! wait_for_file "$COOKIE_FILE" 10; then
  die "control_auth_cookie not found at $COOKIE_FILE"
fi
if [ "$HEXDUMP_FALLBACK" = "1" ]; then
  COOKIE_HEX="$(hexdump -v -e '/1 "%02x"' "$COOKIE_FILE" | tr -d '\r\n')"
else
  COOKIE_HEX="$(xxd -p -c 256 "$COOKIE_FILE" | tr -d '\r\n')"
fi
[ -n "$COOKIE_HEX" ] || die "Failed to read control cookie"

tor_ctrl() { # send control command via nc (cookie auth)
  local cmd="$1"
  {
    printf 'AUTHENTICATE %s\n' "$COOKIE_HEX"
    printf '%s\n' "$cmd"
    printf 'QUIT\n'
  } | nc -w 10 127.0.0.1 "$CTRL_PORT" >"$CFG_DIR/cmd.txt" 2>"$CFG_DIR/cmd_err.txt" || {
    warn "nc failed for control command"
    if [ -s "$CFG_DIR/cmd_err.txt" ]; then cat "$CFG_DIR/cmd_err.txt" >&2; fi
    return 1
  }
  if ! grep -q '^250 OK' "$CFG_DIR/cmd.txt"; then
    warn "Control command failed"
    cat "$CFG_DIR/cmd.txt" >&2
    return 1
  }
  cat "$CFG_DIR/cmd.txt"
  return 0
}

if ! rsp="$(tor_ctrl 'GETINFO status/bootstrap-phase')" ; then
  warn "tor_ctrl failed for bootstrap-phase"
  echo "See logs:"
  echo "  $TOR_LOG"
  echo "  $TOR_RUNLOG"
  exit 1
fi

if echo "$rsp" | grep -q '^250-status/bootstrap-phase='; then
  echo "$rsp" | sed -n 's/^250-status\/bootstrap-phase=//p'
else
  echo "$rsp"
fi

tor_ctrl 'SIGNAL NEWNYM' >/dev/null 2>&1 || warn "SIGNAL NEWNYM failed (non-fatal)"

log "Done."
