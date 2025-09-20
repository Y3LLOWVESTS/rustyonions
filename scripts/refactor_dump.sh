#!/usr/bin/env bash
# Refactor data dump (portable for macOS bash 3.2)
# Requires: cargo, jq, rg (ripgrep), git, sort, awk, sed, wc, date
# Optional (heavy): rustup nightly, cargo-deny, /usr/bin/time (mac) or env time (linux)
# For deep analysis run: RUN_API_SCAN=1 RUN_BUILD_TIME=1 BUILD_TIME_TOP=10 RUN_SANITIZERS=1 RUN_CARGO_DENY=1 scripts/refactor_dump.sh out=refactor_full.md since="120 days ago"



set -euo pipefail

# ---------- Config (safe, fast defaults) ----------
OUT_DIR="refactor_dump"
SINCE="${SINCE:-90 days ago}"
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
OUTPUT_MD="${OUTPUT_MD:-}"              # pass as: out=<file>
MAX_FEATURE_ROWS="${MAX_FEATURE_ROWS:-25}"
EXTRA_SMELLS="${EXTRA_SMELLS:-1}"        # 0 to skip quick regex smells
FORBIDDEN_EDGE_HINT="${FORBIDDEN_EDGE_HINT:-kernel::internal}"

# Heavy steps (skip by default; enable as needed)
RUN_API_SCAN="${RUN_API_SCAN:-0}"        # 1 to run nightly rustdoc JSON public API count
RUN_BUILD_TIME="${RUN_BUILD_TIME:-0}"    # 1 to measure per-crate cargo check timings
BUILD_TIME_TOP="${BUILD_TIME_TOP:-5}"    # if measuring, only top-N crates by rdeps/churn
RUN_SANITIZERS="${RUN_SANITIZERS:-0}"    # 1 to run ASan/TSan (nightly)
RUN_CARGO_DENY="${RUN_CARGO_DENY:-0}"    # 1 to run cargo-deny

# Parse simple key=val args
for arg in "$@"; do
  case "$arg" in
    out=*) OUTPUT_MD="${arg#out=}" ;;
    since=*) SINCE="${arg#since=}" ;;
  esac
done

timestamp() { date +"%Y-%m-%d %H:%M:%S %Z"; }
now="$(timestamp)"
mkdir -p "$OUT_DIR"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing tool: $1" >&2; exit 1; }; }
need cargo
need jq
need rg
need git

# ---------------- Core data ----------------
echo "[dump] workspace cargo metadata"
meta_json="$OUT_DIR/metadata.json"
cargo metadata --no-deps --format-version=1 > "$meta_json"

# Workspace crates list (TSV): name<TAB>manifest<TAB>rootdir
crates_tsv="$OUT_DIR/crates.tsv"
jq -r '
  .packages[]
  | select(.source == null)
  | "\(.name)\t\(.manifest_path)\t\(.manifest_path | sub("/Cargo.toml$"; ""))"
' "$meta_json" > "$crates_tsv"

