
#!/usr/bin/env bash
# Scaffolder for crates/svc-passport2 — structure only, no Rust logic.
# macOS/BSD compatible. Idempotent unless OVERWRITE=1.
# Usage:
#   bash scripts/scaffold_svc_passport2.sh
#   OVERWRITE=1 bash scripts/scaffold_svc_passport2.sh
set -euo pipefail

ROOT="crates/svc-passport2"
OVERWRITE="${OVERWRITE:-0}"

say() { printf "%s\n" "$*"; }
err() { printf "ERROR: %s\n" "$*" >&2; exit 1; }

[ -d "$ROOT" ] || err "Expected existing crate directory: $ROOT"

mkd() { mkdir -p "$1"; say "dir: $1"; }

write() {
  # write <path>  (content is read from stdin via heredoc)
  local path="$1"
  local dir; dir="$(dirname "$path")"
  mkdir -p "$dir"
  if [ -e "$path" ] && [ "$OVERWRITE" != "1" ]; then
    say "skip (exists): $path"
    # Drain stdin to avoid blocking if piped
    cat >/dev/null || true
    return 0
  fi
  say "write: $path"
  cat >"$path"
}

# --- 1) Directories -----------------------------------------------------------
while IFS= read -r d; do
  [ -z "$d" ] && continue
  mkd "$ROOT/$d"
done <<'DIRS'
.
src
src/http
src/http/handlers
src/dto
src/token
src/kms
src/verify
src/state
src/policy
src/bus
src/telemetry
src/util
tests
loom
fuzz
fuzz/fuzz_targets
benches
examples
docs
scripts
testing
testing/profiles
testing/data
config
docker
.github
.github/workflows
DIRS

# --- 2) Top-level files -------------------------------------------------------
write "$ROOT/README.md" <<'EOF'
# svc-passport2 — scaffolded structure (no implementation)
See docs/ for API, CONFIG, CONCURRENCY, QUANTUM, TESTS, INTEROP. HTTP is canonical.
/v1/passport/issue|verify|revoke; token logic is macaroon-style, KMS rotation isolated.
EOF

write "$ROOT/CHANGELOG.md" <<'EOF'
# Changelog (svc-passport2)
## Unreleased
- Scaffold created: directories and placeholder files, no logic.
EOF

write "$ROOT/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["rustfmt", "clippy"]
EOF

