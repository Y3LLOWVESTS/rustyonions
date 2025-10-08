#!/usr/bin/env bash
# Scaffolder for crates/omnigate2 — structure only, no Rust logic.
# Busybox/older shells may lack `mapfile`, so this script avoids it.
set -euo pipefail

ROOT="crates/omnigate2"

mkd() { mkdir -p "$1"; }
mkt() { mkdir -p "$(dirname "$1")"; : > "$1"; }                 # touch empty
mkf() { mkdir -p "$(dirname "$1")"; cat > "$1"; }               # write with heredoc/content

# 1) Directories (explicit, no 'mapfile')
mkd "$ROOT/.cargo"
mkd "$ROOT/src/config"
mkd "$ROOT/src/bootstrap"
mkd "$ROOT/src/admission"
mkd "$ROOT/src/auth"
mkd "$ROOT/src/middleware"
mkd "$ROOT/src/routes/v1/facet"
mkd "$ROOT/src/hydration"
mkd "$ROOT/src/downstream"
mkd "$ROOT/src/readiness"
mkd "$ROOT/src/metrics"
mkd "$ROOT/src/errors"
mkd "$ROOT/src/observability"
mkd "$ROOT/src/runtime"
mkd "$ROOT/src/pq"
mkd "$ROOT/src/zk"
mkd "$ROOT/src/types"
mkd "$ROOT/benches"
mkd "$ROOT/tests"
mkd "$ROOT/fuzz/fuzz_targets"
mkd "$ROOT/docs/mermaid"
mkd "$ROOT/docs/api-history/omnigate"
mkd "$ROOT/specs"
mkd "$ROOT/configs"
mkd "$ROOT/testing/performance/baselines"
mkd "$ROOT/testing/chaos"
mkd "$ROOT/testing/vectors/omnigate"
mkd "$ROOT/scripts"
mkd "$ROOT/.github/workflows"

# 2) Root metadata & config files
mkf "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "omnigate2"
version = "0.0.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Scaffold-only service crate (omnigate2). No implementation yet."

[features]
tls = []
pq = []
zk = []           # read-only; no mutate/settle (enforced by tests/zk_read_only.rs)
reload = []
otel = []

[dependencies]
# Intentionally omitted for scaffold purity; add axum/tokio/etc. when implementing.

[dev-dependencies]
# Add test-only deps later (e.g., proptest, loom, reqwest).
EOF

mkf "$ROOT/README.md" <<'EOF'
# omnigate2 (scaffold)
This is the structure-only scaffold for omnigate2. Files are intentionally minimal placeholders to keep modules small and swappable. Fill in per the crate README contract and acceptance gates.
EOF

mkf "$ROOT/CHANGELOG.md" <<'EOF'
# Changelog (scaffold)
- 0.0.0: Initial skeleton. No public API.
EOF

mkf "$ROOT/LICENSE-APACHE" <<'EOF'
Apache License 2.0 (placeholder). Replace with project standard text if needed.
EOF

mkf "$ROOT/LICENSE-MIT" <<'EOF'
MIT License (placeholder). Replace with project standard text if needed.
EOF

mkf "$ROOT/CODEOWNERS" <<'EOF'
* @your-team/owners
EOF

mkf "$ROOT/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
EOF

mkf "$ROOT/deny.toml" <<'EOF'
# cargo-deny placeholder. Populate advisories/license bans as you wire deps.
EOF

mkf "$ROOT/.cargo/config.toml" <<'EOF'
# Workspace lints; keep warnings loud during dev/CI.
[build]
rustflags = []
EOF

mkf "$ROOT/.gitignore" <<'EOF'
/target
**/*.svg
**/*.profraw
**/flamegraph*.svg
.DS_Store
EOF

# 3) src/ entrypoints (no Rust logic, just placeholders)
mkf "$ROOT/src/main.rs" <<'EOF'
// omnigate2 scaffold main — intentionally empty; add bootstrap wiring later.
fn main() {}
EOF

mkf "$ROOT/src/lib.rs" <<'EOF'
// omnigate2 scaffold lib — re-export minimal surface when you add code.
EOF

# 4) src/config/*
for f in mod.rs env.rs file.rs validate.rs reload.rs; do mkt "$ROOT/src/config/$f"; done