# Dependency edges (workspace, non-dev) (CSV): src,dst
edges_csv="$OUT_DIR/edges.csv"
jq -r '
  .packages[]
  | select(.source == null)
  | {name, dependencies}
  | (.dependencies[]?
      | select((.kind? // "normal") != "dev")
      | select(.source == null)
      | .name) as $dep
  | "\(.name),\($dep)"
' "$meta_json" > "$edges_csv"

# Reverse dependents (Ca): count by dst
rdep_csv="$OUT_DIR/rdep.csv"  # name,count
awk -F, '{c[$2]++} END{for (k in c) print k","c[k]}' "$edges_csv" | sort > "$rdep_csv" || true

# Forward deps (Ce): count by src
fdep_csv="$OUT_DIR/fdep.csv"  # name,count
awk -F, '{c[$1]++} END{for (k in c) print k","c[k]}' "$edges_csv" | sort > "$fdep_csv" || true

# Duplicate versions snapshot
echo "[dump] duplicate dependency versions"
cargo tree -d > "$OUT_DIR/duplicates.txt" || true

# Overview CSV header
overview_csv="$OUT_DIR/crates_overview.csv"
echo "crate,rdep_count,forward_deps,churn_90d,loc_rs,instability" > "$overview_csv"

# Feature hotspots CSV header
feature_hotspots_csv="$OUT_DIR/feature_hotspots.csv"
echo "crate,feature,count" > "$feature_hotspots_csv"

# Join helper: get counts or 0 if missing
get_count() {
  local name="$1" file="$2"
  local val
  val="$(grep -E "^${name}," "$file" | head -n1 | awk -F, '{print $2}' || true)"
  if [ -z "${val:-}" ]; then echo "0"; else echo "$val"; fi
}

# Iterate crates (portable loop; no mapfile/arrays)
# shellcheck disable=SC2162
while IFS=$'\t' read name manifest rootdir; do
  [ -z "${name:-}" ] && continue

  ca="$(get_count "$name" "$rdep_csv")"   # reverse dependents
  ce="$(get_count "$name" "$fdep_csv")"   # forward deps

  # churn: files touched since window
  churn="$(git log --since="$SINCE" --pretty=format: --name-only -- "$rootdir" | rg -N '.*' | wc -l | tr -d ' ')"

  # loc: lines of Rust
  loc_rs="$(rg --trim -n --glob '**/*.rs' '.' "$rootdir" | wc -l | tr -d ' ')"

  # Instability I = Ce / (Ca + Ce)
  denom=$(( ca + ce ))
  if [ "$denom" -eq 0 ]; then I="0.00"; else I=$(awk -v a="$ce" -v b="$denom" 'BEGIN{printf("%.2f", a/b)}'); fi

  echo "$name,$ca,$ce,$churn,$loc_rs,$I" >> "$overview_csv"

  # Reverse tree w/ features → feature hotspots
  tree_rev="$OUT_DIR/${name}.reverse_tree.txt"
  cargo tree -i "$name" -e features --no-dev-deps > "$tree_rev" 2>/dev/null || true
  if rg -q '\([^)]+\)$' "$tree_rev"; then
    rg -e ' \([^)]+\)$' -o "$tree_rev" \
      | sed -E 's/[() ]//g' \
      | tr ',' '\n' \
      | sort | uniq -c | sort -nr | head -n "$MAX_FEATURE_ROWS" \
      | awk -v c="$name" '{print c "," $2 "," $1}' >> "$feature_hotspots_csv"
  fi

  # Forward tree snapshot
  cargo tree -p "$name" -e features > "$OUT_DIR/${name}.forward_tree.txt" 2>/dev/null || true
done < "$crates_tsv"

# Ranked CSV (rdeps desc, then churn desc)
ranked_csv="$OUT_DIR/crates_ranked.csv"
( head -n1 "$overview_csv"; tail -n +2 "$overview_csv" | sort -t, -k2,2nr -k4,4nr ) > "$ranked_csv"

# ---------------- Extras & Heavy (toggled) ----------------

# 1) Cycle & forbidden-edge guesses (cheap heuristics)
echo "[dump] cycle & forbidden-edge guesses"
graph_dot="$OUT_DIR/graph.dot"
cargo tree --workspace -e normal --graph > "$graph_dot" 2>/dev/null || true
# FIX: use -e before pattern so rg doesn't read '->' as a flag
rg -n -e '-> .* -> .* -> .* ->' "$graph_dot" > "$OUT_DIR/cycles_guess.txt" || true
# Forbidden-edge hint: grep for kernel internals imports
rg -n --glob 'crates/**/src/**/*.rs' -e "$FORBIDDEN_EDGE_HINT" > "$OUT_DIR/forbidden_edges_guess.txt" || true

# 2) Public API surface size (nightly rustdoc JSON, optional)
api_count_txt="$OUT_DIR/public_api_count.txt"
api_note_txt="$OUT_DIR/public_api_note.txt"
echo "0" > "$api_count_txt"; echo "skipped public API scan" > "$api_note_txt"
if [ "$RUN_API_SCAN" = "1" ]; then
  if rustup toolchain list | rg -q nightly; then
    echo "[dump] public API size (nightly rustdoc JSON)"
    set +e
    cargo +nightly rustdoc --workspace -- -Z unstable-options --output-format json >/dev/null 2>&1
    status="$?"
    set -e
    if [ "$status" -eq 0 ]; then
      total_pub="$(rg -n --glob 'target/doc/**/*.json' '"visibility":"public"' | wc -l | tr -d ' ')"
      echo "$total_pub" > "$api_count_txt"; echo "OK" > "$api_note_txt"
    else
      echo "0" > "$api_count_txt"; echo "nightly rustdoc JSON failed" > "$api_note_txt"
    fi
  else
    echo "0" > "$api_count_txt"; echo "nightly toolchain not found" > "$api_note_txt"
  fi
fi

# 3) Per-crate build timing (optional, top-N crates)
build_time_csv="$OUT_DIR/build_time.csv"
echo "crate,elapsed_seconds,max_rss_kb" > "$build_time_csv"
if [ "$RUN_BUILD_TIME" = "1" ]; then
  echo "[dump] per-crate build timing (cargo check, top $BUILD_TIME_TOP crates)"
  bt_dir="$OUT_DIR/build_time"; mkdir -p "$bt_dir"

  time_cmd=""; rss_parser=""
  if command -v /usr/bin/time >/dev/null 2>&1 && /usr/bin/time -l true >/dev/null 2>&1; then
    time_cmd="/usr/bin/time -l"           # mac: prints 'maximum resident set size'
    rss_parser='rg -n -e "^\s*([0-9]+)\s+maximum resident set size$" "$log" -or "$1" || echo 0'
  elif command -v /usr/bin/env >/dev/null 2>&1 && /usr/bin/env time -p true >/dev/null 2>&1; then
    time_cmd="/usr/bin/env time -p"       # POSIX: prints 'real X.Y'
    rss_parser='echo 0'
  fi

  # Pick top-N crates by ranked_csv
  top_list="$(tail -n +2 "$ranked_csv" | head -n "$BUILD_TIME_TOP" | awk -F, '{print $1}')"
  echo "$top_list" | while read -r name; do
    [ -z "$name" ] && continue
    log="$bt_dir/${name}.txt"
    if [ -n "$time_cmd" ]; then
      cargo clean -p "$name" >/dev/null 2>&1 || true
      set +e
      eval $time_cmd cargo check -p "$name" > /dev/null 2> "$log"
      rc=$?
      set -e
      if [[ "$time_cmd" == "/usr/bin/time -l" ]]; then
        # mac: estimate elapsed = user+sys if 'real' missing
        user="$(rg -n -e '^\s*([0-9.]+)\s+user$' "$log" -or '$1' || echo 0)"
        sys="$(rg -n -e '^\s*([0-9.]+)\s+sys$' "$log" -or '$1' || echo 0)"
        elapsed="$(awk -v u="$user" -v s="$sys" 'BEGIN{printf("%.2f", u+s)}')"
      else
        elapsed="$(rg -n -e '^real\s+([0-9.]+)$' "$log" -or '$1' || echo 0)"
      fi
      # shellcheck disable=SC2001,SC2016
      rss="$(eval "$rss_parser")"
      echo "$name,${elapsed:-0},${rss:-0}" >> "$build_time_csv"
    else
      echo "$name,0,0" >> "$build_time_csv"
    fi
  done
fi

# 4) Serde wire-contract scan (tag + rename_all)
echo "[dump] serde policy scan (tag/rename_all)"
serde_scan_txt="$OUT_DIR/serde_scan.txt"
rg -n --glob 'crates/**/src/**/*.rs' -e '#\[serde\(' > "$serde_scan_txt" || true
serde_missing_txt="$OUT_DIR/serde_missing_guess.txt"
# Heuristic: public enums without any serde attribute nearby
rg -n --glob 'crates/**/src/**/*.rs' -e 'pub\s+enum\s+\w+' \
  | rg -v -e '#\[serde' > "$serde_missing_txt" || true

# 5) Sanitizer & cargo-deny summaries (optional)
deny_txt="$OUT_DIR/cargo_deny_summary.txt"
echo "skipped cargo-deny" > "$deny_txt"
if [ "$RUN_CARGO_DENY" = "1" ] && command -v cargo-deny >/dev/null 2>&1; then
  echo "[dump] cargo-deny summary"
  set +e; cargo deny check > "$deny_txt" 2>&1; set -e
fi

asan_txt="$OUT_DIR/asan_summary.txt"
tsan_txt="$OUT_DIR/tsan_summary.txt"
echo "skipped ASan/TSan" > "$asan_txt"; echo "skipped ASan/TSan" > "$tsan_txt"
if [ "$RUN_SANITIZERS" = "1" ] && rustup toolchain list | rg -q nightly; then
  echo "[dump] ASan/TSan summaries (nightly)"
  set +e
  RUSTFLAGS="-Zsanitizer=address" RUSTDOCFLAGS="-Zsanitizer=address" ASAN_OPTIONS="detect_leaks=1" \
    cargo +nightly test --workspace --all-targets -Zbuild-std --target "$TARGET" > "$asan_txt" 2>&1
  RUSTFLAGS="-Zsanitizer=thread" RUSTDOCFLAGS="-Zsanitizer=thread" \
    cargo +nightly test --workspace --all-targets -Zbuild-std --target "$TARGET" > "$tsan_txt" 2>&1
  set -e
fi

# ---------------- Quick smells (regex heuristics) ----------------
smells_txt="$OUT_DIR/smells.txt"
if [ "${EXTRA_SMELLS}" = "1" ]; then
  {
    echo "=== PUBLIC STRUCT FIELDS (should be private + ctor) ==="
    rg -n --glob 'crates/**/src/**/*.rs' -e 'pub\s+struct\s+\w+\s*\{\s*[^}]*\n\s*pub\s+\w+\s*:' || true

    echo
    echo "=== STRINGLY FIELDS (status/state/kind/type/role/phase/mode/level) ==="
    rg -n --glob 'crates/**/src/**/*.rs' -e '(status|state|kind|type|role|phase|mode|level)\s*:\s*&?String' || true

    echo
    echo "=== FREE tokio::spawn OUTSIDE SUPERVISOR (grep) ==="
    rg -n --glob 'crates/**/src/**/*.rs' -e 'tokio::spawn' | rg -v -e 'supervisor|kernel' || true

    echo
    echo "=== UNSAFE BLOCKS OUTSIDE ffi/ or hardening/ (grep) ==="
    rg -n --glob 'crates/**/src/**/*.rs' -e 'unsafe\s*\{' | rg -v -e '/ffi/|/hardening/' || true

    echo
    echo "=== FORBIDDEN EDGE HINT (imports of kernel internals) ==="
    rg -n --glob 'crates/**/src/**/*.rs' -e "$FORBIDDEN_EDGE_HINT" || true
  } > "$smells_txt"
fi

# ---------------- Report ----------------
report_md="$OUT_DIR/REPORT.md"
{
  echo "# Refactor Report (Pro, fast defaults)"
  echo
  echo "- Generated: $now"
  echo "- Window for churn: $SINCE"
  echo "- Target: $TARGET"
  echo
  echo "## 1) Summary Tables"
  echo
  echo "### Ranked Crates (by reverse dependents, then churn)"
  echo
  echo '| crate | rdep_count | forward_deps | churn_90d | loc_rs | instability |'
  echo '|------:|-----------:|-------------:|----------:|-------:|------------:|'
  tail -n +2 "$ranked_csv" | head -n 50 | awk -F, '{printf("| %s | %s | %s | %s | %s | %s |\n",$1,$2,$3,$4,$5,$6)}'
  echo
  echo "_Instability = Ce / (Ca + Ce). Lower is more stable (core/kernel); higher is leaf/adapter._"
  echo
  echo "### Feature Hotspots (top $MAX_FEATURE_ROWS per crate)"
  echo
  echo '| crate | feature | count |'
  echo '|------:|:--------|------:|'
  tail -n +2 "$feature_hotspots_csv" | awk -F, '{printf("| %s | %s | %s |\n",$1,$2,$3)}'
  echo
  echo "### Duplicate Dependencies"
  echo
  echo '```text'
  sed -n '1,300p' "$OUT_DIR/duplicates.txt"
  [ "$(wc -l < "$OUT_DIR/duplicates.txt")" -gt 300 ] && echo "... (truncated)"
  echo '```'
  echo
  echo "## 2) Cycles & Forbidden Edges (Heuristics)"
  echo
  echo "**Cycle guesses (from cargo tree --graph):**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/cycles_guess.txt" || true
  echo '```'
  echo
  echo "**Forbidden edge guesses (importing '$FORBIDDEN_EDGE_HINT')**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/forbidden_edges_guess.txt" || true
  echo '```'
  echo
  echo "## 3) Public API Surface (optional)"
  echo
  echo "- Note: $(cat "$api_note_txt")"
  echo "- Estimated public items: **$(cat "$api_count_txt")**"
  echo
  echo "## 4) Build Timing (optional)"
  echo
  echo '| crate | elapsed_seconds | max_rss_kb |'
  echo '|------:|----------------:|-----------:|'
  tail -n +2 "$build_time_csv" | awk -F, '{printf("| %s | %s | %s |\n",$1,$2,$3)}'
  echo
  echo "## 5) Serde Wire-Contract Scan"
  echo
  echo "**Found serde attributes:**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/serde_scan_txt" 2>/dev/null || true
  sed -n '1,200p' "$OUT_DIR/serde_scan.txt" 2>/dev/null || true
  echo '```'
  echo
  echo "**Public enums missing serde tag/rename_all (heuristic):**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/serde_missing_txt" 2>/dev/null || true
  sed -n '1,200p' "$OUT_DIR/serde_missing_guess.txt" 2>/dev/null || true
  echo '```'
  echo
  echo "## 6) Sanitizer & Supply-Chain Summaries (optional)"
  echo
  echo "**cargo-deny summary:**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/cargo_deny_summary.txt" || true
  echo '```'
  echo
  echo "**ASan run summary:**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/asan_summary.txt" || true
  echo '```'
  echo
  echo "**TSan run summary:**"
  echo
  echo '```text'
  sed -n '1,200p' "$OUT_DIR/tsan_summary.txt" || true
  echo '```'
  echo
  if [ -f "$smells_txt" ]; then
    echo "## 7) Quick Smells (regex heuristics)"
    echo
    echo '```text'
    sed -n '1,400p' "$smells_txt"
    [ "$(wc -l < "$smells_txt")" -gt 400 ] && echo "... (truncated)"
    echo '```'
    echo
  fi
  echo "## 8) Per-Crate Trees (reverse + forward)"
  echo
  # shellcheck disable=SC2162
  while IFS=$'\t' read name _ _; do
    [ -z "${name:-}" ] && continue
    echo "### $name"
    echo
    echo "<details><summary>Reverse tree (-i $name -e features)</summary>"
    echo
    echo '```text'
    sed -n '1,300p' "$OUT_DIR/${name}.reverse_tree.txt" 2>/dev/null || true
    [ -f "$OUT_DIR/${name}.reverse_tree.txt" ] && [ "$(wc -l < "$OUT_DIR/${name}.reverse_tree.txt")" -gt 300 ] && echo "... (truncated)"
    echo '```'
    echo
    echo "</details>"
    echo
    echo "<details><summary>Forward tree (-p $name -e features)</summary>"
    echo
    echo '```text'
    sed -n '1,300p' "$OUT_DIR/${name}.forward_tree.txt" 2>/dev/null || true
    [ -f "$OUT_DIR/${name}.forward_tree.txt" ] && [ "$(wc -l < "$OUT_DIR/${name}.forward_tree.txt")" -gt 300 ] && echo "... (truncated)"
    echo '```'
    echo
    echo "</details>"
    echo
  done < "$crates_tsv"
  echo
  echo "## 9) Raw Data Files"
  echo
  echo "- \`$overview_csv\` — per-crate metrics (rdeps, fdeps, churn, loc, instability)"
  echo "- \`$ranked_csv\` — ranked crates by rdeps → churn"
  echo "- \`$feature_hotspots_csv\` — top features per crate by reverse graph"
  echo "- \`$OUT_DIR/duplicates.txt\` — duplicate dependency versions"
  echo "- \`$meta_json\` — cargo metadata snapshot"
  echo "- \`$OUT_DIR/cycles_guess.txt\` — crude cycle hints"
  echo "- \`$OUT_DIR/forbidden_edges_guess.txt\` — kernel/internal import hints"
  echo "- \`$OUT_DIR/public_api_count.txt\` — public API count (if RUN_API_SCAN=1)"
  echo "- \`$OUT_DIR/build_time.csv\` — per-crate build time/RSS (if RUN_BUILD_TIME=1)"
  echo "- \`$OUT_DIR/serde_scan.txt\`, \`$OUT_DIR/serde_missing_guess.txt\` — serde attributes"
  echo "- \`$OUT_DIR/cargo_deny_summary.txt\`, \`$OUT_DIR/asan_summary.txt\`, \`$OUT_DIR/tsan_summary.txt\` (if enabled)"
  [ -f "$smells_txt" ] && echo "- \`$smells_txt\` — quick smells (regex-only)"
} > "$report_md"

# Optional single-file output
if [ -n "${OUTPUT_MD:-}" ]; then
  cp "$report_md" "$OUTPUT_MD"
  echo "[dump] wrote $OUTPUT_MD"
fi

echo "[done] artifacts in $OUT_DIR/"
echo "[done] main report: $report_md"
