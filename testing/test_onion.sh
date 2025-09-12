#!/usr/bin/env bash
# RustyOnions — Onion creation test with HS_DESC wait (macOS/Bash 3.2 safe, no magic sleeps)
set -euo pipefail

#
# How to test
# Terminal A: python3 -m http.server 1777 --bind 127.0.0.1
# Terminal B: DEBUG=1 testing/test_onion.sh
#

# --------------------------- Config ------------------------------------------
TOR_BOOTSTRAP_TIMEOUT="${TOR_BOOTSTRAP_TIMEOUT:-180}"   # seconds to wait for "Bootstrapped 100%"
HS_EVENT_TIMEOUT="${HS_EVENT_TIMEOUT:-60}"              # seconds to wait for HS_DESC UPLOADED
PORT_MIN="${PORT_MIN:-20000}"
PORT_MAX="${PORT_MAX:-30000}"
VIRT_PORT="${VIRT_PORT:-1777}"
TARGET_ADDR="${TARGET_ADDR:-127.0.0.1:1777}"            # local service forwarded by the onion
FOLLOW_LOGS="${FOLLOW_LOGS:-0}"                          # set to 1 to tail -f tor.log live
DEBUG="${DEBUG:-0}"

# --------------------------- UI helpers --------------------------------------
log() { if [[ "${DEBUG}" == "1" ]]; then echo "[test_onion] $*" >&2; fi; }
info() { printf "\033[36m%s\033[0m\n" "$*"; }
ok()   { printf "\033[32m%s\033[0m\n" "$*"; }
err()  { printf "\033[31m%s\033[0m\n" "$*" >&2; }

# --------------------------- Ready helpers -----------------------------------
wait_tcp() { # host port timeout_sec
  local host="$1" port="$2" timeout="${3:-30}" start end
  start=$(date +%s)
  while true; do
    if nc -z "$host" "$port" 2>/dev/null; then return 0; fi
    end=$(date +%s)
    if (( end - start >= timeout )); then
      err "Timeout waiting for TCP ${host}:${port}"
      return 1
    fi
    sleep 0.2
  done
}

wait_log_pattern() { # file regex timeout_sec
  local file="$1" regex="$2" timeout="${3:-60}" start end
  start=$(date +%s)
  while true; do
    if [[ -f "$file" ]] && grep -Eq "$regex" "$file"; then return 0; fi
    end=$(date +%s)
    if (( end - start >= timeout )); then
      err "Timeout waiting for '$regex' in $file"
      return 1
    fi
    sleep 0.2
  done
}

wait_file() { # file timeout_sec
  local file="$1" timeout="${2:-10}" start end
  start=$(date +%s)
  while true; do
    [[ -f "$file" ]] && return 0
    end=$(date +%s)
    if (( end - start >= timeout )); then
      err "Timeout waiting for file to appear: $file"
      return 1
    fi
    sleep 0.2
  done
}

# --------------------------- Port picking (no GNU shuf) -----------------------
rand_in_range() { # min max
  local min="$1"
  local max="$2"
  local span=$(( max - min + 1 ))
  echo $(( (RANDOM % span) + min ))
}

pick_ports() {
  SOCKS_PORT="$(rand_in_range "$PORT_MIN" "$PORT_MAX")"
  CTRL_PORT=$((SOCKS_PORT + 1))
}

# --------------------------- Start Tor ---------------------------------------
pick_ports
DATA_DIR="$(mktemp -d -t tor_onion.XXXXXX)"
LOG_FILE="$DATA_DIR/tor.log"

info "[*] Starting tor"
info "    Socks     : ${SOCKS_PORT}"
info "    Control   : ${CTRL_PORT}"
info "    DataDir   : ${DATA_DIR}"
info "    Tor log   : ${LOG_FILE}"
info "    (hint) tail -f '${LOG_FILE}'  # to watch progress"

tor \
  --RunAsDaemon 0 \
  --SocksPort "127.0.0.1:${SOCKS_PORT}" \
  --ControlPort "127.0.0.1:${CTRL_PORT}" \
  --CookieAuthentication 0 \
  --HashedControlPassword '' \
  --DataDirectory "${DATA_DIR}" \
  --Log "notice file ${LOG_FILE}" \
  >"${LOG_FILE}" 2>&1 &
TOR_PID=$!

TAIL_PID=""
cleanup() {
  info "[*] Cleaning up…"
  [[ -n "$TAIL_PID" ]] && kill "$TAIL_PID" 2>/dev/null || true
  kill "${TOR_PID}" 2>/dev/null || true
  wait "${TOR_PID}" 2>/dev/null || true
  rm -rf "${DATA_DIR}"
}
trap cleanup EXIT INT TERM