# 5) src/bootstrap/*
for f in mod.rs server.rs metrics_server.rs health_probe.rs; do mkt "$ROOT/src/bootstrap/$f"; done

# 6) src/admission/*
for f in mod.rs quotas.rs fair_queue.rs; do mkt "$ROOT/src/admission/$f"; done

# 7) src/auth/*
for f in mod.rs capability.rs passport_client.rs revocation.rs; do mkt "$ROOT/src/auth/$f"; done

# 8) src/middleware/*
for f in mod.rs body_caps.rs decompress_guard.rs corr_id.rs classify.rs slow_loris.rs; do mkt "$ROOT/src/middleware/$f"; done

# 9) src/routes/*
mkt "$ROOT/src/routes/mod.rs"
mkt "$ROOT/src/routes/ops.rs"
mkt "$ROOT/src/routes/v1/mod.rs"
for f in objects.rs index.rs mailbox.rs dht.rs; do mkt "$ROOT/src/routes/v1/$f"; done
for f in mod.rs media.rs graph.rs feed.rs; do mkt "$ROOT/src/routes/v1/facet/$f"; done

# 10) src/hydration/*
for f in mod.rs planner.rs compose.rs; do mkt "$ROOT/src/hydration/$f"; done

# 11) src/downstream/*
for f in mod.rs index_client.rs storage_client.rs mailbox_client.rs dht_client.rs latency.rs hedge.rs; do mkt "$ROOT/src/downstream/$f"; done

# 12) src/readiness/*
for f in mod.rs keys.rs policy.rs; do mkt "$ROOT/src/readiness/$f"; done

# 13) src/metrics/*
for f in mod.rs registry.rs; do mkt "$ROOT/src/metrics/$f"; done

# 14) src/errors/*
for f in mod.rs http_map.rs reasons.rs; do mkt "$ROOT/src/errors/$f"; done

# 15) src/observability/*
for f in mod.rs logging.rs tracing_spans.rs; do mkt "$ROOT/src/observability/$f"; done

# 16) src/runtime/*
for f in mod.rs supervisor.rs worker.rs channels.rs shutdown.rs; do mkt "$ROOT/src/runtime/$f"; done

# 17) src/pq/*
for f in mod.rs negotiate.rs; do mkt "$ROOT/src/pq/$f"; done

# 18) src/zk/*
for f in mod.rs receipts.rs no_mutate.rs; do mkt "$ROOT/src/zk/$f"; done

# 19) src/types/*
for f in mod.rs dto.rs; do mkt "$ROOT/src/types/$f"; done

# 20) benches/*
for f in hydration.rs media_range.rs; do mkt "$ROOT/benches/$f"; done

# 21) tests/*
for f in interop_vectors.rs readyz_overload.rs oap_limits.rs policy_gate.rs metrics_contract.rs loom_fanout.rs hardening.rs zk_read_only.rs; do
  mkt "$ROOT/tests/$f"
done

# 22) fuzz/*
mkf "$ROOT/fuzz/Cargo.toml" <<'EOF'
[package]
name = "omnigate2-fuzz"
version = "0.0.0"
edition = "2021"
publish = false

[package.metadata]
cargo-fuzz = true
EOF
for f in capability.rs decompress_guard.rs headers.rs; do mkt "$ROOT/fuzz/fuzz_targets/$f"; done

# 23) docs/*
for f in API.md CONFIG.md CONCURRENCY.md GOVERNANCE.md IDB.md INTEROP.md OBSERVABILITY.md PERFORMANCE.md QUANTUM.md RUNBOOK.md SECURITY.md TESTS.md; do
  mkf "$ROOT/docs/$f" <<<"# $f (scaffold)
Place canonical content here."
done
for f in arch.mmd sequence.mmd states.mmd; do mkt "$ROOT/docs/mermaid/$f"; done
mkt "$ROOT/docs/api-history/omnigate/<rev>.txt"

# 24) specs/*
mkf "$ROOT/specs/README.md" <<'EOF'
# Running TLC (scaffold)
# Example local run:
# tlc readiness.tla
EOF
for f in readiness.tla admission.tla; do mkt "$ROOT/specs/$f"; done

