#!/usr/bin/env bash
# FILE: testing/run_chutney_e2e.sh
# Run RustyOnions onion E2E against a private Tor mini-network (Chutney hs-v3).
# - Creates a Python venv for Chutney and installs deps
# - Shifts ports on all generated torrcs if base ports are busy (or if forced)
# - Starts hs-v3 net and runs testing/test_onion_e2e.sh inside it
# - Bash 3.2 / macOS friendly

set -euo pipefail

LOG_PREFIX="[ron-chutney]"
say() { echo -e "${LOG_PREFIX} $*"; }
die() { echo -e "${LOG_PREFIX} [!] $*" >&2; exit 1; }

# --------------------- Settings ---------------------
CHUTNEY_DIR="${CHUTNEY_DIR:-third_party/chutney}"
CHUTNEY_NET="${CHUTNEY_NET:-networks/hs-v3}"
CHUTNEY_WAIT_SECS="${CHUTNEY_WAIT_SECS:-90}"

TOR_BIN="${TOR_BIN:-$(command -v tor || true)}"
TOR_GENCERT_BIN="${TOR_GENCERT_BIN:-}"

FAST_ONION="${FAST_ONION:-1}"         # 1 = split-mode inside E2E for speed
KEEP_WORK="${KEEP_WORK:-1}"
FOLLOW_LOGS="${FOLLOW_LOGS:-0}"
TOR_LOG_LEVEL="${TOR_LOG_LEVEL:-notice}"

# Python venv for Chutney
CHUTNEY_VENV_DIR="${CHUTNEY_VENV_DIR:-$CHUTNEY_DIR/.venv}"

# Port shifting:
# If base ports (8000/5100/7100) are busy, or if FORCE_CHUTNEY_PORT_OFFSET=1,
# we add this OFFSET to ALL ports in generated torrcs.
CHUTNEY_PORT_OFFSET="${CHUTNEY_PORT_OFFSET:-30000}"
# ----------------------------------------------------

need() { command -v "$1" >/dev/null 2>&1 || die "Missing dependency: $1"; }
need git
need python3
need lsof

# Ensure TOR binaries
if [[ -z "${TOR_BIN:-}" ]]; then
  die "tor not found in PATH and TOR_BIN not set"
fi
if [[ -z "${TOR_GENCERT_BIN:-}" ]]; then
  guess="$(dirname "$TOR_BIN")/tor-gencert"
  if [[ -x "$guess" ]]; then TOR_GENCERT_BIN="$guess"
  else TOR_GENCERT_BIN="$(command -v tor-gencert || true)"; fi
fi
[[ -n "${TOR_GENCERT_BIN:-}" && -x "$TOR_GENCERT_BIN" ]] || die "tor-gencert not found; set TOR_GENCERT_BIN=/path/to/tor-gencert"

# Clone Chutney if missing
if [[ ! -d "$CHUTNEY_DIR" ]]; then
  say "[*] Cloning Chutney into $CHUTNEY_DIR …"
  git clone https://git.torproject.org/chutney.git "$CHUTNEY_DIR"
fi
chmod +x "$CHUTNEY_DIR/chutney"

# Create/activate venv and install deps
if [[ ! -d "$CHUTNEY_VENV_DIR" ]]; then
  say "[*] Creating Python venv at $CHUTNEY_VENV_DIR"
  python3 -m venv "$CHUTNEY_VENV_DIR"
fi
# shellcheck disable=SC1091
source "$CHUTNEY_VENV_DIR/bin/activate"
python -m pip install --upgrade pip >/dev/null
python -m pip install -q tomli-w typing_extensions typeguard PyYAML >/dev/null

# Force chutney to use venv’s Python
CHUTNEY_PY="$(python -c 'import sys; print(sys.executable)')"
export PYTHON="$CHUTNEY_PY"

# Export Tor paths so Chutney uses the exact Tor you use
export CHUTNEY_TOR="$TOR_BIN"
export CHUTNEY_TOR_GENCERT="$TOR_GENCERT_BIN"

# ---------------- Configure network ----------------
say "[*] Configuring Chutney network: $CHUTNEY_NET"
( cd "$CHUTNEY_DIR" && ./chutney configure "$CHUTNEY_NET" )

# Determine nodes dir symlink (chutney links net/nodes -> net/nodes.<seed>)
NODES_LINK="$CHUTNEY_DIR/net/nodes"
[[ -L "$NODES_LINK" || -d "$NODES_LINK" ]] || die "Chutney nodes dir not found after configure"

