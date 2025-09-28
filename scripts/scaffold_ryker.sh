#!/usr/bin/env bash
# Scaffolds the crates/ryker/ directory with the finalized file tree and stubbed files.
# Usage:
#   scripts/scaffold_ryker.sh [repo_root] [--force]
# Examples:
#   scripts/scaffold_ryker.sh .
#   scripts/scaffold_ryker.sh . --force

set -euo pipefail

ROOT="${1:-.}"
FORCE="${2:-}"
CRATE_DIR="$ROOT/crates/ryker"

overwrite_ok() {
  [[ "$FORCE" == "--force" ]]
}

mkd() {
  mkdir -p "$1"
}

mkfile() {
  local path="$1"
  shift
  local content="$*"
  if [[ -e "$path" ]] && ! overwrite_ok; then
    echo "exists: $path (skip; use --force to overwrite)"
    return 0
  fi
  mkdir -p "$(dirname "$path")"
  printf "%s\n" "$content" > "$path"
  echo "wrote:  $path"
}

# ---------- Directories ----------
DIRS=(
  "$CRATE_DIR/scripts"
  "$CRATE_DIR/docs"
  "$CRATE_DIR/docs/schemas"
  "$CRATE_DIR/docs/api-history/ryker"
  "$CRATE_DIR/docs/benches"
  "$CRATE_DIR/src"
  "$CRATE_DIR/src/config"
  "$CRATE_DIR/src/runtime"
  "$CRATE_DIR/src/supervisor"
  "$CRATE_DIR/src/mailbox"
  "$CRATE_DIR/src/observe"
  "$CRATE_DIR/examples"
  "$CRATE_DIR/tests"
  "$CRATE_DIR/tests/integration"
  "$CRATE_DIR/tests/loom"
  "$CRATE_DIR/tests/vectors/env"
  "$CRATE_DIR/tests/vectors/snapshots"
  "$CRATE_DIR/benches"
  "$CRATE_DIR/fuzz/fuzz_targets"
)
for d in "${DIRS[@]}"; do mkd "$d"; done

# ---------- Root files ----------
mkfile "$CRATE_DIR/Cargo.toml" "\
# ryker — Cargo.toml (library-only). Fill with actual deps/features when coding.
[package]
name = \"ryker\"
version = \"0.1.0\"
edition = \"2021\"
license = \"MIT OR Apache-2.0\"
description = \"Embedded actor & bounded mailbox runtime (library-only)\"
repository = \"\"
readme = \"README.md\"

[lib]
path = \"src/lib.rs\"

