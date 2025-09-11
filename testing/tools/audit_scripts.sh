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
if [ "${#scripts[@]}" -eq 0 ]; then
  echo "No shell scripts found under testing/ — nothing to audit."
  exit 0
fi

# ripgrep optional
have_rg=1
if ! command -v rg >/dev/null 2>&1; then
  have_rg=0
fi

section "Shell guardrails"
for f in "${scripts[@]}"; do
  if ! head -n 5 "$f" | grep -Eq 'set -euo pipefail'; then
    warn "$f: missing 'set -euo pipefail' (add near top)"
  fi
done

section "Missing readiness helpers"
for f in "${scripts[@]}"; do
  if ! grep -Eq '(^|[[:space:]])(source|\.)[[:space:]]+testing/lib/ready\.sh' "$f"; then
    warn "$f: not sourcing testing/lib/ready.sh (use wait_http_ok/wait_tcp/wait_file)"
  fi
done

section "Magic sleeps (≥0.5s)"
if [ $have_rg -eq 1 ]; then
  # sleep 1,2,... OR sleep 0.5/0.6/…
  rg -n --pcre2 'sleep\s+((?:[1-9]\d*)|(?:0\.(?:5|[6-9])\d*))' testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "sleep found: $line  -> replace with wait_* in testing/lib/ready.sh"
  done
else
  # Fallback: crude grep for 'sleep ' + nonzero integer
  grep -RInE 'sleep[[:space:]]+[1-9]' testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "sleep found: $line  -> replace with wait_* in testing/lib/ready.sh"
  done
  # Also flag 'sleep 0.5+' (decimal)
  grep -RInE 'sleep[[:space:]]+0\.(5|[6-9])[0-9]*' testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "sleep found: $line  -> replace with wait_* in testing/lib/ready.sh"
  done
fi

section "read -d '' -a (arg parsing bug)"
if [ $have_rg -eq 1 ]; then
  rg -n --fixed-strings "read -d '' -a" testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "arg parsing: $line  -> use: readarray -d '' -t VAR (or while-read with -d '' if Bash ≥4); for Bash 3.2 use: while IFS= read -r -d '' x; do ...; done"
  done
else
  grep -RIn "read -d '' -a" testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "arg parsing: $line  -> replace with Bash-3.2-safe loop using read -r -d ''"
  done
fi

section "Case-insensitive header greps (should use -i)"
# Look for script lines grepping common headers without -i
if [ $have_rg -eq 1 ]; then
  rg -n --pcre2 'grep\s+-E(?!.*\s-i).*"(Content-(Encoding|Type)|ETag|Cache-Control|Vary)"' testing/ || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "header grep is case-sensitive: $line  -> add -i"
  done
else
  # Simple heuristic: lines with grep -E "Header" but missing -i
  grep -RInE 'grep[[:space:]]+-E[[:space:]]+".*(Content-(Encoding|Type)|ETag|Cache-Control|Vary).*"' testing/ | grep -v -- ' -i' || true | while IFS= read -r line; do
    [ -z "$line" ] && continue
    warn "header grep is case-sensitive: $line  -> add -i"
  done
fi

section "Pack-before-services (sled lock hazard)"
for f in "${scripts[@]}"; do
  if grep -nE 'tldctl[[:space:]]+pack' "$f" >/dev/null 2>&1; then
    pack_line="$(grep -nE 'tldctl[[:space:]]+pack' "$f" | head -n1 | cut -d: -f1)"
    svc_line="$(grep -nE '(svc-index|svc-overlay|svc-storage)' "$f" | head -n1 | cut -d: -f1 || echo 0)"
    if [ -n "${svc_line}" ] && [ "${svc_line}" -ne 0 ] && [ "${svc_line}" -lt "${pack_line}" ]; then
      warn "$f: starts service(s) before pack (lines $svc_line < $pack_line) -> pack first to avoid DB lock"
    fi
  fi
done

section "RON_INDEX_DB coherence (shared path)"
for f in "${scripts[@]}"; do
  if ! grep -Eq 'RON_INDEX_DB=' "$f"; then
    warn "$f: no explicit RON_INDEX_DB; ensure services + gateway + tools share the same path"
  fi
done

section "ETag quoting checks (review)"
# Informational: remind to assert ETag format exactly "b3:<hex>"
for f in "${scripts[@]}"; do
  if grep -Eq 'ETag' "$f"; then
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