# Collect torrc files
TORRC_FILES=( "$NODES_LINK"/*/torrc )
[[ ${#TORRC_FILES[@]} -gt 0 ]] || die "No torrc files found under $NODES_LINK"

# Port conflict detection (common bases)
ports_busy=0
for p in 8000 5100 7100; do
  if lsof -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1; then ports_busy=1; break; fi
done

# Helper to bump ports for a given key across files (tolerates leading whitespace)
bump_key() {
  local key="$1"; shift
  K="$key" OFF="$CHUTNEY_PORT_OFFSET" perl -0777 -pe '
    BEGIN { $off = $ENV{"OFF"}+0; $k = $ENV{"K"}; }
    # Case 1: "Key ...:NNN" (host:port style)
    s/^(\s*\Q$k\E\b[^\n]*?:)(\d+)/$1.($2+$off)/emg;
    # Case 2: "Key NNN" (bare port)
    s/^(\s*\Q$k\E\b[^\n]*?\s)(\d+)(\s|$)/$1.($2+$off).$3/emg;
  ' -i -- "$@"
}

# Shift ports if needed or forced
if [[ "$ports_busy" == "1" || "${FORCE_CHUTNEY_PORT_OFFSET:-0}" == "1" ]]; then
  OFF="$CHUTNEY_PORT_OFFSET"
  say "[*] Detected/forced port shift. Shifting ALL ports by +$OFF …"

  # Adjust DirAuthority host:port (IPv4 case; tolerate leading whitespace)
  OFF="$CHUTNEY_PORT_OFFSET" perl -0777 -pe '
    BEGIN { $off = $ENV{"OFF"}+0; }
    s/^(\s*DirAuthority\b.*?:)(\d+)(\b)/$1.($2+$off).$3/emg;
  ' -i -- "${TORRC_FILES[@]}"

  # Bump common listener directives
  for key in ORPort DirPort ControlPort SocksPort ExtORPort DNSPort TransPort NATDPort MetricsPort; do
    bump_key "$key" "${TORRC_FILES[@]}"
  done

  say "[*] Port shift complete."
else
  say "[*] No common base-port conflicts detected; using default hs-v3 ports."
fi

# ---------------- Start network ----------------
say "[*] Starting Chutney network…"
( cd "$CHUTNEY_DIR" && ./chutney start "$CHUTNEY_NET" )

cleanup() {
  set +e
  say "[*] Stopping Chutney network…"
  ( cd "$CHUTNEY_DIR" && ./chutney stop "$CHUTNEY_NET" ) >/dev/null 2>&1 || true
}
trap cleanup EXIT

say "[*] Waiting up to ${CHUTNEY_WAIT_SECS}s for network bootstrap…"
( cd "$CHUTNEY_DIR" && ./chutney status --wait-for-bootstrap "$CHUTNEY_WAIT_SECS" "$CHUTNEY_NET" )

# ---------------- Build TOR_EXTRA_FLAGS ----------------
TMP_DIR="$(mktemp -d -t ron_chutney.XXXXXX)"
DA_TXT="$TMP_DIR/dir_authorities.txt"
DA_FLAGS_FILE="$TMP_DIR/tor_extra_flags.txt"

grep -h '^DirAuthority ' "${TORRC_FILES[@]}" | sort -u > "$DA_TXT" || true
if [[ ! -s "$DA_TXT" ]]; then
  grep -R -h '^DirAuthority ' "$NODES_LINK" | sort -u > "$DA_TXT" || true
fi
[[ -s "$DA_TXT" ]] || die "Failed to extract DirAuthority lines from Chutney network."

sed 's/^DirAuthority /--DirAuthority /' "$DA_TXT" > "$DA_FLAGS_FILE"
DA_LINES="$(tr '\n' ' ' < "$DA_FLAGS_FILE")"
[[ -n "$DA_LINES" ]] || die "Empty DirAuthority flag set."

# Extra testing flags to keep traffic purely on the private net
TESTING_FLAGS="--TestingTorNetwork 1 --UseDefaultFallbackDirs 0 --UseDefaultBridges 0 --AssumeReachable 1"

say "[*] Using TOR_EXTRA_FLAGS from Chutney (truncated): $(echo "$DA_LINES" | cut -c1-160)…"

# ---------------- Run your E2E ----------------
export FAST_ONION
export KEEP_WORK
export FOLLOW_LOGS
export TOR_LOG_LEVEL
export TOR_BIN

TOR_EXTRA_FLAGS="$TESTING_FLAGS $DA_LINES" \
bash testing/test_onion_e2e.sh
