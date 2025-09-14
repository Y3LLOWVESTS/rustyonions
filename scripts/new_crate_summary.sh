#!/usr/bin/env bash
# new_crate_summary.sh — generate docs/crate-summaries/<crate>.md from a template.
# Features:
# - Cross-platform (macOS/BSD & GNU) — no GNU-only sed.
# - Auto-fills: crate, path, role (heuristic), owner (env/git), maturity (env), last-reviewed (today).
# - Lean mode (--lean) merges sections 14–16 into a single "Improvement Opportunities" block (per Grok’s suggestion).
# - Safe by default: refuses to overwrite unless --force.
# - Helpful usage and exit codes. Comments are OK in scripts. No external deps.

set -euo pipefail

print_usage() {
  cat <<'USAGE'
usage:
  scripts/new_crate_summary.sh [--lean] [--force] [--owner NAME] [--role ROLE] [--maturity STAGE] [--path PATH] [--edit]
                               <crate-name> [<crate-name> ...]

options:
  --lean            Use the condensed template (merges §§14–16 into "Improvement Opportunities").
  --force           Overwrite existing docs/crate-summaries/<crate>.md if it already exists.
  --owner NAME      Owner to stamp into the summary front-matter. Default: $OWNER or git user.name or "unowned".
  --role ROLE       One of: core | service | bin | lib | sdk | test-util. Default: heuristic by crate name.
  --maturity STAGE  One of: draft | alpha | beta | prod. Default: $MATURITY or "draft".
  --path PATH       Path to the crate directory. Default: crates/<crate>.
  --edit            Open created file in $EDITOR if set (ignored if not set).
  -h, --help        Show this help.

environment overrides:
  OWNER, MATURITY, TODAY (YYYY-MM-DD)

examples:
  scripts/new_crate_summary.sh ron-kernel
  scripts/new_crate_summary.sh --lean --owner "Stevan White" svc-overlay svc-storage
  scripts/new_crate_summary.sh --force --role sdk ron-app-sdk
USAGE
}

# ---------- parse flags ----------
LEAN=0
FORCE=0
OWNER="${OWNER:-}"
ROLE_OVERRIDE=""
MATURITY="${MATURITY:-draft}"
PATH_OVERRIDE=""
EDIT=0

ARGS=()
while (( "$#" )); do
  case "${1:-}" in
    --lean) LEAN=1; shift;;
    --force) FORCE=1; shift;;
    --owner) OWNER="${2:-}"; shift 2;;
    --role) ROLE_OVERRIDE="${2:-}"; shift 2;;
    --maturity) MATURITY="${2:-}"; shift 2;;
    --path) PATH_OVERRIDE="${2:-}"; shift 2;;
    --edit) EDIT=1; shift;;
    -h|--help) print_usage; exit 0;;
    --) shift; while (( "$#" )); do ARGS+=("$1"); shift; done; break;;
    -*)
      echo "error: unknown option: $1" >&2
      print_usage
      exit 2
      ;;
    *) ARGS+=("$1"); shift;;
  esac
done

if [ "${#ARGS[@]}" -eq 0 ]; then
  print_usage
  exit 2
fi

# ---------- helpers ----------
trim() { awk '{$1=$1;print}' <<<"${1-}"; }

infer_owner() {
  if [ -n "${OWNER:-}" ]; then
    echo "$(trim "$OWNER")"
    return
  fi
  # try git user.name
  if command -v git >/dev/null 2>&1; then
    local n
    n="$(git config --get user.name || true)"
    if [ -n "$n" ]; then
      echo "$(trim "$n")"
      return
    fi
  fi
  echo "unowned"
}

infer_role() {
  local c="$1"
  if [ -n "$ROLE_OVERRIDE" ]; then
    echo "$ROLE_OVERRIDE"
    return
  fi
  # heuristics
  case "$c" in
    ron-kernel|kernel|core|ron-core) echo "core"; return;;
    svc-*) echo "service"; return;;
    *-sdk|ron-app-sdk|sdk|client|cli) echo "sdk"; return;;
    *-bin|bin-*) echo "bin"; return;;
    test-*|*-test|*-tests|*-fuzz) echo "test-util"; return;;
    *) echo "lib"; return;;
  esac
}

infer_path() {
  local c="$1"
  if [ -n "$PATH_OVERRIDE" ]; then
    echo "$PATH_OVERRIDE"
    return
  fi
  # default workspace layout
  echo "crates/${c}"
}

today_iso() {
  if [ -n "${TODAY:-}" ]; then
    echo "$TODAY"
    return
  fi
  # POSIX date +%F works on BSD/macOS and GNU
  date +%F
}

ensure_dirs() {
  mkdir -p "docs/crate-summaries"
}

write_file() {
  local outfile="$1"
  local payload="$2"
  if [ -f "$outfile" ] && [ "$FORCE" -ne 1 ]; then
    echo "error: $outfile already exists (use --force to overwrite)" >&2
    return 1
  fi
  printf "%s" "$payload" > "$outfile"
  echo "Wrote $outfile"
}

open_in_editor() {
  local f="$1"
  if [ "$EDIT" -eq 1 ] && [ -n "${EDITOR:-}" ]; then
    "$EDITOR" "$f" || true
  fi
}

