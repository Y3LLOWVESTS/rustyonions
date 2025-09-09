#!/usr/bin/env bash
# Purpose: Concurrency/Aliasing invariants gate for CI.
# - FAILS on:
#     * runtime singletons in crates/ ("static mut" | "lazy_static!")
#     * long sleeps (>=0.5s) in testing/ unless:
#         - the line has '# allow-sleep', or
#         - the file is listed in testing/ci_sleep_allowlist.txt
#     * docs that mention SHA-256 (we are BLAKE3-only)
#     * OAP-1 spec missing max_frame = 1 MiB
# - WARNS on (heuristics):
#     * await while holding lock patterns (with tightened regex)
#     * use of tokio::sync::broadcast::Receiver
#     * Arc<(Mutex|RwLock)> occurrences
#     * tokio::io::split
#     * Prometheus register_* with unwrap|expect
# Bash 3.2 compatible.

set -euo pipefail

fail() { echo "[CI-INVARIANTS][FAIL] $*" >&2; exit 1; }
warn() { echo "[CI-INVARIANTS][WARN] $*" >&2; }
ok()   { echo "[CI-INVARIANTS] $*"; }

# --- robust repo-root detection (works from any cwd) --------------------------
if git rev-parse --show-toplevel >/dev/null 2>&1; then
  ROOT_DIR="$(git rev-parse --show-toplevel)"
else
  SCRIPT="$0"
  case "$SCRIPT" in
    /*) ABS="$SCRIPT" ;;
    *)  ABS="$PWD/$SCRIPT" ;;
  esac
  SCRIPT_DIR="$(cd "$(dirname "$ABS")" && pwd)"
  ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
fi

cd "$ROOT_DIR"
TESTING_DIR="$ROOT_DIR/testing"

normalize_path() {
  local p="$1"
  while printf '%s' "$p" | grep -q '/testing/testing/'; do
    p="${p//\/testing\/testing\//\/testing\/}"
  done
  p="$(printf '%s' "$p" | sed -e 's#\([^:]\)//#\1/#g')"
  echo "$p"
}

echo "[CI-INVARIANTS] ripgrep: $(rg --version | head -n1)"
echo "[CI-INVARIANTS] pwd: $(pwd)"

# 1) No global mutable singletons in runtime code.
if rg -n "static mut|lazy_static!" -S "$ROOT_DIR/crates" >/dev/null 2>&1; then
  rg -n "static mut|lazy_static!" -S "$ROOT_DIR/crates"
  fail "Found 'static mut' or 'lazy_static!' in runtime code under crates/. Use OnceLock/OnceCell + Arc instead."
fi
ok "No 'static mut' or 'lazy_static!' under crates/"

# 2) Long sleeps gate (>= 0.5s) in testing/ with allowlist and annotation.
ALLOWLIST_FILE="$TESTING_DIR/ci_sleep_allowlist.txt"
ALLOWLIST=""
if [ -f "$ALLOWLIST_FILE" ]; then
  ALLOWLIST="$(grep -vE '^\s*(#|$)' "$ALLOWLIST_FILE" || true)"
  echo "[CI-INVARIANTS] Sleep allowlist loaded"
else
  echo "[CI-INVARIANTS] No sleep allowlist found (expected at testing/ci_sleep_allowlist.txt)."
fi

SLEEP_TMP="$(mktemp)"
RG_PATTERN='^[^#]*\bsleep\s+(?:[1-9][0-9]*(?:\.[0-9]+)?|0\.(?:5|[6-9][0-9]*))\b'
rg -n --pcre2 "$RG_PATTERN" "$TESTING_DIR" -S \
  -g '!lib/ready.sh' \
  -g '!tools/**' \
  -g '!**/annotate_allow_sleep*.sh' \
  > "$SLEEP_TMP" || true

# Drop lines already annotated with '# allow-sleep'
FILTERED_TMP="$(mktemp)"
> "$FILTERED_TMP"
if [ -s "$SLEEP_TMP" ]; then
  while IFS= read -r LINE; do
    case "$LINE" in
      *allow-sleep*) : ;;
      *) echo "$LINE" >> "$FILTERED_TMP" ;;
    esac
  done < "$SLEEP_TMP"
fi

