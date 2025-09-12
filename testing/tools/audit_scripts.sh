#!/usr/bin/env bash
# Audits test/smoke scripts for C&A compliance and common breakage points.
# macOS/Bash 3.2 compatible (no globstar/readarray); avoids GNU-only flags.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

fail=0

section() { printf "\n== %s ==\n" "$*"; }
note()    { printf " - %s\n" "$*"; }
warn()    { printf " ! %s\n" "$*"; fail=1; }

# Build the script list (Bash 3.2-friendly)
scripts=()
while IFS= read -r -d '' f; do
  scripts+=("$f")
done < <(find testing -type f -name "*.sh" -print0)

echo "[*] Auditing scripts under testing/ ..."
[ "${#scripts[@]}" -ne 0 ] || { echo "No shell scripts found under testing/ — nothing to audit."; exit 0; }

# ripgrep optional
have_rg=1; command -v rg >/dev/null 2>&1 || have_rg=0

is_helper() {
  case "$1" in
    testing/lib/*|testing/tools/*) return 0 ;; # helper dirs
    *) return 1 ;;
  esac
}

# Return first non-comment line number containing a pattern
first_nc_line() {
  # $1=file, $2=regex
  awk -v pat="$2" '
    /^[[:space:]]*#/ {next}
    $0 ~ pat { print NR; exit 0 }
  ' "$1"
}

# True if file has a non-comment match anywhere
has_nc() {
  # $1=file, $2=regex
  awk -v pat="$2" '
    /^[[:space:]]*#/ {next}
    $0 ~ pat { exit 0 }
    END { exit 1 }
  ' "$1"
}

# -----------------------------------------------------------------------------
section "Shell guardrails"
for f in "${scripts[@]}"; do
  # Look in the first 20 lines, ignoring pure comment/blank lines
  if ! awk 'NR<=20 && $0 !~ /^[[:space:]]*#/ && $0 ~ /set -euo pipefail/ {found=1} END{exit(found?0:1)}' "$f"; then
    warn "$f: missing 'set -euo pipefail' (add near top)"
  fi
done

# -----------------------------------------------------------------------------
section "Missing readiness helpers"
for f in "${scripts[@]}"; do
  # Helpers themselves don't need to source ready.sh
  is_helper "$f" && continue
  # Only nag if the script actually does net/service work
  if has_nc "$f" '(svc-index|svc-overlay|svc-storage|gateway|gwsmoke|curl|nc)'; then
    if ! grep -Eq '(^|[[:space:]])(source|\.)[[:space:]]+testing/lib/ready\.sh' "$f"; then
      warn "$f: not sourcing testing/lib/ready.sh (use wait_http_ok/wait_tcp/wait_file)"
    fi
  fi
done

# -----------------------------------------------------------------------------
section "Magic sleeps (≥0.5s, non-comment code lines only)"
if [ $have_rg -eq 1 ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    file="${line%%:*}"
    is_helper "$file" && continue
    case "$line" in *allow-sleep*) continue ;; esac
    warn "sleep found: $line  -> replace with wait_* in testing/lib/ready.sh"
  done < <(rg -n --hidden --pcre2 '^\s*(?!#).*?\bsleep\s+((?:[1-9]\d*)|(?:0\.(?:5|[6-9])\d*))' testing/ || true)
else
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    file="${line%%:*}"
    is_helper "$file" && continue
    content="${line#*:*:}"
    # Trim leading whitespace and skip if starts with '#'
    case "${content#"${content%%[![:space:]]*}"}" in \#*) continue ;; esac
    case "$content" in *allow-sleep*) continue ;; esac
    warn "sleep found: $line  -> replace with wait_* in testing/lib/ready.sh"
  done < <(grep -RInE 'sleep[[:space:]]+([1-9][0-9]*|0\.(5|[6-9])[0-9]*)' testing/ 2>/dev/null || true)
fi

# -----------------------------------------------------------------------------
section "read -d '' -a (arg parsing bug)"
if [ $have_rg -eq 1 ]; then
  rg -n --fixed-strings "read -d '' -a" testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "arg parsing: $line  -> use: readarray -d '' -t VAR (Bash ≥4) or a Bash-3.2-safe while-read loop"
  done
else
  grep -RIn "read -d '' -a" testing/ 2>/dev/null || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "arg parsing: $line  -> replace with Bash-3.2-safe loop using read -r -d ''"
  done
fi

# -----------------------------------------------------------------------------
section "Case-insensitive header greps (should use -i)"
# Heuristic: any grep for common HTTP headers missing -i (non-comment lines)
if [ $have_rg -eq 1 ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "header grep is case-sensitive: $line  -> add -i"
  done < <(rg -n --pcre2 '^\s*(?!#).*grep\s+-E(?![^"\047]*\s-i)[^"\047]*["\047]?(Content-(Encoding|Type)|ETag|Cache-Control|Vary)["\047]?' testing/ || true)
else
  grep -RInE '^[[:space:]]*[^#].*grep[[:space:]]+-E[[:space:]]+.*(Content-(Encoding|Type)|ETag|Cache-Control|Vary)' testing/ 2>/dev/null \
    | grep -vE ' -i( |$)' || true | while IFS= read -r line; do
      [ -z "$line" ] && continue
      warn "header grep is case-sensitive: $line  -> add -i"
    done
fi

# -----------------------------------------------------------------------------
section "Pack-before-services (sled lock hazard)"
for f in "${scripts[@]}"; do
  pack_line="$(first_nc_line "$f" 'tldctl[[:space:]]+pack')"
  [ -n "${pack_line:-}" ] || continue
  svc_line="$(first_nc_line "$f" '(svc-index|svc-overlay|svc-storage)')"
  if [ -n "${svc_line:-}" ] && [ "$svc_line" -lt "$pack_line" ]; then
    warn "$f: starts service(s) before pack (lines $svc_line < $pack_line) -> pack first to avoid DB lock"
  fi
done

# -----------------------------------------------------------------------------
section "RON_INDEX_DB coherence (shared path)"
for f in "${scripts[@]}"; do
  # Only require this for scripts that start the stack or gateway/services (non-comment)
  if has_nc "$f" '(svc-index|svc-overlay|svc-storage|gateway|gwsmoke)'; then
    if ! grep -Eq 'RON_INDEX_DB=' "$f"; then
      warn "$f: no explicit RON_INDEX_DB; ensure services + gateway + tools share the same path"
    fi
  fi
done

# -----------------------------------------------------------------------------
section "ETag quoting checks (review)"
# Informational: remind to assert ETag format exactly "b3:<hex>"
for f in "${scripts[@]}"; do
  if grep -Eqi 'ETag' "$f"; then
    note "$f: verify ETag assertions expect exact quoted form: \"b3:<hex>\""
  fi
done

echo
if [ $fail -eq 0 ]; then
  echo "[OK] No blocking issues found. Convert any remaining sleeps to readiness and keep the gate strict."
  exit 0
else
  echo "[!] Issues found. Fix the warnings above, then re-run:"
  echo "bash testing/tools/audit_scripts.sh"
  exit 1
fi