write "$ROOT/.gitignore" <<'EOF'
/target
**/*.rs.bk
.DS_Store
.env
/tmp
EOF

# --- 3) src/ roots ------------------------------------------------------------
write "$ROOT/src/lib.rs" <<'EOF'
//! svc-passport2 library surface (scaffold). Keep modules <300 LOC when implemented.
pub mod config;
pub mod error;
pub mod metrics;
pub mod health;
pub mod bootstrap;

pub mod http;
pub mod dto;
pub mod token;
pub mod kms;
pub mod verify;
pub mod state;
pub mod policy;
pub mod bus;
pub mod telemetry;
pub mod util;

// Re-export candidates (uncomment when implemented):
// pub use crate::{config::Config, metrics::Metrics, health::Health};
EOF

write "$ROOT/src/main.rs" <<'EOF'
//! Binary entrypoint (scaffold only). No runtime logic yet.
fn main() {
    println!("svc-passport2 scaffold created.");
}
EOF

write "$ROOT/src/config.rs" <<'EOF'
//! Configuration loader (scaffold). Env-first; hot-reload planned via config watcher.
pub struct Config; // TODO: define per docs/CONFIG.md
EOF

write "$ROOT/src/error.rs" <<'EOF'
//! Unified error taxonomy (scaffold). See README §10 for the table.
pub enum Error {
    // ConfigError,
    // KeyError,
    // MintError,
    // StateError,
}
EOF

write "$ROOT/src/metrics.rs" <<'EOF'
//! Prometheus metrics (scaffold): counters/histograms/gauges.
pub struct Metrics; // TODO per README §6
EOF

write "$ROOT/src/health.rs" <<'EOF'
//! Liveness/readiness state (scaffold).
pub struct Health; // TODO
EOF

write "$ROOT/src/bootstrap.rs" <<'EOF'
//! Process wiring (scaffold): config, telemetry, router, background tasks.
pub fn plan() {
    // TODO: construct dependencies; no side-effects in scaffold.
}
EOF

# --- 4) HTTP stack ------------------------------------------------------------
write "$ROOT/src/http/mod.rs" <<'EOF'
//! Axum integration (scaffold): router, middleware, uniform errors.
pub mod router;
pub mod middleware;
pub mod handlers;
EOF

write "$ROOT/src/http/router.rs" <<'EOF'
//! Router declaration (scaffold).
//! Endpoints: /v1/passport/issue | /verify | /revoke | /healthz | /readyz | /metrics
pub fn plan_routes() {
    // TODO
}
EOF

write "$ROOT/src/http/middleware.rs" <<'EOF'
//! Middleware (scaffold): tracing, timeouts, body limits, request-id.
pub fn plan_layers() {
    // TODO
}
EOF

write "$ROOT/src/http/handlers/mod.rs" <<'EOF'
//! HTTP handlers (scaffold). Keep each handler small.
pub mod issue;
pub mod verify;
pub mod revoke;
pub mod healthz;
pub mod readyz;
EOF

write "$ROOT/src/http/handlers/issue.rs" <<'EOF'
//! POST /v1/passport/issue (scaffold)
EOF

write "$ROOT/src/http/handlers/verify.rs" <<'EOF'
//! POST /v1/passport/verify (preflight, non-authoritative) — scaffold
EOF

write "$ROOT/src/http/handlers/revoke.rs" <<'EOF'
//! POST /v1/passport/revoke (scaffold)
EOF

write "$ROOT/src/http/handlers/healthz.rs" <<'EOF'
//! GET /healthz (scaffold)
EOF

write "$ROOT/src/http/handlers/readyz.rs" <<'EOF'
//! GET /readyz (scaffold)
EOF

# --- 5) DTOs ------------------------------------------------------------------
write "$ROOT/src/dto/mod.rs" <<'EOF'
//! DTOs (scaffold): serde-enabled request/response types.
pub mod issue;
pub mod verify;
pub mod revoke;
EOF

write "$ROOT/src/dto/issue.rs" <<'EOF'
//! Issue request/response DTOs (scaffold). See docs/API.md.
EOF

write "$ROOT/src/dto/verify.rs" <<'EOF'
//! Verify (preflight) request/response DTOs (scaffold). See docs/API.md.
EOF

write "$ROOT/src/dto/revoke.rs" <<'EOF'
//! Revoke request/response DTOs (scaffold). See docs/API.md.
EOF

# --- 6) Token logic -----------------------------------------------------------
write "$ROOT/src/token/mod.rs" <<'EOF'
//! Token (macaroon-style) module (scaffold): Signer/Passport traits.
pub mod macaroon;
pub mod caveat;
pub mod attenuate;
pub mod encode;
EOF

write "$ROOT/src/token/macaroon.rs" <<'EOF'
//! Signing & envelope glue (scaffold). PQ-hybrid seam is feature-gated.
EOF

write "$ROOT/src/token/caveat.rs" <<'EOF'
//! Caveat whitelist & parser (scaffold). See README Supported Caveats table.
EOF

write "$ROOT/src/token/attenuate.rs" <<'EOF'
//! Attenuation rules (scaffold): always reduce authority, never widen.
EOF

write "$ROOT/src/token/encode.rs" <<'EOF'
//! Encoding helpers (scaffold): url-safe base64, size enforcement.
EOF

# --- 7) KMS layer -------------------------------------------------------------
write "$ROOT/src/kms/mod.rs" <<'EOF'
//! KMS abstraction & rotation worker (scaffold).
pub mod client;
pub mod rotation;
pub mod keyslot;
EOF

write "$ROOT/src/kms/client.rs" <<'EOF'
//! ron-kms client adapter (scaffold): timeouts/retries planned.
EOF

write "$ROOT/src/kms/rotation.rs" <<'EOF'
//! Rotation worker (scaffold): atomic swap of active KeySlot, jittered retries.
EOF

write "$ROOT/src/kms/keyslot.rs" <<'EOF'
//! KeySlot (scaffold): {kid, alg, not_before, not_after}, zeroize on drop planned.
EOF

# --- 8) Verify (preflight) ----------------------------------------------------
write "$ROOT/src/verify/mod.rs" <<'EOF'
//! Preflight verification module (scaffold). Authority remains in ron-auth.
pub mod preflight;
EOF

write "$ROOT/src/verify/preflight.rs" <<'EOF'
//! Cheap structural checks and warnings (scaffold).
EOF

# --- 9) State & audit (no PII) ------------------------------------------------
write "$ROOT/src/state/mod.rs" <<'EOF'
//! Service shared state container (scaffold).
pub mod issuer;
pub mod audit;
EOF

write "$ROOT/src/state/issuer.rs" <<'EOF'
//! In-RAM issuer registry: active/old KIDs, grace window, epoch counter (scaffold).
EOF

write "$ROOT/src/state/audit.rs" <<'EOF'
//! Minimal non-identifying audit (scaffold): time, kid, op, audience_hash.
EOF

# --- 10) Policy hooks (optional) ---------------------------------------------
write "$ROOT/src/policy/mod.rs" <<'EOF'
//! Optional policy engine adapter (scaffold). Feature-gated in Cargo when added.
pub mod eval;
EOF

write "$ROOT/src/policy/eval.rs" <<'EOF'
//! Adapter to ron-policy (scaffold).
EOF

# --- 11) Bus RPC (feature = "bus-rpc") ---------------------------------------
write "$ROOT/src/bus/mod.rs" <<'EOF'
//! Experimental Bus RPC surface (scaffold). HTTP remains canonical.
pub mod rpc;
EOF

write "$ROOT/src/bus/rpc.rs" <<'EOF'
//! mint_cap RPC handler stubs (scaffold).
EOF

# --- 12) Telemetry ------------------------------------------------------------
write "$ROOT/src/telemetry/mod.rs" <<'EOF'
//! Telemetry module (scaffold): tracing + prometheus setup.
pub mod tracing_init;
pub mod prometheus;
EOF

write "$ROOT/src/telemetry/tracing_init.rs" <<'EOF'
//! Tracing subscriber wiring (scaffold).
EOF

write "$ROOT/src/telemetry/prometheus.rs" <<'EOF'
//! Prometheus exporter wiring (scaffold).
EOF

# --- 13) Utilities ------------------------------------------------------------
write "$ROOT/src/util/mod.rs" <<'EOF'
//! Utility prelude (scaffold).
pub mod time;
pub mod id;
pub mod hashing;
EOF

write "$ROOT/src/util/time.rs" <<'EOF'
//! Time helpers (scaffold): TTL math, monotonic deadlines.
EOF

write "$ROOT/src/util/id.rs" <<'EOF'
//! Correlation/request IDs; token refs (scaffold).
EOF

write "$ROOT/src/util/hashing.rs" <<'EOF'
//! Salted hashing for privacy-preserving audit (scaffold).
EOF

# --- 14) tests/ ---------------------------------------------------------------
write "$ROOT/tests/api_issue.rs" <<'EOF'
// Black-box test scaffold for /v1/passport/issue
#[test] fn issue_scaffold() { assert!(true); }
EOF

write "$ROOT/tests/api_verify.rs" <<'EOF'
// Black-box test scaffold for /v1/passport/verify
#[test] fn verify_scaffold() { assert!(true); }
EOF

write "$ROOT/tests/api_revoke.rs" <<'EOF'
// Black-box test scaffold for /v1/passport/revoke
#[test] fn revoke_scaffold() { assert!(true); }
EOF

write "$ROOT/tests/readiness.rs" <<'EOF'
// Readiness gate scaffold
#[test] fn readiness_scaffold() { assert!(true); }
EOF

write "$ROOT/tests/invariants.rs" <<'EOF'
// Property/invariant test scaffold: attenuation never widens authority.
#[test] fn invariants_scaffold() { assert!(true); }
EOF

# --- 15) loom/ & fuzz/ & benches/ & examples/ --------------------------------
write "$ROOT/loom/rotation_loom.rs" <<'EOF'
// Loom model scaffold for rotation/mint interleavings
#[test] fn loom_rotation_scaffold() { assert!(true); }
EOF

write "$ROOT/fuzz/fuzz_targets/envelope_parse.rs" <<'EOF'
#![no_main]
// Fuzz target scaffold: envelope_parse
use libfuzzer_sys::fuzz_target;
fuzz_target!(|_data: &[u8]| {
    // TODO
});
EOF

write "$ROOT/fuzz/fuzz_targets/caveat_mutation.rs" <<'EOF'
#![no_main]
// Fuzz target scaffold: caveat_mutation
use libfuzzer_sys::fuzz_target;
fuzz_target!(|_data: &[u8]| {
    // TODO
});
EOF

write "$ROOT/benches/issue_bench.rs" <<'EOF'
// Criterion bench scaffold: /issue path
fn main() { /* bench placeholder */ }
EOF

