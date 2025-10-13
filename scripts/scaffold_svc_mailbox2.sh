#!/usr/bin/env bash
# Scaffolds the svc-mailbox2 crate with modular folders and placeholder files (no code).
# Idempotent: re-runnable; files are overwritten with the same placeholders.

set -euo pipefail

ROOT="crates/svc-mailbox2"

say() { printf "%s\n" "$*"; }
mkd() { mkdir -p "$1"; say "dir: $1"; }
wr()  { local fp="$1"; shift; printf "%s\n" "$*" > "$fp"; say "write: $fp"; }

mkd "$ROOT"
mkd "$ROOT/benches"
mkd "$ROOT/configs"
mkd "$ROOT/docs"
mkd "$ROOT/docs/diagrams"
mkd "$ROOT/docs/vectors"
mkd "$ROOT/examples"
mkd "$ROOT/fuzz"
mkd "$ROOT/fuzz/fuzz_targets"
mkd "$ROOT/scripts"
mkd "$ROOT/src"
mkd "$ROOT/src/observability"
mkd "$ROOT/src/auth"
mkd "$ROOT/src/http"
mkd "$ROOT/src/grpc"
mkd "$ROOT/src/domain"
mkd "$ROOT/src/runtime"
mkd "$ROOT/src/bus"
mkd "$ROOT/src/pq"
mkd "$ROOT/src/util"
mkd "$ROOT/tests"
mkd "$ROOT/.github/workflows"

# -------------------------
# Top-level crate plumbing
# -------------------------
wr "$ROOT/Cargo.toml" "\
[package]
name = \"svc-mailbox2\"
version = \"0.1.0\"
edition = \"2021\"
license = \"MIT OR Apache-2.0\"
publish = false

[features]
default = [\"tls\", \"serde\"]
tls = []
otel = []
grpc = []

[dependencies]
# Kept empty intentionally for scaffold; add workspace deps when implementing.

[dev-dependencies]
"

wr "$ROOT/README.md" "\
# svc-mailbox2

> Scaffolded crate layout mirroring svc-mailbox (store-and-forward messaging plane).
> This file is a placeholder; copy the finalized README from svc-mailbox and replace names as needed.

- Pillar: 11 — Messaging & Extensions
- Primary surfaces: HTTP/1.1+TLS, NDJSON stream, /metrics /healthz /readyz
- Invariants: at-least-once, visibility lease, idempotency triple, bounded queues, DLQ
"

wr "$ROOT/LICENSE-APACHE" "Placeholder — copy the Apache-2.0 license text here."
wr "$ROOT/LICENSE-MIT"    "Placeholder — copy the MIT license text here."
wr "$ROOT/rust-toolchain.toml" "\
[toolchain]
channel = \"1.80.0\"
components = [\"rustfmt\", \"clippy\"]
profile = \"minimal\"
"

wr "$ROOT/build.rs" "\
// build.rs placeholder — embed build metadata (e.g., git SHA) at implementation time.
fn main() {}
"

# -------------------------
# Configs
# -------------------------
wr "$ROOT/configs/svc-mailbox.toml" "\
# svc-mailbox2 default config (placeholder)
listen = \"127.0.0.1:9410\"
metrics_addr = \"127.0.0.1:9600\"
amnesia = \"on\"
visibility_ms = 5000
max_inflight = 1024
# Add more keys as you implement.
"

# -------------------------
# Docs (as code)
# -------------------------
wr "$ROOT/docs/IDB.md"              "<!-- Invariant-Driven Blueprint placeholder for svc-mailbox2 -->"
wr "$ROOT/docs/CONFIG.md"           "<!-- Configuration reference placeholder -->"
wr "$ROOT/docs/CONCURRENCY.md"      "<!-- Concurrency model placeholder: supervisor, listener, workers, scanner, reprocessor -->"
wr "$ROOT/docs/INTEROP.md"          "<!-- Interop surface placeholder: HTTP endpoints, DTOs, OAP/1 limits -->"
wr "$ROOT/docs/OBSERVABILITY.md"    "<!-- Metrics & health/readiness placeholder -->"
wr "$ROOT/docs/PERFORMANCE.md"      "<!-- SLOs & harness placeholder -->"
wr "$ROOT/docs/SECURITY.md"         "<!-- Threat model, macaroon caps, TLS posture placeholder -->"
wr "$ROOT/docs/QUANTUM.md"          "<!-- PQ milestones M0–M3 placeholder -->"
wr "$ROOT/docs/RUNBOOK.md"          "<!-- Ops runbooks placeholder: saturation, DLQ, leases -->"

wr "$ROOT/docs/diagrams/arch.mmd" "\
flowchart LR
  A[caller] -->|HTTP/NDJSON| B(svc-mailbox2)
  B --> C[(DLQ store)]
  B --> D[[Prometheus]]
  style B fill:#0b7285,stroke:#083344,color:#fff
"
wr "$ROOT/docs/diagrams/sequence.mmd" "\
sequenceDiagram
  actor Client
  participant S as svc-mailbox2
  Client->>S: POST /v1/send
  S-->>Client: 200 { msg_id, duplicate? }
  Client->>S: POST /v1/recv (visibility=5s)
  S-->>Client: batch envelopes
  Client->>S: POST /v1/ack/{msg_id}
"
wr "$ROOT/docs/diagrams/state.mmd" "\
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: start()
  Running --> Backoff: saturation/restarts
  Backoff --> Running: jittered retry
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
"

wr "$ROOT/docs/vectors/http_send.json" "\
{ \"topic\": \"user:42:inbox\", \"idem_key\": \"demo-1\", \"payload_b64\": \"SGk=\" }
"
wr "$ROOT/docs/vectors/http_recv.json" "\
{ \"topic\": \"user:42:inbox\", \"batch\": 32, \"visibility_ms\": 5000 }
"
wr "$ROOT/docs/vectors/http_ack.json" "\
{ \"msg_id\": \"00000000-0000-0000-0000-000000000000\" }
"

# -------------------------
# Examples / benches / fuzz
# -------------------------
wr "$ROOT/examples/client.rs" "\
// Example placeholder — client usage will be added during implementation.
fn main() { /* placeholder */ }
"
wr "$ROOT/benches/enqueue_dequeue.rs" "\
// Criterion benches placeholder — fill with enqueue/dequeue latency benches later.
fn main() {}
"

wr "$ROOT/fuzz/Cargo.toml" "\
[package]
name = \"svc-mailbox2-fuzz\"
version = \"0.1.0\"
publish = false
edition = \"2021\"

[dependencies]
libfuzzer-sys = { version = \"0.4\", features = [\"arbitrary-derive\"] }

[workspace]
members = []
"

wr "$ROOT/fuzz/fuzz_targets/oap_frame_parser.rs" "\
// fuzz target placeholder for OAP frame parser
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data; // placeholder
});
"
wr "$ROOT/fuzz/fuzz_targets/envelope_deser.rs" "\
// fuzz target placeholder for envelope deserialization
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data; // placeholder
});
"