# ---------- templates ----------
full_template() {
  # args: crate path role owner maturity date
  local CR="$1" P="$2" R="$3" O="$4" M="$5" D="$6"
  cat <<EOF
---
crate: ${CR}
path: ${P}
role: ${R}
owner: ${O}
maturity: ${M}
last-reviewed: ${D}
---

## 1) One-liner
What does this crate do in one sentence?

## 2) Primary Responsibilities (must-have)
- (1–3 bullets, describe the essential responsibilities only)

## 3) Non-Goals (won’t do)
- (Boundaries that prevent scope creep)

## 4) Public API Surface (externally used)
- Re-exports: ...
- Key types: ...
- Key functions/traits/macros: ...
- Events/messages emitted (e.g., \`KernelEvent::...\`): ...
- HTTP endpoints (if any): ...
- CLI subcommands/args (if a bin): ...

## 5) Dependencies & Coupling
### Internal crates used
- crate → why needed (capability), stability (tight/loose), replaceable? [yes/no]

### External crates used (top 5 most important)
- crate (version/pins/features) → why needed, risk (license/maintenance)

### Runtime services touched
- Network: [ports/protocols]
- Storage: [sled / fs paths / env vars]
- OS/Process: [signals, subprocesses, UDS paths]
- Crypto: [blake3/sha2/base64/hex/PQ plans?]

## 6) Config & Feature Flags
- Env vars: NAME (default, units) → effect
- Config structs/fields (versioning?): …
- Cargo features: [list] → behavior changes

## 7) Observability
- Metrics (Prometheus names/labels): …
- Health/readiness signals: …
- Logs (notable spans/targets): …

## 8) Concurrency Model
- Tasks/channels/backpressure; locks/timeouts/retries

## 9) Persistence & Data Model
- DB/schema or key prefixes; artifacts/retention

## 10) Error Taxonomy
- Retryable vs terminal; HTTP/CLI mapping

## 11) Security Posture
- Validation, authn/z, TLS, secrets, PQ-readiness

## 12) Performance Notes
- Hot paths; targets/timeouts

## 13) Tests
- Unit / Integration / E2E / fuzz / loom

## 14) Known Gaps / Tech Debt
- …

## 15) Overlap & Redundancy Signals
- Duplicates with: …
- API overlap: …

## 16) Streamlining Opportunities
- Merge/extract/replace/simplify …

## 17) Change Log (recent)
- YYYY-MM-DD — short note

## 18) Readiness Score (0–5 each)
- API clarity: _
- Test coverage: _
- Observability: _
- Config hygiene: _
- Security posture: _
- Performance confidence: _
- Coupling (lower is better): _
EOF
}

lean_template() {
  # args: crate path role owner maturity date
  local CR="$1" P="$2" R="$3" O="$4" M="$5" D="$6"
  cat <<EOF
---
crate: ${CR}
path: ${P}
role: ${R}
owner: ${O}
maturity: ${M}
last-reviewed: ${D}
---

## 1) One-liner
What does this crate do in one sentence?

## 2) Primary Responsibilities
- (1–3 bullets, essential responsibilities only)

## 3) Non-Goals
- (Boundaries that prevent scope creep)

## 4) Public API Surface
- Re-exports: …
- Key types / functions / traits: …
- Events / HTTP / CLI (if any): …

## 5) Dependencies & Coupling
- Internal crates → why, stability (tight/loose), replaceable? [yes/no]
- External crates (top 5; pins/features) → why, risk (license/maintenance)
- Runtime services: Network / Storage / OS / Crypto

## 6) Config & Feature Flags
- Env vars, config structs, cargo features → effect

## 7) Observability
- Metrics, readiness/health, logs

## 8) Concurrency Model
- Tasks/channels/backpressure; locks/timeouts/retries

## 9) Persistence & Data Model
- DB/schema or key prefixes; artifacts/retention

## 10) Errors & Security
- Error taxonomy (retryable vs terminal); authn/z, TLS, secrets, PQ-readiness

## 11) Performance Notes
- Hot paths; latency/throughput targets

## 12) Tests
- Unit / Integration / E2E / fuzz / loom

## 13) Improvement Opportunities
- Known gaps / tech debt
- Overlap & redundancy signals (duplicates, API overlap)
- Streamlining (merge/extract/replace/simplify)

## 14) Change Log (recent)
- YYYY-MM-DD — short note

## 15) Readiness Score (0–5 each)
- API clarity: _
- Test coverage: _
- Observability: _
- Config hygiene: _
- Security posture: _
- Performance confidence: _
- Coupling (lower is better): _
EOF
}

# ---------- main ----------
ensure_dirs
TODAY="$(today_iso)"
OWNER_VAL="$(infer_owner)"

for CRATE in "${ARGS[@]}"; do
  CRATE_TRIM="$(trim "$CRATE")"
  [ -z "$CRATE_TRIM" ] && { echo "error: empty crate name"; exit 2; }

  ROLE_VAL="$(infer_role "$CRATE_TRIM")"
  PATH_VAL="$(infer_path "$CRATE_TRIM")"

  OUT="docs/crate-summaries/${CRATE_TRIM}.md"

  # Warn if path doesn't exist (non-fatal)
  if [ ! -d "$PATH_VAL" ]; then
    echo "warn: path '$PATH_VAL' does not exist (override with --path PATH)" >&2
  fi

  if [ "$LEAN" -eq 1 ]; then
    CONTENT="$(lean_template "$CRATE_TRIM" "$PATH_VAL" "$ROLE_VAL" "$OWNER_VAL" "$MATURITY" "$TODAY")"
  else
    CONTENT="$(full_template "$CRATE_TRIM" "$PATH_VAL" "$ROLE_VAL" "$OWNER_VAL" "$MATURITY" "$TODAY")"
  fi

  write_file "$OUT" "$CONTENT"
  open_in_editor "$OUT"
done
