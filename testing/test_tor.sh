#!/usr/bin/env bash
# RustyOnions — Tor bootstrap + control channel smoke test (plain cookie auth)
# macOS Bash 3.2 compatible

set -u
: "${QUIET:=0}"
: "${STAY_UP:=0}"
: "${TOR_BOOTSTRAP_TIMEOUT:=180}"
: "${SOCKS_PORT:=auto}"
: "${CTRL_PORT:=auto}"
: "${TOR_BRIDGES:=}"
: "${TOR_BRIDGES_INLINE:=}"
: "${OBFS4_PROXY:=}"
: "${RO_DATA_DIR:=.data}"
: "${DO_E2E:=0}"

log()  { [ "$QUIET" = "1" ] || echo "[*] $*"; }
warn() { echo "[!] $*" >&2; }
die()  { warn "$*"; exit 1; }

# ---------- preflight ----------
need() { command -v "$1" >/dev/null 2>&1 || die "Missing dependency: $1"; }
need tor
need nc
need xxd

# ---------- helpers ----------
find_free_port() {
  for p in $(seq 19050 19999); do
    nc -z 127.0.0.1 "$p" >/dev/null 2>&1 || { echo "$p"; return 0; }
  done
  return 1
}

wait_for_file() {
  # wait_for_file <path> <timeout_sec>
  local f="$1" t="$2" i=0
  while [ ! -f "$f" ] && [ $i -lt "$t" ]; do
    sleep 1; i=$((i+1))
  done
  [ -f "$f" ]
}

# ---------- workspace ----------
WORK_ROOT="$(mktemp -d -t tor_test.XXXXXXXX)"
CFG_DIR="$WORK_ROOT/testrun"
mkdir -p "$CFG_DIR" || die "failed to mkdir $CFG_DIR"

TOR_DIR_ROOT="${RO_DATA_DIR:-.data}"
mkdir -p "$TOR_DIR_ROOT" || true

TOR_DATA_DIR="$WORK_ROOT/tor"
mkdir -p "$TOR_DATA_DIR"

TOR_LOG="$WORK_ROOT/tor.log"
TOR_RUNLOG="$WORK_ROOT/tor_run.log"
COOKIE_FILE="$TOR_DATA_DIR/control_auth_cookie"

# ---------- ports (ensure different) ----------
if [ "$SOCKS_PORT" = "auto" ]; then
  SOCKS_PORT="$(find_free_port)" || die "No free SocksPort found"
fi
if [ "$CTRL_PORT" = "auto" ]; then
  while :; do
    CANDIDATE="$(find_free_port)" || die "No free ControlPort found"
    [ "$CANDIDATE" != "$SOCKS_PORT" ] && CTRL_PORT="$CANDIDATE" && break
  done
fi
[ "$SOCKS_PORT" != "$CTRL_PORT" ] || die "SocksPort ($SOCKS_PORT) and ControlPort ($CTRL_PORT) must be different"

# ---------- torrc ----------
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

# Optional bridges / pluggable transports
if [ -n "$TOR_BRIDGES" ] && [ -f "$TOR_BRIDGES" ]; then
  {
    echo "UseBridges 1"
    cat "$TOR_BRIDGES"
  } >>"$TORRC"
fi
if [ -n "$TOR_BRIDGES_INLINE" ]; then
  echo "UseBridges 1" >>"$TORRC"
  printf "%s\n" "$TOR_BRIDGES_INLINE" >>"$TORRC"
fi
[ -n "$OBFS4_PROXY" ] && echo "ClientTransportPlugin obfs4 exec $OBFS4_PROXY" >>"$TORRC"

# ---------- start tor ----------
log "Starting tor (Socks=$SOCKS_PORT, Control=$CTRL_PORT)…"
( tor -f "$TORRC" ) >"$TOR_RUNLOG" 2>&1 &
TOR_PID=$!

sleep 0.8
if ! kill -0 "$TOR_PID" 2>/dev/null; then
  warn "Tor failed to start. Recent output:"
  tail -n +1 "$TOR_RUNLOG" 2>/dev/null | tail -n 40 >&2 || true
  [ -f "$TOR_LOG" ] && { echo "--- tor.log ---" >&2; tail -n 60 "$TOR_LOG" >&2; }
  die "Start failure (ports in use? permissions?)."
fi

cleanup() {
  if [ "$STAY_UP" = "1" ]; then
    log "STAY_UP=1 → leaving processes running."
    log "  Tor log:     $TOR_LOG"
    log "  ControlPort: 127.0.0.1:$CTRL_PORT"
    log "  SocksPort:   127.0.0.1:$SOCKS_PORT"
  else
    if [ -n "${TOR_PID:-}" ] && kill -0 "$TOR_PID" 2>/dev/null; then
      kill "$TOR_PID" 2>/dev/null || true
      sleep 0.8
      kill -9 "$TOR_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK_ROOT" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# ---------- bootstrap wait ----------
BOOT_TIMEOUT="$TOR_BOOTSTRAP_TIMEOUT"
log "Waiting for Tor bootstrap (timeout ${BOOT_TIMEOUT}s)…"
i=0
last_print=""
while [ $i -lt "$BOOT_TIMEOUT" ]; do
  if [ -f "$TOR_LOG" ]; then
    if grep -q "Bootstrapped 100%" "$TOR_LOG" 2>/dev/null; then
      log "Tor bootstrapped."
      break
    fi
    cur="$(grep -E 'Bootstrapped [0-9]+' "$TOR_LOG" | tail -n 1 || true)"
    if [ -n "$cur" ] && [ "$cur" != "$last_print" ]; then
      echo "[*] $cur"
      last_print="$cur"
    fi
  fi
  sleep 1
  i=$((i+1))
done
[ $i -lt "$BOOT_TIMEOUT" ] || die "Timed out waiting for Tor bootstrap. See $TOR_LOG and $TOR_RUNLOG"

wait_for_file "$COOKIE_FILE" 10 || die "control_auth_cookie not found at $COOKIE_FILE"
COOKIE_HEX="$(xxd -p -c 256 "$COOKIE_FILE" | tr -d '\r\n')"
[ -n "$COOKIE_HEX" ] || die "Failed to read control cookie"

# ---------- control helper (plain cookie auth) ----------
tor_ctrl() {
  # tor_ctrl "GETINFO status/bootstrap-phase"  (example)
  local cmd="$1"
  {
    printf 'AUTHENTICATE %s\n' "$COOKIE_HEX"
    printf '%s\n' "$cmd"
    printf 'QUIT\n'
  } | nc -w 10 127.0.0.1 "$CTRL_PORT" >"$CFG_DIR/cmd.txt" 2>"$CFG_DIR/cmd_err.txt" || {
    warn "nc failed for control command"
    [ -s "$CFG_DIR/cmd_err.txt" ] && cat "$CFG_DIR/cmd_err.txt" >&2
    return 1
  }

  # Did auth succeed?
  if ! grep -q '^250 OK' "$CFG_DIR/cmd.txt"; then
    warn "Control command failed"
    cat "$CFG_DIR/cmd.txt" >&2
    return 1
  fi

  cat "$CFG_DIR/cmd.txt"
  return 0
}

# ---- Show bootstrap phase via control port ----
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

# Optional sanity: NEWNYM (non-fatal)
tor_ctrl 'SIGNAL NEWNYM' >/dev/null 2>&1 || warn "SIGNAL NEWNYM failed (non-fatal)"

log "Done."