# -------------------------
# Scripts
# -------------------------
wr "$ROOT/scripts/dev_run.sh" "\
#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=\${RUST_LOG:-info}
export METRICS_ADDR=\${METRICS_ADDR:-127.0.0.1:9600}
echo \"(placeholder) run svc-mailbox2 with config ./configs/svc-mailbox.toml\"
"
wr "$ROOT/scripts/render_mermaid.sh" "\
#!/usr/bin/env bash
set -euo pipefail
for f in \$(git ls-files \"$ROOT/docs/diagrams/*.mmd\" 2>/dev/null || true); do
  out=\"\${f%.mmd}.svg\"
  echo \"render: \$f -> \$out\"
  mmdc -i \"\$f\" -o \"\$out\"
done
"
wr "$ROOT/scripts/check_golden_metrics.sh" "\
#!/usr/bin/env bash
set -euo pipefail
echo \"(placeholder) curl /metrics and grep for expected metric names\"
"

# -------------------------
# Source tree (comments only)
# -------------------------
wr "$ROOT/src/main.rs" "\
// Binary entrypoint placeholder for svc-mailbox2.
// Intentionally no code — will wire config, routes, and runtime when implementing.
fn main() {}
"
wr "$ROOT/src/lib.rs" "\
// Re-exports placeholder for svc-mailbox2 library surface.
"

wr "$ROOT/src/config.rs" "\
// Typed configuration placeholder (env > file > defaults).
"
wr "$ROOT/src/error.rs" "\
// Error taxonomy placeholder: AuthError, Busy, State, Integrity...
"
wr "$ROOT/src/metrics.rs" "\
// Prometheus metrics registry and helpers placeholder.
"

wr "$ROOT/src/observability/health.rs" "\
// /healthz, /readyz, /metrics handlers placeholder.
"
wr "$ROOT/src/observability/tracing.rs" "\
// Tracing / logging setup placeholder (JSON logs, correlation IDs).
"

wr "$ROOT/src/auth/macaroon.rs" "\
// Macaroon capability verification placeholder (KID, scope checks).
"
wr "$ROOT/src/auth/caps_cache.rs" "\
// Capability verifier cache placeholder (TTL, rotation).
"

wr "$ROOT/src/http/mod.rs" "\
// HTTP router composition and middleware placeholder (limits, timeouts).
"
wr "$ROOT/src/http/routes.rs" "\
// Handlers placeholder: /v1/send, /v1/recv, /v1/recv/stream, /ack, /nack, /dlq/reprocess.
"
wr "$ROOT/src/http/dto.rs" "\
// DTOs placeholder with deny_unknown_fields and payload_hash (b3:<hex>).
"
wr "$ROOT/src/http/middleware.rs" "\
// Middleware placeholder: body limits, timeouts, correlation ID, capability extraction.
"

wr "$ROOT/src/grpc/mod.rs" "\
// Feature-gated gRPC bootstrap placeholder (disabled by default).
"
wr "$ROOT/src/grpc/proto.rs" "\
// gRPC proto stubs placeholder (ron.mailbox.v1).
"

wr "$ROOT/src/domain/envelope.rs" "\
// Envelope model placeholder: msg_id, topic, idem_key, payload_hash, attempt, version header.
"
wr "$ROOT/src/domain/shard.rs" "\
// Shard queue abstraction placeholder: put/recv/ack/nack, capacity checks, metrics.
"
wr "$ROOT/src/domain/visibility.rs" "\
// Visibility lease management placeholder.
"
wr "$ROOT/src/domain/dlq.rs" "\
// DLQ envelope and reprocess semantics placeholder.
"
wr "$ROOT/src/domain/idempotency.rs" "\
// Idempotency triple checks placeholder (topic, idem_key, payload_hash).
"

wr "$ROOT/src/runtime/supervisor.rs" "\
// Supervisor task placeholder: spawn listener, workers, scanner, reprocessor; graceful shutdown.
"
wr "$ROOT/src/runtime/listener.rs" "\
// Listener placeholder: accept HTTP requests, route to shard work queues.
"
wr "$ROOT/src/runtime/worker.rs" "\
// Worker placeholder: per-shard dequeuer with visibility handling.
"
wr "$ROOT/src/runtime/scanner.rs" "\
// Scanner placeholder: requeues timed-out inflight messages, increments metrics.
"
wr "$ROOT/src/runtime/reprocessor.rs" "\
// DLQ reprocessor placeholder: operator-triggered flow with backoff.
"
wr "$ROOT/src/runtime/shutdown.rs" "\
// Graceful shutdown orchestration placeholder.
"

wr "$ROOT/src/bus/events.rs" "\
// Bus events placeholder: health/restart/broadcast signals.
"

wr "$ROOT/src/pq/posture.rs" "\
// PQ posture telemetry & readiness gating placeholder (PQ_ONLY, peer posture).
"

wr "$ROOT/src/util/backoff.rs" "\
// Jittered backoff helpers placeholder.
"
wr "$ROOT/src/util/time.rs" "\
// Duration/size parsing and monotonic clock helpers placeholder.
"

# -------------------------
# Tests
# -------------------------
wr "$ROOT/tests/golden_metrics.rs" "\
// Asserts expected metric names exist (placeholder).
#[test] fn golden_metrics() { assert!(true); }
"
wr "$ROOT/tests/http_surface.rs" "\
// HTTP surface tests placeholder: send/recv/ack/streaming.
#[test] fn http_surface() { assert!(true); }
"
wr "$ROOT/tests/visibility.rs" "\
// Visibility lease property tests placeholder.
#[test] fn visibility_props() { assert!(true); }
"
wr "$ROOT/tests/idempotency.rs" "\
// Idempotency triple tests placeholder.
#[test] fn idempotency_triple() { assert!(true); }
"
wr "$ROOT/tests/readiness.rs" "\
// Readiness shedding behavior tests placeholder.
#[test] fn readiness_sheds_writes_first() { assert!(true); }
"

# -------------------------
# GitHub Actions
# -------------------------
wr "$ROOT/.github/workflows/ci.yml" "\
name: ci
on: [push, pull_request]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy -p svc-mailbox2 -- -D warnings || true
      - run: cargo test -p svc-mailbox2 --all-features || true
"
wr "$ROOT/.github/workflows/render-mermaid.yml" "\
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in \$(git ls-files 'crates/svc-mailbox2/docs/diagrams/*.mmd'); do
            out=\"\${f%.mmd}.svg\"
            echo \"render: \$f -> \$out\"
            mmdc -i \"\$f\" -o \"\$out\"
          done
"

say "Done. Scaffold created at $ROOT"
