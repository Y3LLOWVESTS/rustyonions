#!/usr/bin/env bash
# RustyOnions — Onion creation test with HS_DESC wait
set -euo pipefail

TOR_BOOTSTRAP_TIMEOUT="${TOR_BOOTSTRAP_TIMEOUT:-180}"
HS_EVENT_TIMEOUT="${HS_EVENT_TIMEOUT:-60}"   # how long to wait for HS_DESC UPLOADED
PORT_MIN="${PORT_MIN:-20000}"
PORT_MAX="${PORT_MAX:-30000}"
VIRT_PORT="${VIRT_PORT:-1777}"
TARGET_ADDR="${TARGET_ADDR:-127.0.0.1:1777}"  # local service to forward to

pick_port() { shuf -i "${PORT_MIN}-${PORT_MAX}" -n1; }
SOCKS_PORT=$(pick_port)
CTRL_PORT=$((SOCKS_PORT + 1))

DATA_DIR="$(mktemp -d -t tor_onion.XXXXXX)"
LOG_FILE="$DATA_DIR/tor.log"

echo "[*] Starting tor (Socks=${SOCKS_PORT}, Control=${CTRL_PORT}, DataDir=${DATA_DIR})…"
tor \
  --RunAsDaemon 0 \
  --SocksPort "127.0.0.1:${SOCKS_PORT}" \
  --ControlPort "127.0.0.1:${CTRL_PORT}" \
  --CookieAuthentication 0 \
  --HashedControlPassword '' \
  --DataDirectory "${DATA_DIR}" \
  >"${LOG_FILE}" 2>&1 &
TOR_PID=$!

cleanup() {
  echo "[*] Cleaning up…"
  kill "${TOR_PID}" 2>/dev/null || true
  rm -rf "${DATA_DIR}"
}
trap cleanup EXIT

echo "[*] Waiting for Tor bootstrap (timeout ${TOR_BOOTSTRAP_TIMEOUT}s)…"
SECONDS=0
while (( SECONDS < TOR_BOOTSTRAP_TIMEOUT )); do
  if grep -q 'Bootstrapped 100' "${LOG_FILE}"; then
    echo "[*] Tor bootstrapped."
    break
  fi
  sleep 2
done
if ! grep -q 'Bootstrapped 100' "${LOG_FILE}"; then
  echo "[!] Tor did not bootstrap within timeout"
  tail -n 80 "${LOG_FILE}" || true
  exit 1
fi

# ---- Control-port helpers (/dev/tcp) ----
ctrl_send() { printf '%s\r\n' "$2" >&"$1"; }
ctrl_read_line() { local fd="$1" to="${2:-5}" line; if IFS= read -r -t "$to" -u "$fd" line; then printf '%s\n' "$line"; return 0; else return 124; fi; }

echo "[*] Connecting to control port…"
exec 3<>"/dev/tcp/127.0.0.1/${CTRL_PORT}" || { echo "[!] Failed to open control port"; exit 1; }
ctrl_read_line 3 1 >/dev/null || true  # ignore banner

ctrl_send 3 'AUTHENTICATE'
line="$(ctrl_read_line 3 5 || true)"; line="${line%$'\r'}"
[[ "$line" =~ ^250[[:space:]]*OK$ ]] || { echo "[!] AUTHENTICATE failed: ${line:-<no line>}"; exit 1; }
echo "[*] AUTHENTICATE: OK"

# Subscribe to HS_DESC events now (so we can see UPLOADED)
ctrl_send 3 'SETEVENTS HS_DESC'
ctrl_read_line 3 2 >/dev/null || true

# Create onion
ctrl_send 3 "ADD_ONION NEW:ED25519-V3 Flags=DiscardPK Port=${VIRT_PORT},${TARGET_ADDR}"

SERVICE_ID=""
deadline=$((SECONDS + 10))
while (( SECONDS < deadline )); do
  line="$(ctrl_read_line 3 2 || true)"; [[ -z "$line" ]] && continue
  line="${line%$'\r'}"; echo "$line"
  if [[ "$line" =~ ^250[[:space:]]*-ServiceID=([a-z2-7]{56})$ ]]; then
    SERVICE_ID="${BASH_REMATCH[1]}"
  fi
  [[ "$line" =~ ^250[[:space:]]*OK$ ]] && break
done
[[ -n "$SERVICE_ID" ]] || { echo "[!] Failed to parse ServiceID"; exit 1; }

echo "[+] Onion created: ${SERVICE_ID}.onion:${VIRT_PORT}"
echo "[*] Waiting for HS descriptor upload (timeout ${HS_EVENT_TIMEOUT}s)…"

# Wait for 650 HS_DESC ... UPLOADED that mentions our service
hs_deadline=$((SECONDS + HS_EVENT_TIMEOUT))
UPLOADED=0
while (( SECONDS < hs_deadline )); do
  line="$(ctrl_read_line 3 2 || true)" || true
  [[ -z "$line" ]] && continue
  line="${line%$'\r'}"; echo "$line"
  # Be permissive: look for '650 HS_DESC' + 'UPLOADED' + our service id
  if [[ "$line" =~ ^650[[:space:]]+HS_DESC[[:space:]].*UPLOADED.*(${SERVICE_ID}) ]]; then
    UPLOADED=1
    break
  fi
done

if (( UPLOADED == 1 )); then
  echo "[+] HS_DESC UPLOADED confirmed for ${SERVICE_ID}"
else
  echo "[!] Did not see HS_DESC UPLOADED event within ${HS_EVENT_TIMEOUT}s (may still be publishing)"
fi

# Best-effort reachability probe (retry a few times)
if command -v curl >/dev/null; then
  for i in 1 2 3; do
    echo "[*] curl probe #$i via socks (5s)…"
    if curl --max-time 5 --socks5-hostname "127.0.0.1:${SOCKS_PORT}" -sS "http://${SERVICE_ID}.onion:${VIRT_PORT}" >/dev/null; then
      echo "[+] curl probe succeeded"
      break
    else
      echo "[!] curl probe timed out"; sleep 5
    fi
  done
fi

echo "[*] Done. (Tor will stop on exit)"