write "$ROOT/examples/issue_cli.rs" <<'EOF'
// Minimal example scaffold: issues a passport via HTTP (to be implemented)
fn main() {
    println!("issue_cli scaffold");
}
EOF

# --- 16) docs/ ----------------------------------------------------------------
write "$ROOT/docs/API.md" <<'EOF'
# API (svc-passport2)
This is a placeholder in the scaffold; see workspace canonical docs.
EOF

write "$ROOT/docs/CONFIG.md" <<'EOF'
# CONFIG (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/CONCURRENCY.md" <<'EOF'
# CONCURRENCY (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/GOVERNANCE.md" <<'EOF'
# GOVERNANCE (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/IDB.md" <<'EOF'
# IDB (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/OBSERVABILITY.md" <<'EOF'
# OBSERVABILITY (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/QUANTUM.md" <<'EOF'
# QUANTUM (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/TESTS.md" <<'EOF'
# TESTS (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/INTEROP.md" <<'EOF'
# INTEROP (svc-passport2)
Placeholder in scaffold.
EOF

write "$ROOT/docs/arch.mmd" <<'EOF'
%% Mermaid architecture diagram (placeholder)
flowchart LR
  A[client] --> B(svc-passport2)
  B --> C[ron-kms]
  B --> D[ron-auth]