[features]
default = [\"metrics\", \"tracing\", \"amnesia\"]
metrics = []
tracing = []
amnesia = []
loom = []
dev-cli = []
# pq-pass is intentionally omitted; PQ is N/A for ryker as a lib.

[package.metadata.docs.rs]
all-features = true

[dependencies]
# Populate later: small, audited set only.

[dev-dependencies]
criterion = \"*\"
"

mkfile "$CRATE_DIR/README.md" "\
# ryker
# NOTE: This is a placeholder if README doesn't exist; replace/keep your finalized README."

mkfile "$CRATE_DIR/CHANGELOG.md" "\
# Changelog — ryker
All notable changes to this project will be documented here (SemVer)."

mkfile "$CRATE_DIR/LICENSE-MIT" "\
MIT License
Copyright (c) $(date +%Y) Stevan"

mkfile "$CRATE_DIR/LICENSE-APACHE" "\
                                 Apache License
                           Version 2.0, January 2004
# Stub license file — replace with full text if needed."

mkfile "$CRATE_DIR/rust-toolchain.toml" "\
[toolchain]
channel = \"1.80.0\"
components = [\"rustfmt\", \"clippy\"]"

mkfile "$CRATE_DIR/.clippy.toml" "\
warns = []
deny = [\"clippy::await_holding_lock\"]
# Consider adding: large_enum_variant, needless_pass_by_value (as policy)."

mkfile "$CRATE_DIR/.gitignore" "\
target/
**/target/
Cargo.lock
*.svg
fuzz/artifacts/
criterion/
"

mkfile "$CRATE_DIR/ryker.example.toml" "\
# Example config — validated via RykerConfig::from_env_validated()
[mailbox]
capacity = 256
max_msg_bytes = \"64KiB\"
default_deadline_ms = 1000

[fairness]
batch_messages = 32
yield_every_n = 64

[supervisor]
backoff_base_ms = 100
backoff_cap_ms  = 5000

[observe]
metrics = true
queue_depth_sampling = true

[amnesia]
enabled = false
"

# ---------- scripts ----------
mkfile "$CRATE_DIR/scripts/render-mermaid.sh" "\
#!/usr/bin/env bash
set -euo pipefail
# Renders docs/*.mmd to SVG using mermaid-cli if available.
ROOT=\${1:-\"$(dirname "$CRATE_DIR")\"}
cd \"$CRATE_DIR\"
if ! command -v mmdc >/dev/null 2>&1; then
  echo \"mmdc not found. Install: npm i -g @mermaid-js/mermaid-cli\"
  exit 1
fi
for f in docs/*.mmd; do
  [ -e \"\$f\" ] || continue
  out=\${f%.mmd}.svg
  mmdc -i \"\$f\" -o \"\$out\"
  echo \"rendered: \$out\"
done
"

mkfile "$CRATE_DIR/scripts/public_api_snapshot.sh" "\
#!/usr/bin/env bash
set -euo pipefail
cd \"$(dirname "$CRATE_DIR")/..\"
if ! command -v cargo-public-api >/dev/null 2>&1; then
  echo \"cargo-public-api not found (cargo install cargo-public-api)\"
  exit 1
fi
cargo public-api --manifest-path crates/ryker/Cargo.toml --simplified \
  > crates/ryker/docs/api-history/ryker/vX.Y.Z.txt
echo \"updated public API snapshot -> docs/api-history/ryker/vX.Y.Z.txt\"
"

mkfile "$CRATE_DIR/scripts/dev-notes.md" "\
# Dev notes — ryker
- Loom: run targeted: RUSTFLAGS=\"--cfg loom\" cargo test -p ryker --test loom/loom_mailbox_basic -- --nocapture
- Fuzz: cargo fuzz run fuzz_parse_config_toml -- -max_total_time=60
- Bench: cargo bench -p ryker && see docs/benches/README.md
"

# ---------- docs ----------
mkfile "$CRATE_DIR/docs/API.md" "# API contract — ryker\n(Stub; replace with your finalized document.)"
mkfile "$CRATE_DIR/docs/CONFIG.md" "# CONFIG — precedence & validation\n(Stub; replace with your finalized document.)"
mkfile "$CRATE_DIR/docs/CONCURRENCY.md" "# CONCURRENCY — invariants & loom plan\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/GOVERNANCE.md" "# GOVERNANCE — decisions & appeals\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/IDB.md" "# IDB — invariants/design/implementation/gates\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/INTEROP.md" "# INTEROP — host integration boundaries\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/OBSERVABILITY.md" "# OBSERVABILITY — metrics & tracing\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/PERFORMANCE.md" "# PERFORMANCE — bench protocol & SLOs\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/QUANTUM.md" "# QUANTUM — PQ exposure N/A; amnesia semantics\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/RUNBOOK.md" "# RUNBOOK — operator flow (even as a lib)\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/SECURITY.md" "# SECURITY — threat model, zeroize scope, supply chain\n(Stub; replace.)"
mkfile "$CRATE_DIR/docs/TESTS.md" "# TESTS — Bronze→Silver→Gold gates\n(Stub; replace.)"

mkfile "$CRATE_DIR/docs/arch.mmd" "\
flowchart LR
  A[Caller] -->|enqueue| B(ryker::Mailbox<T>)
  B -->|pull| C[Actor]
  C --> D[[Metrics via host]]
  C --> E[[Tracing]]
"
mkfile "$CRATE_DIR/docs/sequence.mmd" "\
sequenceDiagram
  actor Caller
  participant MB as Mailbox<T>
  participant ACT as Actor
  Caller->>MB: try_send(Request{deadline})
  MB-->>Caller: Busy? (if full)
  ACT->>MB: pull()
  ACT-->>Caller: reply (ok/timeout)
"
mkfile "$CRATE_DIR/docs/state.mmd" "\
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: spawn(actor)
  Running --> Backoff: panic
  Backoff --> Running: jittered restart
  Running --> Shutdown: cancel
  Shutdown --> [*]
"

mkfile "$CRATE_DIR/docs/schemas/ryker.config.schema.json" "\
{
  \"\$schema\": \"https://json-schema.org/draft/2020-12/schema\",
  \"\$id\": \"https://rustyonions.local/schemas/ryker.config.schema.json\",
  \"title\": \"RykerConfig\",
  \"type\": \"object\",
  \"additionalProperties\": false
}
"

mkfile "$CRATE_DIR/docs/api-history/ryker/vX.Y.Z.txt" "\
# Generated by cargo public-api --simplified
# Update on public surface changes.
"

mkfile "$CRATE_DIR/docs/governance_history.md" "\
# Governance history — ryker
- YYYY-MM-DD: Initialized."

mkfile "$CRATE_DIR/docs/benches/README.md" "\
# Benches — ryker
- Baselines live under target/criterion (local).
- Compare runs with cargo-criterion or Criterion reports."

# ---------- src ----------
mkfile "$CRATE_DIR/src/lib.rs" "\
/*! ryker — embedded actor & bounded mailbox runtime (library-only).
    Public facade only. See README for invariants and acceptance gates. */
pub mod prelude;
pub mod errors;
pub mod config;
pub mod runtime;
pub mod supervisor;
pub mod mailbox;
pub mod observe;
"

mkfile "$CRATE_DIR/src/prelude.rs" "\
/*! Public prelude — curated exports for convenient use. */
pub use crate::config::*;
pub use crate::runtime::*;
pub use crate::mailbox::*;
pub use crate::supervisor::*;
"

mkfile "$CRATE_DIR/src/errors.rs" "\
/*! Error taxonomy (stub). Keep variants stable; map to host semantics. */
#[allow(dead_code)]
pub enum Error {
    Busy,
    TooLarge,
    Closed,
    Timeout,
}
"

# config
mkfile "$CRATE_DIR/src/config/mod.rs" "\
/*! Config facade (stub). */
pub mod model;
pub mod loader;
pub mod reload;

pub use model::*;
"
mkfile "$CRATE_DIR/src/config/model.rs" "\
/*! Typed config model (stub). Enforce strict validation; deny unknown fields. */
#[allow(dead_code)]
pub struct RykerConfig;
"
mkfile "$CRATE_DIR/src/config/loader.rs" "\
/*! Loader (stub). Precedence: Builder -> Env(RYKER_*) -> File -> Defaults. */
"
mkfile "$CRATE_DIR/src/config/reload.rs" "\
/*! Reload hooks (stub). Track ryker_config_reload_total/errors_total. */
"

# runtime
mkfile "$CRATE_DIR/src/runtime/mod.rs" "\
/*! Runtime facade (stub). Runtime-agnostic; no executor ownership. */
pub mod runtime;
pub use runtime::Runtime;
"
mkfile "$CRATE_DIR/src/runtime/runtime.rs" "\
/*! Tiny glue type (stub). Provides mailbox builders; no background tasks. */
#[allow(dead_code)]
pub struct Runtime;
"

# supervisor
mkfile "$CRATE_DIR/src/supervisor/mod.rs" "\
/*! Supervisor facade (stub). */
pub mod backoff;
pub mod supervisor;
"
mkfile "$CRATE_DIR/src/supervisor/backoff.rs" "\
/*! Decorrelated jitter algorithm (stub). Property-testable. */
"
mkfile "$CRATE_DIR/src/supervisor/supervisor.rs" "\
/*! Restart wrapper (stub). Never hold locks across .await. */
"

# mailbox
mkfile "$CRATE_DIR/src/mailbox/mod.rs" "\
/*! Mailbox facade (stub). */
pub mod builder;
pub mod queue;
pub mod observer;
pub mod error;

pub use builder::MailboxBuilder;
pub use queue::Mailbox;
"
mkfile "$CRATE_DIR/src/mailbox/builder.rs" "\
/*! MailboxBuilder (stub). Capacity/size/deadline/fairness with hot/cold rules. */
#[allow(dead_code)]
pub struct MailboxBuilder<T>(std::marker::PhantomData<T>);
"
mkfile "$CRATE_DIR/src/mailbox/queue.rs" "\
/*! Bounded single-consumer queue (stub). Explicit Busy; sampled depth. */
#[allow(dead_code)]
pub struct Mailbox<T>(std::marker::PhantomData<T>);
"
mkfile "$CRATE_DIR/src/mailbox/observer.rs" "\
/*! MailboxObserver hooks (stub). Forward to metrics facade without exporter lock-in. */
"
mkfile "$CRATE_DIR/src/mailbox/error.rs" "\
/*! Mailbox-specific errors (stub). Busy/TooLarge/Closed/Deadline. */
"

# observe
mkfile "$CRATE_DIR/src/observe/mod.rs" "\
/*! Observability facade (stub). Feature-gated; keep labels low-cardinality. */
pub mod metrics;
pub mod trace;
"
mkfile "$CRATE_DIR/src/observe/metrics.rs" "\
/*! Metrics facade (stub). Names:
    ryker_mailbox_depth
    ryker_mailbox_dropped_total{reason}
    ryker_busy_rejections_total
    ryker_handler_latency_seconds{outcome}
    ryker_actor_restarts_total
*/"
mkfile "$CRATE_DIR/src/observe/trace.rs" "\
/*! Tracing helpers (stub). Spans: ryker.mailbox.enqueue, ryker.actor.handle, ryker.config.reload. */
"

# ---------- examples ----------
mkfile "$CRATE_DIR/examples/actor_loop.rs" "\
/*! Minimal actor loop (stub). See README for full example. */"
mkfile "$CRATE_DIR/examples/config_dump.rs" "\
/*! Feature-gated config dump tool (stub). */"

# ---------- tests ----------
mkfile "$CRATE_DIR/tests/integration/config_env_snapshot.rs" "\
/*! Gate: config precedence & validation (stub test). */"
mkfile "$CRATE_DIR/tests/integration/backpressure.rs" "\
/*! Gate: full queue -> Busy + dropped_total{reason=capacity} (stub test). */"
mkfile "$CRATE_DIR/tests/integration/deadline.rs" "\
/*! Gate: deadline -> outcome=timeout in histogram (stub test). */"
mkfile "$CRATE_DIR/tests/integration/reload_hot_cold.rs" "\
/*! Gate: hot(deadline/fairness) vs cold(capacity/size) reload semantics (stub test). */"
mkfile "$CRATE_DIR/tests/integration/amnesia.rs" "\
/*! Gate: amnesia zeroize behavior on/off (stub test). */"
mkfile "$CRATE_DIR/tests/integration/supervisor_backoff.rs" "\
/*! Gate: decorrelated jitter bounds + rapid-fail ceiling (stub test). */"
mkfile "$CRATE_DIR/tests/integration/metrics_contract.rs" "\
/*! Gate: metrics contract golden test (stub). Compare to vectors/snapshots/metrics_contract.txt. */"

mkfile "$CRATE_DIR/tests/feature_matrix.rs" "\
/*! Compile-only feature matrix checks (stub). Ensures public surface builds across combos. */"

mkfile "$CRATE_DIR/tests/loom/loom_mailbox_basic.rs" "\
/*! Loom: N producers -> 1 consumer; no deadlocks; FIFO per mailbox (stub). */"
mkfile "$CRATE_DIR/tests/loom/loom_shutdown.rs" "\
/*! Loom: graceful shutdown; cancel-safe; no double-drop (stub). */"
mkfile "$CRATE_DIR/tests/loom/loom_backpressure.rs" "\
/*! Loom: deterministic Busy under contention (stub). */"

mkfile "$CRATE_DIR/tests/vectors/snapshots/config_snapshot.toml" "# snapshot placeholder"
mkfile "$CRATE_DIR/tests/vectors/snapshots/config_snapshot.json" "{ \"snapshot\": true }"
mkfile "$CRATE_DIR/tests/vectors/snapshots/metrics_contract.txt" "# expected metric names/labels go here"

# ---------- benches ----------
mkfile "$CRATE_DIR/benches/enqueue.rs" "\
/*! Criterion bench: enqueue hot path (stub). */"
mkfile "$CRATE_DIR/benches/dequeue.rs" "\
/*! Criterion bench: dequeue hot path (stub). */"
mkfile "$CRATE_DIR/benches/batch.rs" "\
/*! Criterion bench: fairness batch tuning (stub). */"

# ---------- fuzz ----------
mkfile "$CRATE_DIR/fuzz/fuzz_targets/fuzz_parse_config_toml.rs" "\
#![no_main]
// cargo-fuzz target (stub): parse TOML config."
mkfile "$CRATE_DIR/fuzz/fuzz_targets/fuzz_parse_config_json.rs" "\
#![no_main]
// cargo-fuzz target (stub): parse JSON config."
mkfile "$CRATE_DIR/fuzz/fuzz_targets/fuzz_mailbox_ops.rs" "\
#![no_main]
// cargo-fuzz target (stub): randomized mailbox ops."