# 25) configs/*
mkf "$ROOT/configs/omnigate.toml" <<'EOF'
# omnigate2 dev defaults (scaffold)
# OAP frame cap = 1_048_576; stream chunk = 65_536
EOF
mkt "$ROOT/configs/staging.toml"
mkt "$ROOT/configs/amnesia.toml"

# 26) testing/*
mkf "$ROOT/testing/performance/hydrate_mix.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: put your loadgen here and assert p95 targets.
EOF
chmod +x "$ROOT/testing/performance/hydrate_mix.sh"
mkf "$ROOT/testing/performance/baselines/p95_hydration.json" <<'EOF'
{"p95_ms":150}
EOF
mkf "$ROOT/testing/performance/baselines/p95_range.json" <<'EOF'
{"p95_ms":100}
EOF
mkf "$ROOT/testing/chaos/README.md" <<'EOF'
# Chaos/Soak (scaffold)
Use scenario.yml to define latency/error injections for weekly runs.
EOF
mkf "$ROOT/testing/chaos/scenario.yml" <<'EOF'
# Scaffold example for chaos; fill with providers and injections.
injections: []
EOF
for f in range_read.json error_413.json unauth_401.json; do mkt "$ROOT/testing/vectors/omnigate/$f"; done

# 27) scripts/*
mkf "$ROOT/scripts/check_boundary.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: add grep/lints for forbidden deps and patterns.
EOF
mkf "$ROOT/scripts/render_mermaid.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: render docs/mermaid/*.mmd to SVGs.
EOF
mkf "$ROOT/scripts/ci_metrics_guard.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: scrape /metrics and assert label/name contracts.
EOF
mkf "$ROOT/scripts/inject_faults.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: locally simulate latency/errors.
EOF
mkf "$ROOT/scripts/soak.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: 24h soak runner placeholder.
EOF
mkf "$ROOT/scripts/hnDL_sim.sh" <<'EOF'
#!/usr/bin/env bash
# Scaffold: Harvest-now-decrypt-later drill placeholder.
EOF
chmod +x "$ROOT"/scripts/*.sh

# 28) workflows
mkf "$ROOT/.github/workflows/ci.yml" <<'EOF'
name: ci
on: [push, pull_request]
jobs:
  build-test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        arch: [x86_64, aarch64]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: echo "Scaffold CI placeholder."
EOF

mkf "$ROOT/.github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Render mermaid placeholder."
EOF

mkf "$ROOT/.github/workflows/perf-regression.yml" <<'EOF'
name: perf-regression
on:
  workflow_dispatch:
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Perf regression scaffold."
EOF

mkf "$ROOT/.github/workflows/chaos.yml" <<'EOF'
name: chaos
on:
  schedule:
    - cron: "0 3 * * 0"
jobs:
  soak:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Chaos/soak scaffold."
EOF

mkf "$ROOT/.github/workflows/public-api.yml" <<'EOF'
name: public-api
on: [pull_request]
jobs:
  guard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Public API guard scaffold."
EOF

# 29) Verification — ensure we didn’t miss anything
EXPECTED=(
  "Cargo.toml" "README.md" "CHANGELOG.md" "LICENSE-APACHE" "LICENSE-MIT" "CODEOWNERS" "rust-toolchain.toml" "deny.toml" ".cargo/config.toml" ".gitignore"
  "src/main.rs" "src/lib.rs"
  "src/config/mod.rs" "src/config/env.rs" "src/config/file.rs" "src/config/validate.rs" "src/config/reload.rs"
  "src/bootstrap/mod.rs" "src/bootstrap/server.rs" "src/bootstrap/metrics_server.rs" "src/bootstrap/health_probe.rs"
  "src/admission/mod.rs" "src/admission/quotas.rs" "src/admission/fair_queue.rs"
  "src/auth/mod.rs" "src/auth/capability.rs" "src/auth/passport_client.rs" "src/auth/revocation.rs"
  "src/middleware/mod.rs" "src/middleware/body_caps.rs" "src/middleware/decompress_guard.rs" "src/middleware/corr_id.rs" "src/middleware/classify.rs" "src/middleware/slow_loris.rs"
  "src/routes/mod.rs" "src/routes/ops.rs" "src/routes/v1/mod.rs" "src/routes/v1/objects.rs" "src/routes/v1/index.rs" "src/routes/v1/mailbox.rs" "src/routes/v1/dht.rs"
  "src/routes/v1/facet/mod.rs" "src/routes/v1/facet/media.rs" "src/routes/v1/facet/graph.rs" "src/routes/v1/facet/feed.rs"
  "src/hydration/mod.rs" "src/hydration/planner.rs" "src/hydration/compose.rs"
  "src/downstream/mod.rs" "src/downstream/index_client.rs" "src/downstream/storage_client.rs" "src/downstream/mailbox_client.rs" "src/downstream/dht_client.rs" "src/downstream/latency.rs" "src/downstream/hedge.rs"
  "src/readiness/mod.rs" "src/readiness/keys.rs" "src/readiness/policy.rs"
  "src/metrics/mod.rs" "src/metrics/registry.rs"
  "src/errors/mod.rs" "src/errors/http_map.rs" "src/errors/reasons.rs"
  "src/observability/mod.rs" "src/observability/logging.rs" "src/observability/tracing_spans.rs"
  "src/runtime/mod.rs" "src/runtime/supervisor.rs" "src/runtime/worker.rs" "src/runtime/channels.rs" "src/runtime/shutdown.rs"
  "src/pq/mod.rs" "src/pq/negotiate.rs"
  "src/zk/mod.rs" "src/zk/receipts.rs" "src/zk/no_mutate.rs"
  "src/types/mod.rs" "src/types/dto.rs"
  "benches/hydration.rs" "benches/media_range.rs"
  "tests/interop_vectors.rs" "tests/readyz_overload.rs" "tests/oap_limits.rs" "tests/policy_gate.rs" "tests/metrics_contract.rs" "tests/loom_fanout.rs" "tests/hardening.rs" "tests/zk_read_only.rs"
  "fuzz/fuzz_targets/capability.rs" "fuzz/fuzz_targets/decompress_guard.rs" "fuzz/fuzz_targets/headers.rs" "fuzz/Cargo.toml"
  "docs/API.md" "docs/CONFIG.md" "docs/CONCURRENCY.md" "docs/GOVERNANCE.md" "docs/IDB.md" "docs/INTEROP.md" "docs/OBSERVABILITY.md" "docs/PERFORMANCE.md" "docs/QUANTUM.md" "docs/RUNBOOK.md" "docs/SECURITY.md" "docs/TESTS.md"
  "docs/mermaid/arch.mmd" "docs/mermaid/sequence.mmd" "docs/mermaid/states.mmd"
  "docs/api-history/omnigate/<rev>.txt"
  "specs/readiness.tla" "specs/admission.tla" "specs/README.md"
  "configs/omnigate.toml" "configs/staging.toml" "configs/amnesia.toml"
  "testing/performance/hydrate_mix.sh" "testing/performance/baselines/p95_hydration.json" "testing/performance/baselines/p95_range.json"
  "testing/chaos/scenario.yml" "testing/chaos/README.md"
  "testing/vectors/omnigate/range_read.json" "testing/vectors/omnigate/error_413.json" "testing/vectors/omnigate/unauth_401.json"
  "scripts/check_boundary.sh" "scripts/render_mermaid.sh" "scripts/ci_metrics_guard.sh" "scripts/inject_faults.sh" "scripts/soak.sh" "scripts/hnDL_sim.sh"
  ".github/workflows/ci.yml" ".github/workflows/render-mermaid.yml" ".github/workflows/perf-regression.yml" ".github/workflows/chaos.yml" ".github/workflows/public-api.yml"
)

missing=0
for rel in "${EXPECTED[@]}"; do
  if [[ ! -e "$ROOT/$rel" ]]; then
    echo "MISSING: $rel"
    missing=$((missing+1))
  fi
done

echo
echo "Created files: $(find "$ROOT" -type f | wc -l)"
echo "Missing files: $missing"
echo
echo "Tree:"
( cd "$ROOT" && find . -type d -print | sed 's|[^-][^/]*/|  |g;s|/|- |' )

if [[ $missing -ne 0 ]]; then
  echo "ERROR: Scaffold incomplete."
  exit 1
else
  echo "SUCCESS: Scaffold matches the requested file tree."
fi
