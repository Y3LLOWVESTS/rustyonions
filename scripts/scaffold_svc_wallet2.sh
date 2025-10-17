#!/usr/bin/env bash
# scaffold_svc_wallet2.sh — create svc-wallet2 scaffold (dirs + files)
# Usage:
#   bash scripts/scaffold_svc_wallet2.sh
#   bash scripts/scaffold_svc_wallet2.sh --force   # overwrite existing files

set -euo pipefail

FORCE=0
if [[ "${1:-}" == "--force" ]]; then
  FORCE=1
fi

CRATE_DIR="crates/svc-wallet2"

# Write helper: writes stdin to file; skips if exists unless --force
write() {
  local path="$1"
  local dir
  dir="$(dirname "$path")"
  mkdir -p "$dir"
  if [[ -e "$path" && "$FORCE" -eq 0 ]]; then
    echo "skip (exists): $path"
    return 0
  fi
  # shellcheck disable=SC2094
  cat > "$path"
  echo "write: $path"
}

echo "scaffolding into: $CRATE_DIR"

# ---------- Root files ----------
write "$CRATE_DIR/.gitignore" <<'EOF'
target
**/*.svg
.DS_Store
Cargo.lock
coverage/
EOF

write "$CRATE_DIR/LICENSE-MIT" <<'EOF'
MIT License

Copyright (c) 2025

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

(See LICENSE-APACHE for alternate terms.)
EOF

write "$CRATE_DIR/LICENSE-APACHE" <<'EOF'
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/
(standard Apache-2.0 text)
EOF

write "$CRATE_DIR/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
EOF

write "$CRATE_DIR/Cargo.toml" <<'EOF'
[package]
name = "svc-wallet2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[package.metadata.docs]
msrv = "1.80.0"

[features]
default = ["tls"]
tls = []
arti = []         # optional Tor transport feature
legacy_pay = []   # guarded legacy behavior (off by default)

[dependencies]
anyhow = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt-multi-thread","macros","time","io-util"] }
axum = { version = "0.7", features = ["tokio","http1","http2","json"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace","limit","timeout","compression-gzip"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
prometheus = "0.14"
tokio-rustls = "0.26"
http = "1"
hyper = { version = "1", features = ["http1","server"] }
blake3 = "1"

[dev-dependencies]
criterion = "0.5"
tokio = { version = "1", features = ["rt-multi-thread","macros","time","io-util","test-util"] }
EOF

write "$CRATE_DIR/CHANGELOG.md" <<'EOF'
# Changelog — svc-wallet2

## [0.1.0] - 2025-10-16
- Initial scaffold (service; HTTP is the public contract).
EOF

write "$CRATE_DIR/README.md" <<'EOF'
# svc-wallet2

> Scaffolded service crate; HTTP is the public API. See docs/ for canon templates and acceptance gates.

- Use docs/openapi/svc-wallet.yaml for contract.
- See docs/IDB.md, SECURITY.md, INTEROP.md, PERFORMANCE.md, QUANTUM.md, TESTS.md, RUNBOOK.md, GOVERNANCE.md.
EOF

# ---------- .cargo ----------
write "$CRATE_DIR/.cargo/config.toml" <<'EOF'
[build]
rustflags = []

[alias]
ci = "fmt --all && clippy -- -D warnings && test -p svc-wallet2"
EOF

# ---------- configs ----------
write "$CRATE_DIR/configs/svc-wallet.example.toml" <<'EOF'
# Example runtime configuration for svc-wallet2
bind_addr = "0.0.0.0:8080"
metrics_addr = "127.0.0.1:0"
ledger_url = "http://127.0.0.1:8081"
auth_url = "http://127.0.0.1:8082"
policy_url = "http://127.0.0.1:8083"
amnesia = false
pq_mode = "off"          # off | hybrid | pq_only
policy_reload_grace_ms = 500
EOF

write "$CRATE_DIR/configs/policies.sample.json" <<'EOF'
{
  "version": 1,
  "rules": [
    { "account": "*", "asset": "USD", "actions": ["issue","transfer","burn"],
      "ceilings": {"single": "100000", "daily": "1000000"}, "ttl": "1h" }
  ]
}
EOF

# ---------- docs & diagrams ----------
write "$CRATE_DIR/docs/ALL_DOCS.md" <<'EOF'
<!-- Place the combined crate docs here (IDB, API, CONFIG, CONCURRENCY, SECURITY, OBSERVABILITY, PERFORMANCE, QUANTUM, RUNBOOK, TESTS, GOVERNANCE, INTEROP). -->
EOF

write "$CRATE_DIR/docs/openapi/svc-wallet.yaml" <<'EOF'
openapi: 3.0.3
info:
  title: svc-wallet HTTP API
  version: 1.0.0
servers:
  - url: http://localhost:8080
paths:
  /healthz:
    get:
      summary: Liveness
      responses: { '200': { description: OK } }
  /readyz:
    get:
      summary: Readiness
      responses: { '200': { description: Ready } }
  /metrics:
    get:
      summary: Prometheus metrics
      responses: { '200': { description: Text exposition } }
  /v1/balance:
    get:
      summary: Get balance
      parameters:
        - in: query
          name: account
          required: true
          schema: { type: string }
        - in: query
          name: asset
          required: true
          schema: { type: string }
      responses:
        '200': { description: Balance response }
  /v1/issue:
    post:
      summary: Issue funds (Idempotency-Key required)
      responses: { '200': { description: Receipt } }
  /v1/transfer:
    post:
      summary: Transfer funds (Idempotency-Key required)
      responses: { '200': { description: Receipt } }
  /v1/burn:
    post:
      summary: Burn funds (Idempotency-Key required)
      responses:
        '200': { description: Receipt }
  /v1/tx/{txid}:
    get:
      summary: Get transaction receipt
      parameters:
        - in: path
          name: txid
          required: true
          schema: { type: string }
      responses:
        '200': { description: Receipt }
EOF

write "$CRATE_DIR/docs/arch.mmd" <<'EOF'
flowchart LR
  subgraph Client/Node
    A[Gateway/SDK] -->|HTTP+TLS| B(svc-wallet2)
  end
  B -->|cap verify| C[svc-passport/ron-auth]
  B -->|append commit| D[ron-ledger]
  B -->|counters| F[ron-accounting]
  B -->|Metrics| E[[Prometheus]]
  style B fill:#0b7285,stroke:#083344,color:#fff
EOF

write "$CRATE_DIR/docs/sequence.mmd" <<'EOF'
sequenceDiagram
  actor Client
  participant G as svc-gateway
  participant W as svc-wallet2
  participant L as ron-ledger
  Client->>G: POST /v1/transfer (Idempotency-Key)
  G->>W: forward (cap)
  W->>L: append commit
  L-->>W: receipt(txid, nonce)
  W-->>G: 200 + deterministic receipt
  G-->>Client: 200 + receipt
EOF

write "$CRATE_DIR/docs/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: start()
  Running --> Degraded: upstream stall (ledger/policy)
  Degraded --> Running: recover
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
EOF

# ---------- scripts ----------
write "$CRATE_DIR/scripts/dev-run.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=${RUST_LOG:-info}
export SVC_WALLET_BIND_ADDR=${SVC_WALLET_BIND_ADDR:-0.0.0.0:8080}
export SVC_WALLET_METRICS_ADDR=${SVC_WALLET_METRICS_ADDR:-127.0.0.1:0}
cargo run -p svc-wallet2
EOF
chmod +x "$CRATE_DIR/scripts/dev-run.sh"

write "$CRATE_DIR/scripts/chaos-ledger-stall.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Example chaos drill placeholder:
# ronctl chaos --target svc-wallet2 --inject ledger_stall --duration 30s --kpi "readyz_writes_shed<=500ms,error_rate<1%"
echo "Simulated: ledger stall injection (placeholder)"
EOF
chmod +x "$CRATE_DIR/scripts/chaos-ledger-stall.sh"

write "$CRATE_DIR/scripts/reload-policy.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
PID=$(pgrep -f svc-wallet2 || true)
if [[ -n "$PID" ]]; then
  kill -HUP "$PID"
  echo "Sent SIGHUP to svc-wallet2 ($PID)"
else
  echo "svc-wallet2 not running"
fi
EOF
chmod +x "$CRATE_DIR/scripts/reload-policy.sh"

# ---------- GitHub Actions ----------
write "$CRATE_DIR/.github/workflows/ci.yml" <<'EOF'
name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
      - run: cargo fmt --all --check
      - run: cargo clippy -p svc-wallet2 -- -D warnings
      - run: cargo test -p svc-wallet2
EOF

write "$CRATE_DIR/.github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files 'crates/svc-wallet2/docs/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
EOF

write "$CRATE_DIR/.github/workflows/fuzz.yml" <<'EOF'
name: fuzz
on:
  workflow_dispatch:
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-fuzz
      - run: echo "placeholder: integrate fuzz targets in /fuzz"
EOF

write "$CRATE_DIR/.github/workflows/loom.yml" <<'EOF'
name: loom
on:
  workflow_dispatch:
jobs:
  loom:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: RUSTFLAGS="--cfg loom" cargo test -p svc-wallet2 --test loom_invariants -- --nocapture || true
EOF

# ---------- benches ----------
write "$CRATE_DIR/benches/balance_read.rs" <<'EOF'
use criterion::{criterion_group, criterion_main, Criterion};
fn bench_balance(c: &mut Criterion) { c.bench_function("balance_read", |b| b.iter(|| 1)); }
criterion_group!(benches, bench_balance);
criterion_main!(benches);
EOF

write "$CRATE_DIR/benches/transfer_commit.rs" <<'EOF'
use criterion::{criterion_group, criterion_main, Criterion};
fn bench_transfer(c: &mut Criterion) { c.bench_function("transfer_commit", |b| b.iter(|| 1)); }
criterion_group!(benches, bench_transfer);
criterion_main!(benches);
EOF

# ---------- fuzz skeleton ----------
write "$CRATE_DIR/fuzz/fuzz_targets/dto_spend_parser.rs" <<'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = std::str::from_utf8(data);
});
EOF

write "$CRATE_DIR/fuzz/fuzz_targets/spend_request_roundtrip.rs" <<'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data.len();
});
EOF

write "$CRATE_DIR/fuzz/fuzz_targets/sequence_machine.rs" <<'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data.iter().fold(0u64, |acc, b| acc.wrapping_add(*b as u64));
});
EOF

# ---------- testing/load ----------
write "$CRATE_DIR/testing/load/k6-transfer.js" <<'EOF'
import http from 'k6/http';
import { sleep } from 'k6';
export const options = { vus: 10, duration: '10s' };
export default function () {
  http.post('http://127.0.0.1:8080/v1/transfer', JSON.stringify({}), {
    headers: { 'Content-Type': 'application/json', 'Idempotency-Key': 'demo' }
  });
  sleep(0.1);
}
EOF

write "$CRATE_DIR/testing/load/vegeta-balance.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "GET http://127.0.0.1:8080/v1/balance?account=alice&asset=USD" | vegeta attack -duration=10s | vegeta report
EOF
chmod +x "$CRATE_DIR/testing/load/vegeta-balance.sh"

# ---------- src skeleton ----------
write "$CRATE_DIR/src/main.rs" <<'EOF'
fn main() {
    // svc-wallet2 entrypoint (service binary).
    // HTTP is the public contract; keep Rust surface private.
    println!("svc-wallet2 scaffold");
}
EOF

write "$CRATE_DIR/src/lib.rs" <<'EOF'
#![forbid(unsafe_code)]
// Internal-only crate surface. HTTP is the public API.
EOF

write "$CRATE_DIR/src/supervisor.rs" <<'EOF'
#![allow(unused)]
// Supervisory wiring: child tasks, shutdown, readiness snapshots (stub).
EOF

write "$CRATE_DIR/src/readiness.rs" <<'EOF'
#![allow(unused)]
// Readiness logic; degrade-first on upstream stalls (stub).
EOF

write "$CRATE_DIR/src/config.rs" <<'EOF'
#![allow(unused)]
// Env + flag parsing; SIGHUP reload grace window (stub).
EOF

write "$CRATE_DIR/src/errors.rs" <<'EOF'
#![allow(unused)]
// Error taxonomy → HTTP mapping (stub).
EOF

write "$CRATE_DIR/src/metrics.rs" <<'EOF'
#![allow(unused)]
// Prometheus metrics registry + canonical metrics (stub).
EOF

# middleware
write "$CRATE_DIR/src/middleware/mod.rs" <<'EOF'
pub mod request_id;
pub mod tracing_log;
pub mod limits;
pub mod decompress_cap;
pub mod timeouts;
pub mod rate_limit;
pub mod shedder;
EOF

write "$CRATE_DIR/src/middleware/request_id.rs" <<'EOF'
// Correlation ID propagation (stub).
EOF
write "$CRATE_DIR/src/middleware/tracing_log.rs" <<'EOF'
// JSON tracing middleware (stub).
EOF
write "$CRATE_DIR/src/middleware/limits.rs" <<'EOF'
// Body size limits (<= 1 MiB) (stub).
EOF
write "$CRATE_DIR/src/middleware/decompress_cap.rs" <<'EOF'
// Decompression ratio cap (<= 10x) (stub).
EOF
write "$CRATE_DIR/src/middleware/timeouts.rs" <<'EOF'
// Read/Write/Idle timeouts (stub).
EOF
write "$CRATE_DIR/src/middleware/rate_limit.rs" <<'EOF'
// Local rate limiting; Retry-After headers (stub).
EOF
write "$CRATE_DIR/src/middleware/shedder.rs" <<'EOF'
// Readiness-aware write shedding (stub).
EOF

# routes
write "$CRATE_DIR/src/routes/mod.rs" <<'EOF'
pub mod health;
pub mod metrics;
pub mod v1;
EOF

write "$CRATE_DIR/src/routes/health.rs" <<'EOF'
// /healthz endpoint (stub).
EOF

write "$CRATE_DIR/src/routes/metrics.rs" <<'EOF'
// /metrics endpoint (stub).
EOF

write "$CRATE_DIR/src/routes/v1/mod.rs" <<'EOF'
pub mod balance;
pub mod issue;
pub mod transfer;
pub mod burn;
pub mod receipt;
EOF

write "$CRATE_DIR/src/routes/v1/balance.rs" <<'EOF'
// GET /v1/balance (stub).
EOF
write "$CRATE_DIR/src/routes/v1/issue.rs" <<'EOF'
// POST /v1/issue (stub).
EOF
write "$CRATE_DIR/src/routes/v1/transfer.rs" <<'EOF'
// POST /v1/transfer (stub).
EOF
write "$CRATE_DIR/src/routes/v1/burn.rs" <<'EOF'
// POST /v1/burn (stub).
EOF
write "$CRATE_DIR/src/routes/v1/receipt.rs" <<'EOF'
// GET /v1/tx/{txid} (stub).
EOF

# dto
write "$CRATE_DIR/src/dto/mod.rs" <<'EOF'
pub mod requests;
pub mod responses;
pub mod errors;
EOF

write "$CRATE_DIR/src/dto/requests.rs" <<'EOF'
// Request DTOs (serde deny_unknown_fields) (stub).
EOF
write "$CRATE_DIR/src/dto/responses.rs" <<'EOF'
// Response DTOs (stub).
EOF
write "$CRATE_DIR/src/dto/errors.rs" <<'EOF'
// Wire error DTO (stub).
EOF

# auth/policy/ledger/accounting
write "$CRATE_DIR/src/auth/mod.rs" <<'EOF'
pub mod caps;
EOF
write "$CRATE_DIR/src/auth/caps.rs" <<'EOF'
// Capability verification (stub).
EOF
write "$CRATE_DIR/src/policy/mod.rs" <<'EOF'
pub mod enforce;
EOF
write "$CRATE_DIR/src/policy/enforce.rs" <<'EOF'
// Policy admission control and reload (stub).
EOF
write "$CRATE_DIR/src/ledger/mod.rs" <<'EOF'
pub mod client;
pub mod types;
EOF
write "$CRATE_DIR/src/ledger/client.rs" <<'EOF'
// ron-ledger client (stub).
EOF
write "$CRATE_DIR/src/ledger/types.rs" <<'EOF'
// Commit/receipt wire types (stub).
EOF
write "$CRATE_DIR/src/accounting/mod.rs" <<'EOF'
pub mod client;
EOF
write "$CRATE_DIR/src/accounting/client.rs" <<'EOF'
// ron-accounting client (stub).
EOF

# seq/idem/cache/util
write "$CRATE_DIR/src/seq/mod.rs" <<'EOF'
pub mod nonce;
EOF
write "$CRATE_DIR/src/seq/nonce.rs" <<'EOF'
// Per-account nonces (stub).
EOF
write "$CRATE_DIR/src/idem/mod.rs" <<'EOF'
pub mod store;
EOF
write "$CRATE_DIR/src/idem/store.rs" <<'EOF'
// Idempotency-Key store (stub).
EOF
write "$CRATE_DIR/src/cache/mod.rs" <<'EOF'
pub mod balance_cache;
pub mod invalidator;
EOF
write "$CRATE_DIR/src/cache/balance_cache.rs" <<'EOF'
// Balance snapshot cache (stub).
EOF
write "$CRATE_DIR/src/cache/invalidator.rs" <<'EOF'
// Cache invalidation hooks (stub).
EOF
write "$CRATE_DIR/src/util/blake3_receipt.rs" <<'EOF'
// Short receipt hashing helper (stub).
EOF
write "$CRATE_DIR/src/util/headers.rs" <<'EOF'
// Header parsing helpers (stub).
EOF
write "$CRATE_DIR/src/util/parsing.rs" <<'EOF'
// Safe parsing with size/ratio guards (stub).
EOF

# ---------- tests ----------
write "$CRATE_DIR/tests/harness.rs" <<'EOF'
// Spins an in-process server with example config (stub).
EOF

write "$CRATE_DIR/tests/api_contract.rs" <<'EOF'
// Validates OpenAPI parity for required paths/status codes (stub).
EOF

write "$CRATE_DIR/tests/i_1_doublespend.rs" <<'EOF'
// Invariant: no doublespends (stub).
EOF
write "$CRATE_DIR/tests/i_2_nonnegativity.rs" <<'EOF'
// Invariant: non-negativity (stub).
EOF
write "$CRATE_DIR/tests/i_3_conservation.rs" <<'EOF'
// Invariant: conservation (stub).
EOF
write "$CRATE_DIR/tests/i_4_caps_policy.rs" <<'EOF'
// Invariant: capability/policy gates (stub).
EOF
write "$CRATE_DIR/tests/i_5_dto_hygiene.rs" <<'EOF'
// Invariant: DTO hygiene (stub).
EOF
write "$CRATE_DIR/tests/i_7_ledger_primacy.rs" <<'EOF'
// Invariant: ledger is source of truth (stub).
EOF
write "$CRATE_DIR/tests/i_8_observability.rs" <<'EOF'
// Invariant: metrics/tracing present (stub).
EOF
write "$CRATE_DIR/tests/i_9_amnesia.rs" <<'EOF'
// Invariant: amnesia mode has no persistence (stub).
EOF
write "$CRATE_DIR/tests/i_11_transport_bounds.rs" <<'EOF'
// Invariant: body<=1MiB; decompress<=10x; timeouts (stub).
EOF
write "$CRATE_DIR/tests/i_12_overflow_ceilings.rs" <<'EOF'
// Invariant: ceilings overflow guarded (stub).
EOF
write "$CRATE_DIR/tests/readiness_and_shedding.rs" <<'EOF'
// Readiness sheds writes first on upstream stalls (stub).
EOF
write "$CRATE_DIR/tests/vectors_receipt_hash.rs" <<'EOF'
// Golden vectors for receipt determinism (stub).
EOF
write "$CRATE_DIR/tests/loom_invariants.rs" <<'EOF'
// Loom: sequencing/idempotency races (stub).
EOF

echo "done."
