#!/usr/bin/env bash
# Remove all Chutney code, artifacts, and references from the repo.
# Handles normal dirs *and* half-broken submodules. macOS/Linux compatible.
# Use --yes to skip the prompt.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AUTO_YES=0

usage() {
  cat <<'USAGE'
usage:
  scripts/remove_chutney.sh [--yes]

options:
  --yes     Do not prompt for confirmation (assume "yes")
USAGE
}

while (( "$#" )); do
  case "${1:-}" in
    --yes) AUTO_YES=1; shift;;
    -h|--help) usage; exit 0;;
    *) echo "unknown option: $1" >&2; usage; exit 2;;
  esac
done

say() { printf "[remove-chutney] %s\n" "$*"; }

confirm() {
  if [ "$AUTO_YES" -eq 1 ]; then return 0; fi
  # Lowercase conversion: macOS bash 3.2 doesn't support ${var,,} reliably; use tr
  printf "[remove-chutney] Proceed with deletion? (yes/no): "
  read -r ans || true
  ans="$(printf "%s" "${ans:-}" | tr '[:upper:]' '[:lower:]')"
  case "$ans" in y|yes) return 0 ;; *) return 1 ;; esac
}

# Return submodule name given a path, if present in .gitmodules
# e.g., path "chutney" -> name "chutney" (but could be something else)
submodule_name_for_path() {
  local path="$1"
  [ -f ".gitmodules" ] || { echo ""; return; }
  # Lines like: submodule.NAME.path PATH
  local line name
  # shellcheck disable=SC2002
  line="$(git config -f .gitmodules --get-regexp 'submodule\..*\.path' 2>/dev/null | awk -v p="$path" '$NF==p')"
  [ -z "$line" ] && { echo ""; return; }
  # Extract NAME from 'submodule.NAME.path'
  name="$(printf "%s" "$line" | awk '{print $1}' | sed -e 's/^submodule\.//' -e 's/\.path$//')"
  echo "$name"
}

# Safely remove a path whether it's a submodule (even half-broken) or plain dir
remove_path() {
  local p="$1"
  [ -e "$p" ] || return 0

  local name
  name="$(submodule_name_for_path "$p" || true)"

  if [ -n "$name" ]; then
    say "detected submodule config for '$p' (name: $name) — cleaning…"
    # Try to deinit; ignore failure if state is broken
    git submodule deinit -f -- "$p" 2>/dev/null || true
    # Remove submodule sections from both .gitmodules and local config
    git config -f .gitmodules --remove-section "submodule.$name" 2>/dev/null || true
    git config --remove-section "submodule.$name" 2>/dev/null || true
    # Remove the cached submodule repo if it exists
    rm -rf ".git/modules/$name" 2>/dev/null || true
    # Now try removing the working tree from the index; if that fails, fall back to rm -rf
    git rm -r -f --cached "$p" 2>/dev/null || true
  fi

  # Whether submodule or not, ensure the directory is gone and untracked
  if git ls-files --error-unmatch "$p" >/dev/null 2>&1; then
    git rm -r -f "$p" || true
  fi
  rm -rf "$p" || true
  say "removed: $p"
}

clean_gitmodules_if_empty() {
  if [ -f ".gitmodules" ] && [ ! -s ".gitmodules" ]; then
    say ".gitmodules is empty — removing"
    git rm -f .gitmodules 2>/dev/null || rm -f .gitmodules || true
  fi
}

list_hits() {
  say "Scanning for references…"
  if command -v rg >/dev/null 2>&1; then
    rg -n "chutney|CHUTNEY" -S || true
  else
    grep -RinE "chutney|CHUTNEY" . || true
  fi
}

main() {
  cd "$ROOT"

  say "Targets to remove:"
  printf "  - chutney/\n"
  printf "  - third_party/chutney/\n"
  printf "  - testing/run_chutney_e2e.sh\n"
  printf "  - any chutney venv/net artifacts\n"
  printf "  - CI lines mentioning chutney (best effort)\n"

  list_hits
  if ! confirm; then say "Aborted."; exit 1; fi

  # 1) Remove the chutney dirs (handles submodule or plain dir)
  remove_path "chutney"
  remove_path "third_party/chutney"

  # 2) Runner script
  if [ -e "testing/run_chutney_e2e.sh" ]; then
    if git ls-files --error-unmatch "testing/run_chutney_e2e.sh" >/dev/null 2>&1; then
      git rm -f testing/run_chutney_e2e.sh || true
    fi
    rm -f testing/run_chutney_e2e.sh || true
    say "removed: testing/run_chutney_e2e.sh"
  fi

  # 3) Artifacts
  rm -rf "chutney/.venv" "third_party/chutney/.venv" \
         "chutney/net"   "third_party/chutney/net" || true

  # 4) CI workflow lines (case-insensitive delete; remove file if empty)
  if [ -d ".github/workflows" ]; then
    # Find files that mention chutney
    files="$(grep -RIl "chutney" .github/workflows || true)"
    if [ -n "${files:-}" ]; then
      while IFS= read -r f; do
        [ -z "$f" ] && continue
        say "editing workflow: $f"
        sed -i.bak '/chutney/Id' "$f" || true
        if [ ! -s "$f" ]; then
          git rm -f "$f" 2>/dev/null || rm -f "$f" || true
        else
          rm -f "$f.bak" || true
          git add "$f" || true
        fi
      done <<< "$files"
    fi
  fi

  # 5) If .gitmodules is empty now, drop it
  clean_gitmodules_if_empty

  # 6) Final scan
  list_hits

  say "If no meaningful hits remain, commit next:"
  say "  git commit -m \"Remove Chutney: clean submodule config and delete trees\""
}

main "$@"