# Ensure log file exists; optionally follow it
wait_file "$LOG_FILE" 10
if [[ "$FOLLOW_LOGS" = "1" ]]; then
  # Follow from the beginning so user sees past lines too
  tail -n +1 -F "$LOG_FILE" >&2 &
  TAIL_PID=$!
fi

# Wait for control socket to accept connections, then for bootstrap to 100%.
wait_tcp "127.0.0.1" "${CTRL_PORT}" 30
info "[*] Waiting for Tor bootstrap… (timeout ${TOR_BOOTSTRAP_TIMEOUT}s)"
wait_log_pattern "${LOG_FILE}" 'Bootstrapped 100' "${TOR_BOOTSTRAP_TIMEOUT}"
ok "[*] Tor bootstrapped."

# --------------------------- Control-port helpers ----------------------------
ctrl_send() { # fd string
  printf '%s\r\n' "$2" >&"$1"
}
ctrl_read_line() { # fd timeout_sec
  local fd="$1" to="${2:-5}" line
  if IFS= read -r -t "$to" -u "$fd" line; then
    line="${line%$'\r'}"
    printf '%s\n' "$line"
    return 0
  else
    return 124
  fi
}

# Open control port via Bash /dev/tcp (Bash 3.2 feature)
info "[*] Connecting to control port…"
# shellcheck disable=SC3030
exec 3<>"/dev/tcp/127.0.0.1/${CTRL_PORT}" || { err "[!] Failed to open control port"; exit 1; }
ctrl_read_line 3 1 >/dev/null || true  # ignore banner

ctrl_send 3 'AUTHENTICATE'
line="$(ctrl_read_line 3 5 || true)"
[[ "$line" =~ ^250[[:space:]]*OK$ ]] || { err "[!] AUTHENTICATE failed: ${line:-<no line>}"; exit 1; }
ok "[*] AUTHENTICATE: OK"

# Subscribe to HS_DESC events to see "UPLOADED"
ctrl_send 3 'SETEVENTS HS_DESC'
ctrl_read_line 3 2 >/dev/null || true

# --------------------------- ADD_ONION ---------------------------------------
# Map onion VIRT_PORT -> TARGET_ADDR
ctrl_send 3 "ADD_ONION NEW:ED25519-V3 Flags=DiscardPK Port=${VIRT_PORT},${TARGET_ADDR}"

SERVICE_ID=""
create_deadline=$(( $(date +%s) + 15 ))
while true; do
  line="$(ctrl_read_line 3 2 || true)" || true
  [[ -z "$line" ]] && { [[ $(date +%s) -ge $create_deadline ]] && break || continue; }
  echo "$line"
  if [[ "$line" =~ ^250[[:space:]]*-ServiceID=([a-z2-7]{56})$ ]]; then
    SERVICE_ID="${BASH_REMATCH[1]}"
  fi
  [[ "$line" =~ ^250[[:space:]]*OK$ ]] && break
done

[[ -n "$SERVICE_ID" ]] || { err "[!] Failed to parse ServiceID from ADD_ONION"; exit 1; }
ok "[+] Onion created: ${SERVICE_ID}.onion:${VIRT_PORT}"

# --------------------------- Wait for HS_DESC upload -------------------------
info "[*] Waiting for HS descriptor upload (timeout ${HS_EVENT_TIMEOUT}s)…"
hs_deadline=$(( $(date +%s) + HS_EVENT_TIMEOUT ))
UPLOADED=0
while true; do
  line="$(ctrl_read_line 3 2 || true)" || true
  [[ -n "$line" ]] && echo "$line"
  if [[ -n "$line" && "$line" =~ ^650[[:space:]]+HS_DESC[[:space:]].*UPLOADED.*(${SERVICE_ID}) ]]; then
    UPLOADED=1; break
  fi
  [[ $(date +%s) -ge $hs_deadline ]] && break
done

if (( UPLOADED == 1 )); then
  ok "[+] HS_DESC UPLOADED confirmed for ${SERVICE_ID}"
else
  info "[!] Did not see HS_DESC UPLOADED within ${HS_EVENT_TIMEOUT}s (may still be publishing)"
fi

# --------------------------- Optional reachability probe ---------------------
if command -v curl >/dev/null; then
  for i in 1 2 3; do
    info "[*] curl probe #$i via SOCKS5H (5s)…"
    if curl --max-time 5 --socks5-hostname "127.0.0.1:${SOCKS_PORT}" -sS \
        "http://${SERVICE_ID}.onion:${VIRT_PORT}" >/dev/null; then
      ok "[+] curl probe succeeded"
      break
    else
      info "[!] curl probe timed out"
      sleep 0.2
    fi
  done
fi

ok "[*] Done. (Tor will stop on exit)"