# Remove allowlisted files (with normalization + basename match)
VIOL_TMP="$(mktemp)"
> "$VIOL_TMP"
if [ -s "$FILTERED_TMP" ]; then
  while IFS= read -r LINE; do
    PATH_PART_RAW="${LINE%%:*}"
    PATH_PART="$(normalize_path "$PATH_PART_RAW")"
    ALLOWED_FILE="no"
    if [ -n "$ALLOWLIST" ]; then
      while IFS= read -r AL; do
        [ -z "$AL" ] && continue
        AL_NORM="$(normalize_path "$AL")"
        if [ "$PATH_PART" = "$AL_NORM" ] || [ "$(basename "$PATH_PART")" = "$(basename "$AL_NORM")" ]; then
          ALLOWED_FILE="yes"
          break
        fi
      done <<< "$ALLOWLIST"
    fi
    if [ "$ALLOWED_FILE" = "no" ]; then
      echo "$LINE" >> "$VIOL_TMP"
    fi
  done < "$FILTERED_TMP"
fi

if [ -s "$VIOL_TMP" ]; then
  cat "$VIOL_TMP"
  rm -f "$SLEEP_TMP" "$FILTERED_TMP" "$VIOL_TMP"
  fail "Replace these long sleeps (>=0.5s) with testing/lib/ready.sh polling helpers or annotate with '# allow-sleep' while migrating."
fi

rm -f "$SLEEP_TMP" "$FILTERED_TMP" "$VIOL_TMP"
ok "No non-allowlisted long sleeps without annotation"

# 3) Docs: hashing/addressing language (BLAKE3-only; no stray SHA-256).
if rg -n 'sha-?256' -S "$ROOT_DIR/README.md" "$ROOT_DIR/docs" >/dev/null 2>&1; then
  rg -n 'sha-?256' -S "$ROOT_DIR/README.md" "$ROOT_DIR/docs"
  fail "Docs mention SHA-256. Update wording to BLAKE3 (b3:<hex>) or move SHA-256 to 'legacy/compat' context for scripts only."
fi
if ! rg -n 'b3:' -S "$ROOT_DIR/README.md" "$ROOT_DIR/docs" >/dev/null 2>&1; then
  warn "Docs do not visibly mention 'b3:'. Ensure addressing text shows BLAKE3 (b3:<hex>)."
else
  ok "Docs reference BLAKE3 addressing (b3:<hex>)"
fi

# 4) OAP/1 protocol bound must be explicit in spec.
if ! rg -n 'max_frame\s*=\s*1\s*Mi?B' -S "$ROOT_DIR/docs" >/dev/null 2>&1; then
  fail "OAP-1 spec/docs must include: max_frame = 1 MiB."
fi
ok "OAP-1 max_frame bound present in docs"

# 5) Heuristics (WARN)
# Tightened pattern: only warn if Mutex/RwLock is near a lock/read/write call and an .await nearby.
TIGHT_LOCK_AWAIT='((Mutex|RwLock)[^;\n]{0,200}\.(lock|read|write)\(\)[^;\n]{0,200}\.await)|(\.await[^;\n]{0,200}(Mutex|RwLock)[^;\n]{0,200}\.(lock|read|write)\()'
if rg -nU --pcre2 "$TIGHT_LOCK_AWAIT" -S "$ROOT_DIR/crates" >/dev/null 2>&1; then
  rg -nU --pcre2 "$TIGHT_LOCK_AWAIT" -S "$ROOT_DIR/crates"
  warn "Potential await-while-holding-lock (Mutex/RwLock) patterns found (heuristic). Review these."
else
  ok "No obvious await-while-holding-lock (Mutex/RwLock) patterns (heuristic)"
fi

if rg -n 'tokio::sync::broadcast::Receiver' -S "$ROOT_DIR/crates" >/dev/null 2>&1; then
  rg -n 'tokio::sync::broadcast::Receiver' -S "$ROOT_DIR/crates"
  warn "Found broadcast::Receiver uses. Ensure one receiver per task and no lock is held across await."
fi

if rg -n 'Arc<\s*(Mutex|RwLock)\s*>' -S "$ROOT_DIR/crates" >/dev/null 2>&1; then
  rg -n 'Arc<\s*(Mutex|RwLock)\s*>' -S "$ROOT_DIR/crates"
  warn "Arc<Mutex|RwLock> present. Confirm locked regions are short and never .await while held."
fi

if rg -n 'tokio::io::split' -S "$ROOT_DIR/crates" >/dev/null 2>&1; then
  rg -n 'tokio::io::split' -S "$ROOT_DIR/crates"
  warn "tokio::io::split seen; verify no aliasing/lifetime issues (prefer owned halves)."
fi

if rg -n 'register_(counter|histogram|gauge).*unwrap|register_(counter|histogram|gauge).*expect' -S "$ROOT_DIR/crates" >/dev/null 2>&1; then
  rg -n 'register_(counter|histogram|gauge).*unwrap|register_(counter|histogram|gauge).*expect' -S "$ROOT_DIR/crates"
  warn "Prometheus registration followed by unwrap/expect. Prefer fallible init with Option handles."
fi

echo "[CI-INVARIANTS] OK"
