#!/usr/bin/env bash
# Scaffolds crates/micronode with the agreed file tree (empty files).
# Usage:
#   chmod +x scripts/scaffold_micronode.sh
#   ./scripts/scaffold_micronode.sh [repo_root]
# If repo_root is omitted, current directory is used.

set -euo pipefail

ROOT="${1:-.}"
CRATE_DIR="$ROOT/crates/micronode"

# --- Directory list ---
DIRS=$(cat <<'EOF'
configs
docs
docs/api-history
docs/diagrams
specs
scripts
src
src/cli
src/config
src/observability
src/security
src/adapters
src/facets
src/concurrency
src/http
tests
tests_property
tests_chaos
tests_loom
fuzz
benches
examples
EOF
)

# --- File list ---
FILES=$(cat <<'EOF'
Cargo.toml
README.md
LICENSE-APACHE
LICENSE-MIT
.gitignore
.clippy.toml
.rustfmt.toml
deny.toml

configs/micronode.toml
configs/micronode.dev.toml
configs/micronode.amnesia.off.toml
configs/micronode.pq.toml
configs/micronode.pq.required.toml

docs/ALL_DOCS.md
docs/API.md
docs/CONFIG.md
docs/CONCURRENCY.md
docs/GOVERNANCE.md
docs/INTEROP.md
docs/PERFORMANCE.md
docs/QUANTUM.md
docs/RUNBOOK.md
docs/SECURITY.md
docs/IDB.md
docs/api-history/micronode.http.json
docs/api-history/metrics.scrape.txt
docs/api-history/pq-matrix.json
docs/diagrams/micronode_arch.mmd
docs/diagrams/micronode_arch.svg

specs/oap-1.md
specs/oap_vectors.json
specs/pq_handshake_cases.json

scripts/run_dev.sh
scripts/fs_spy_amnesia.sh
scripts/gen_diagrams.sh
scripts/smoke_oap_limits.sh
scripts/chaos_degrade_shed.sh
scripts/pq_matrix_ci.sh

src/main.rs
src/lib.rs
src/app.rs
src/state.rs
src/limits.rs
src/errors.rs
src/types.rs

src/cli/mod.rs
src/cli/args.rs
src/cli/run.rs

src/config/mod.rs
src/config/schema.rs
src/config/load.rs
src/config/env_overlay.rs
src/config/cli_overlay.rs
src/config/validate.rs
src/config/hot_reload.rs

src/observability/mod.rs
src/observability/metrics.rs
src/observability/health.rs
src/observability/ready.rs
src/observability/version.rs
src/observability/logging.rs

src/security/mod.rs
src/security/auth_macaroon.rs
src/security/tls_rustls.rs
src/security/amnesia.rs
src/security/pq_toggle.rs
src/security/pq_config.rs
src/security/pq_observe.rs

src/adapters/mod.rs
src/adapters/index_client.rs
src/adapters/mailbox_client.rs
src/adapters/storage_client.rs
src/adapters/overlay_client.rs
src/adapters/policy_client.rs

src/facets/mod.rs
src/facets/graph.rs
src/facets/search.rs
src/facets/feed.rs
src/facets/media.rs

src/concurrency/mod.rs
src/concurrency/backpressure.rs
src/concurrency/shutdown.rs
src/concurrency/registry.rs

src/http/mod.rs
src/http/admin.rs
src/http/routes.rs

tests/admin_parity.rs
tests/amnesia_proof.rs
tests/oap_limits.rs
tests/backpressure.rs
tests/facets_proxy.rs
tests/pq_modes.rs
tests/pq_fallback.rs

tests_property/oap_fuzz.rs
tests_property/pq_handshake_props.rs

tests_chaos/degrade_shed.rs

fuzz/config_from_env_fuzz.rs
fuzz/pq_kex_fuzz.rs

tests_loom/shutdown_interleavings.rs

benches/oap_frame_perf.rs
benches/readiness_walk.rs
benches/pq_overhead.rs

examples/quickstart.rs
EOF
)

# --- helpers ---
mkd() { mkdir -p "$CRATE_DIR/$1"; }
mkf() {
  local p="$CRATE_DIR/$1"
  mkdir -p "$(dirname "$p")"
  [[ -e "$p" ]] || : > "$p"
}

# create dirs
created_dirs=0
while IFS= read -r d; do
  [[ -z "$d" ]] && continue
  mkd "$d"
  created_dirs=$((created_dirs+1))
done <<<"$DIRS"

# create files
created_files=0
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  mkf "$f"
  created_files=$((created_files+1))
done <<<"$FILES"

# seed top-level placeholders if empty
seed_if_empty() {
  local path="$1" text="$2"
  if [[ ! -s "$path" ]]; then
    printf "%s\n" "$text" > "$path"
  fi
}
seed_if_empty "$CRATE_DIR/Cargo.toml" '# Placeholder: to be filled later'
seed_if_empty "$CRATE_DIR/README.md"  '# micronode'
seed_if_empty "$CRATE_DIR/LICENSE-APACHE" 'Apache-2.0 (add full text later)'
seed_if_empty "$CRATE_DIR/LICENSE-MIT"    'MIT (add full text later)'
seed_if_empty "$CRATE_DIR/.gitignore" $'target/\n**/*.svg\n**/*.mermaid-cache\n'

# make scripts executable (even though empty)
for s in run_dev.sh fs_spy_amnesia.sh gen_diagrams.sh smoke_oap_limits.sh chaos_degrade_shed.sh pq_matrix_ci.sh; do
  if [[ -f "$CRATE_DIR/scripts/$s" ]]; then
    chmod +x "$CRATE_DIR/scripts/$s" || true
  fi
done

# report + tree
echo "Scaffolded at: $(cd "$CRATE_DIR" && pwd)"
echo "Created $created_dirs directories and $created_files files (idempotent)."

if command -v tree >/dev/null 2>&1; then
  tree "$CRATE_DIR"
else
  (cd "$CRATE_DIR" && find . -print | sed 's,^\.,crates/micronode,')
fi