EOF

write "$ROOT/docs/sequence.mmd" <<'EOF'
%% Mermaid sequence (placeholder)
sequenceDiagram
  participant App
  participant P as svc-passport2
  App->>P: /v1/passport/issue
  P-->>App: 201 {token}
EOF

write "$ROOT/docs/state.mmd" <<'EOF'
%% Mermaid state (placeholder)
stateDiagram-v2
  [*] --> Idle
  Idle --> Running
  Running --> Shutdown
  Shutdown --> [*]
EOF

# --- 17) scripts/ & testing/ --------------------------------------------------
write "$ROOT/scripts/inject_kms_latency.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "[scaffold] inject_kms_latency.sh <latency>" >&2
EOF
chmod +x "$ROOT/scripts/inject_kms_latency.sh"

write "$ROOT/scripts/rotate_under_load.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "[scaffold] rotate_under_load.sh" >&2
EOF
chmod +x "$ROOT/scripts/rotate_under_load.sh"

write "$ROOT/scripts/load_issue_profile.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "[scaffold] load_issue_profile.sh <profile.json>" >&2
EOF
chmod +x "$ROOT/scripts/load_issue_profile.sh"

write "$ROOT/testing/profiles/issue_80_20_local.json" <<'EOF'
{
  "description": "80% issue, 15% verify, 5% revoke (local)",
  "rps": 100,
  "duration_s": 60
}
EOF

# --- 18) config/ --------------------------------------------------------------
write "$ROOT/config/local.env.example" <<'EOF'
BIND=127.0.0.1:9085
METRICS_ADDR=127.0.0.1:9605
DEFAULT_TTL_SECS=900
MAX_TTL_SECS=3600
AMNESIA=true
EOF

write "$ROOT/config/default.toml" <<'EOF'
# default config (scaffold)
[service]
bind = "127.0.0.1:9085"
[metrics]
addr = "127.0.0.1:9605"
[policy]
enabled = false
EOF

# --- 19) docker/ --------------------------------------------------------------
write "$ROOT/docker/Dockerfile" <<'EOF'
# Dockerfile scaffold (no build steps yet)
FROM scratch
EOF

write "$ROOT/docker/docker-compose.yml" <<'EOF'
version: "3.9"
services:
  svc-passport2:
    image: svcpassport2:scaffold
    build:
      context: ..
      dockerfile: docker/Dockerfile
EOF

# --- 20) GitHub Workflows -----------------------------------------------------
write "$ROOT/.github/workflows/ci.yml" <<'EOF'
name: ci-scaffold
on: [push, pull_request]
jobs:
  noop:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "ci scaffold"
EOF

write "$ROOT/.github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid-scaffold
on: [push, pull_request]
jobs:
  noop:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "render scaffold"
EOF

write "$ROOT/.github/workflows/obs.yml" <<'EOF'
name: obs-scaffold
on: [push, pull_request]
jobs:
  noop:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "obs scaffold"
EOF

say "✅ Scaffold complete in $ROOT"

