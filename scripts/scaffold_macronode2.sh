#!/usr/bin/env bash
# Scaffolds crates/macronode2 with your specified file tree (empty files).
# Usage:
#   chmod +x scripts/scaffold_macronode2.sh
#   ./scripts/scaffold_macronode2.sh [repo_root]
# If repo_root is omitted, current directory is used.

set -euo pipefail

ROOT="${1:-.}"
CRATE_DIR="$ROOT/crates/macronode2"

# --- Directory list ---
DIRS=$(cat <<'EOF'
src
src/observability
src/config
src/cli
src/supervisor
src/readiness
src/http_admin
src/http_admin/middleware
src/http_admin/handlers
src/services
src/bus
src/facets
src/security
src/pq
src/util
tests
benches
fuzz
fuzz/oap_frame_parser
scripts
docs
docs/api-history
docs/api-history/macronode
docs/openapi
.github
.github/workflows
EOF
)

# --- File list ---
FILES=$(cat <<'EOF'
Cargo.toml
README.md
LICENSE
.gitignore
src/main.rs
src/errors.rs
src/types.rs
src/observability/mod.rs
src/observability/metrics.rs
src/observability/logging.rs
src/config/mod.rs
src/config/schema.rs
src/config/load.rs
src/config/env_overlay.rs
src/config/cli_overlay.rs
src/config/validate.rs
src/config/hot_reload.rs
src/cli/mod.rs
src/cli/args.rs
src/cli/run.rs
src/cli/version.rs
src/cli/check.rs
src/cli/config_print.rs
src/cli/config_validate.rs
src/cli/doctor.rs
src/supervisor/mod.rs
src/supervisor/lifecycle.rs
src/supervisor/backoff.rs
src/supervisor/crash_policy.rs
src/supervisor/shutdown.rs
src/supervisor/health_reporter.rs
src/readiness/mod.rs
src/readiness/deps.rs
src/http_admin/mod.rs
src/http_admin/router.rs
src/http_admin/middleware/mod.rs
src/http_admin/middleware/request_id.rs
src/http_admin/middleware/timeout.rs
src/http_admin/middleware/auth.rs
src/http_admin/middleware/rate_limit.rs
src/http_admin/handlers/version.rs
src/http_admin/handlers/healthz.rs
src/http_admin/handlers/readyz.rs
src/http_admin/handlers/metrics.rs
src/http_admin/handlers/status.rs
src/http_admin/handlers/reload.rs
src/http_admin/handlers/shutdown.rs
src/services/mod.rs
src/services/registry.rs
src/services/spawn.rs
src/services/svc_gateway.rs
src/services/svc_overlay.rs
src/services/svc_index.rs
src/services/svc_storage.rs
src/services/svc_mailbox.rs
src/services/svc_dht.rs
src/bus/mod.rs
src/bus/events.rs
src/facets/mod.rs
src/facets/permits.rs
src/facets/quotas.rs
src/security/mod.rs
src/security/tls.rs
src/security/macaroon.rs
src/security/amnesia.rs
src/pq/mod.rs
src/pq/hybrid.rs
src/util/sizes.rs
src/util/dur.rs
tests/admin_smoke.rs
tests/readiness_drain.rs
tests/metrics_contract.rs
benches/admin_paths_latency.rs
fuzz/oap_frame_parser/.keep
scripts/dump_http_surface.sh
scripts/dump_metrics_names.sh
scripts/render_mermaid.sh
docs/ALL_DOCS.md
docs/API.md
docs/CONFIG.md
docs/CONCURRENCY.md
docs/GOVERNANCE.md
docs/IDB.md
docs/INTEROP.md
docs/OBSERVABILITY.md
docs/PERFORMANCE.md
docs/QUANTUM.md
docs/RUNBOOK.md
docs/SECURITY.md
docs/TESTS.md
docs/api-history/macronode/cli-vX.Y.Z.txt
docs/api-history/macronode/http-vX.Y.Z.json
docs/api-history/macronode/metrics-vX.Y.Z.txt
docs/openapi/macronode.v1.yaml
.github/workflows/ci.yml
.github/workflows/render-mermaid.yml
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
seed_if_empty "$CRATE_DIR/README.md"  '# macronode2'
seed_if_empty "$CRATE_DIR/LICENSE"    'MIT OR Apache-2.0 (add full text later)'
seed_if_empty "$CRATE_DIR/.gitignore" $'target/\n**/*.svg\n**/*.mermaid-cache\n'

# ensure .keep in empty dirs we care about
for d in fuzz/oap_frame_parser docs/api-history docs/api-history/macronode docs/openapi .github/workflows; do
  [[ -d "$CRATE_DIR/$d" ]] || continue
  if [[ -z "$(ls -A "$CRATE_DIR/$d")" ]]; then
    : > "$CRATE_DIR/$d/.keep"
  fi
done

# report + tree
echo "Scaffolded at: $(cd "$CRATE_DIR" && pwd)"
echo "Created $created_dirs directories and $created_files files (idempotent)."

if command -v tree >/dev/null 2>&1; then
  tree "$CRATE_DIR"
else
  (cd "$CRATE_DIR" && find . -print | sed 's,^\.,crates/macronode2,')
fi
