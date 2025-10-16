#!/usr/bin/env bash
# Scaffold for crates/ron-accounting2 (no code — files + placeholders only)

set -euo pipefail

CRATE_DIR="crates/ron-accounting2"
FORCE="${FORCE:-0}"

say() { printf "%s\n" "$*"; }
ok()  { printf "write: %s\n" "$*"; }
skp() { printf "skip : %s (exists, non-empty)\n" "$*"; }

mkd() {
  mkdir -p "$1"
  printf "dir  : %s\n" "$1"
}

write_file() {
  local path="$1"; shift
  local content="$*"
  if [[ -f "$path" && -s "$path" && "$FORCE" != "1" ]]; then
    skp "$path"
    return 0
  fi
  printf "%b" "$content" > "$path"
  ok "$path"
}

# 1) Directories
say "Scaffolding ron-accounting2…"
mkd "$CRATE_DIR"
mkd "$CRATE_DIR/.cargo"
mkd "$CRATE_DIR/src"
mkd "$CRATE_DIR/src/utils"
mkd "$CRATE_DIR/src/accounting"
mkd "$CRATE_DIR/src/exporter"
mkd "$CRATE_DIR/src/wal"
mkd "$CRATE_DIR/src/config"
mkd "$CRATE_DIR/tests/unit"
mkd "$CRATE_DIR/tests/prop"
mkd "$CRATE_DIR/tests/loom"
mkd "$CRATE_DIR/tests/vectors/ron-accounting"
mkd "$CRATE_DIR/benches"
mkd "$CRATE_DIR/examples"
mkd "$CRATE_DIR/docs/diagrams"
mkd "$CRATE_DIR/ops/alerts"
mkd "$CRATE_DIR/.github/workflows"
mkd "scripts"

# 2) Root files
write_file "$CRATE_DIR/.gitignore" "\
target/
**/*.swp
.DS_Store
"

write_file "$CRATE_DIR/LICENSE-MIT" "\
MIT License

Copyright (c) $(date +%Y)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction…
"

write_file "$CRATE_DIR/LICENSE-APACHE" "\
                                 Apache License
                           Version 2.0, January 2004
http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION…
"

write_file "$CRATE_DIR/CHANGELOG.md" "\
# Changelog — ron-accounting2

All notable changes to this crate will be documented in this file.

## [0.1.0] - UNRELEASED
- Initial scaffold (no code).
"

write_file "$CRATE_DIR/README.md" "\
# ron-accounting2

> Scaffold placeholder. Replace with the finalized README once ready.

This crate mirrors the **ron-accounting** library structure: in-memory metering, time-sliced snapshots, export to ledger, amnesia-aware.  
This file is a placeholder so CI has something to render.
"

# 3) Cargo + tool configs
write_file "$CRATE_DIR/Cargo.toml" "\
[package]
name = \"ron-accounting2\"
version = \"0.1.0\"
edition = \"2021\"
license = \"MIT OR Apache-2.0\"
rust-version = \"1.80\"
description = \"In-memory usage metering (transient counters/snapshots) — scaffold only\"
repository = \"\"

[lib]
path = \"src/lib.rs\"

[features]
default = [\"serde\", \"parking_lot\"]
export-serde = []
ledger-wire = []
wal = []

[dependencies]
# Filled by workspace in real crate; left empty in scaffold.

[dev-dependencies]
"

write_file "$CRATE_DIR/.cargo/config.toml" "\
[build]
rustflags = []

[target.'cfg(loom)']
rustflags = [\"--cfg\", \"loom\"]
"

# 4) src: top-level modules (placeholders only)
write_file "$CRATE_DIR/src/lib.rs" "\
/*!
ron-accounting2 — library scaffold only (no implementation).

Modules:
- errors.rs — error taxonomy
- metrics.rs — metric names/registration
- readiness.rs — readiness keys
- normalize.rs — label normalization contract
- utils/ — small helpers
- accounting/ — labels, recorder, window, slice, rollover
- exporter/ — trait, router, lane, worker, ack_lru
- wal/ — feature-gated persistence (auto-off under Amnesia)
- config/ — schema + validate + load

Replace these placeholders with real code when ready.
*/

pub mod errors;
pub mod metrics;
pub mod readiness;
pub mod normalize;
pub mod utils;
pub mod accounting;
pub mod exporter;
#[cfg(feature = \"wal\")]
pub mod wal;
pub mod config;
"

write_file "$CRATE_DIR/src/errors.rs" "\
// Placeholder: error taxonomy (#[non_exhaustive]) goes here.
"

write_file "$CRATE_DIR/src/metrics.rs" "\
// Placeholder: Prometheus metric keys + registration live here.
"

write_file "$CRATE_DIR/src/readiness.rs" "\
// Placeholder: readiness keys and snapshot struct live here.
"

write_file "$CRATE_DIR/src/normalize.rs" "\
// Placeholder: label normalization and templating helpers (no PII).
"

write_file "$CRATE_DIR/src/utils/time.rs" "\
// Placeholder: UTC boundary math, window tick helpers.
"

write_file "$CRATE_DIR/src/utils/hashing.rs" "\
// Placeholder: BLAKE3 helpers and digest chaining.
"

write_file "$CRATE_DIR/src/utils/encode.rs" "\
// Placeholder: canonical encode helpers (DAG-CBOR/MsgPack caps).
"

# 5) src/accounting
write_file "$CRATE_DIR/src/accounting/mod.rs" "\
pub mod labels;
pub mod dimensions;
pub mod recorder;
pub mod window;
pub mod slice;
pub mod rollover;

// Placeholder exports: types & façade would be re-exported from here.
"

write_file "$CRATE_DIR/src/accounting/labels.rs" "\
// Placeholder: label builder (tenant, method, route, service, region).
"

write_file "$CRATE_DIR/src/accounting/dimensions.rs" "\
// Placeholder: dimensions like requests, bytes_in, bytes_out.
"

write_file "$CRATE_DIR/src/accounting/recorder.rs" "\
// Placeholder: sharded atomic counters; no awaits on hot path.
"

write_file "$CRATE_DIR/src/accounting/window.rs" "\
// Placeholder: time-sliced ring buffer and rollover policy.
"

write_file "$CRATE_DIR/src/accounting/slice.rs" "\
// Placeholder: SealedSlice metadata, versioning, digest.
"

write_file "$CRATE_DIR/src/accounting/rollover.rs" "\
// Placeholder: boundary detection and sealing.
"

# 6) src/exporter
write_file "$CRATE_DIR/src/exporter/mod.rs" "\
pub mod trait_;
pub mod router;
pub mod lane;
pub mod worker;
pub mod ack_lru;

// Rename 'trait_' to 'trait' in code; 'trait.rs' conflicts with keyword in some tooling.
"

write_file "$CRATE_DIR/src/exporter/trait.rs" "\
// Placeholder: trait Exporter + Ack enum contracts.
"

# keep a copy named trait_.rs for editors that dislike 'trait.rs'
write_file "$CRATE_DIR/src/exporter/trait_.rs" "\
// Duplicate placeholder to avoid editor tooling conflicts.
"

write_file "$CRATE_DIR/src/exporter/router.rs" "\
// Placeholder: fanout from pending slices to ordered lanes; fairness + bounds.
"

write_file "$CRATE_DIR/src/exporter/lane.rs" "\
// Placeholder: per-stream bounded queue with ordering guard.
"

write_file "$CRATE_DIR/src/exporter/worker.rs" "\
// Placeholder: single-writer exporter with jittered backoff and latency metrics.
"

write_file "$CRATE_DIR/src/exporter/ack_lru.rs" "\
/* Placeholder: small LRU for (slice_id,digest) idempotency */
"

# 7) src/wal (feature)
write_file "$CRATE_DIR/src/wal/mod.rs" "\
// Placeholder: WAL handle (feature = \"wal\"); auto-off under Amnesia.
"

write_file "$CRATE_DIR/src/wal/segment.rs" "\
// Placeholder: length-delimited, checksummed segments; fsync policy.
"

write_file "$CRATE_DIR/src/wal/replay.rs" "\
// Placeholder: startup scan + checksum verify + re-enqueue.
"

write_file "$CRATE_DIR/src/wal/fs.rs" "\
// Placeholder: bounded spawn_blocking file ops with deadlines.
"

# 8) src/config
write_file "$CRATE_DIR/src/config/mod.rs" "\
pub mod schema;
pub mod validate;
pub mod load;

// Placeholder: public Config surface re-exported from here.
"

write_file "$CRATE_DIR/src/config/schema.rs" "\
// Placeholder: serde structs for window/slices/limits/export/wal.
"

write_file "$CRATE_DIR/src/config/validate.rs" "\
// Placeholder: fail-closed rules; amnesia implies WAL disabled.
"

write_file "$CRATE_DIR/src/config/load.rs" "\
// Placeholder: env → file → defaults precedence; effective-config log.
"

# 9) tests (unit/prop/loom) — placeholders
write_file "$CRATE_DIR/tests/unit/recording_tests.rs" "\
// Placeholder tests: monotone increments, saturating adds.
"

write_file "$CRATE_DIR/tests/unit/rollover_tests.rs" "\
// Placeholder tests: single rollover per boundary; skew tolerance.
"

write_file "$CRATE_DIR/tests/unit/exporter_ordering_tests.rs" "\
// Placeholder tests: no N+1 before N; shed on overflow.
"

write_file "$CRATE_DIR/tests/unit/wal_tests.rs" "\
// Placeholder tests: replay; checksum skip; quotas (feature = \"wal\").
"

write_file "$CRATE_DIR/tests/prop/encoding_prop.rs" "\
// Placeholder proptests: snapshot round-trip (serde); deny unknown fields.
"

write_file "$CRATE_DIR/tests/prop/labels_prop.rs" "\
// Placeholder proptests: normalizer idempotence; UTF-8 safety; no PII.
"

write_file "$CRATE_DIR/tests/loom/router_model.rs" "\
// Placeholder loom model: router fairness and no deadlock.
"

write_file "$CRATE_DIR/tests/loom/shutdown_model.rs" "\
// Placeholder loom model: clean shutdown with no lock across await.
"

# 10) benches/examples
write_file "$CRATE_DIR/benches/record.rs" "\
// Placeholder bench: increment hot-path micro-benchmark.
"

write_file "$CRATE_DIR/benches/seal.rs" "\
// Placeholder bench: snapshot seal + encode micro-benchmark.
"

write_file "$CRATE_DIR/examples/minimal.rs" "\
// Placeholder example: construct labels, record, snapshot, print count.
fn main() {}
"

write_file "$CRATE_DIR/examples/export_to_mock.rs" "\
// Placeholder example: show how to plug a mock exporter.
fn main() {}
"

# 11) test vectors
write_file "$CRATE_DIR/tests/vectors/ron-accounting/sliceV1_dagcbor.json" "{}\n"
write_file "$CRATE_DIR/tests/vectors/ron-accounting/sliceV1_dagcbor.bin" ""
write_file "$CRATE_DIR/tests/vectors/ron-accounting/sliceV1_msgpack.bin" ""
write_file "$CRATE_DIR/tests/vectors/ron-accounting/sliceV1_digest.txt" ""
write_file "$CRATE_DIR/tests/vectors/ron-accounting/oap1_rollover_evt.bin" ""

# 12) docs (placeholders) — you can replace with your completed docs
write_file "$CRATE_DIR/docs/API.MD" "# API — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/CONFIG.MD" "# CONFIG — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/CONCURRENCY.MD" "# CONCURRENCY — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/GOVERNANCE.MD" "# GOVERNANCE — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/IDB.md" "# IDB — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/INTEROP.MD" "# INTEROP — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/OBSERVABILITY.MD" "# OBS — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/OLD_README.md" "# OLD_README — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/PERFORMANCE.MD" "# PERFORMANCE — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/QUANTUM.MD" "# QUANTUM — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/RUNBOOK.MD" "# RUNBOOK — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/SECURITY.MD" "# SECURITY — ron-accounting2 (scaffold)\n"
write_file "$CRATE_DIR/docs/TESTS.MD" "# TESTS — ron-accounting2 (scaffold)\n"

write_file "$CRATE_DIR/docs/diagrams/arch.mmd" "%% Mermaid arch placeholder\n"
write_file "$CRATE_DIR/docs/diagrams/sequence.mmd" "%% Mermaid sequence placeholder\n"
write_file "$CRATE_DIR/docs/diagrams/state.mmd" "%% Mermaid state placeholder\n"

# 13) ops + CI stubs
write_file "$CRATE_DIR/ops/alerts/ron-accounting.yaml" "\
# Placeholder: PromQL alerts (export backlog, evictions, latency).
"

write_file "$CRATE_DIR/.github/workflows/clippy.yml" "\
name: clippy
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo clippy -p ron-accounting2 -- -D warnings
"

write_file "$CRATE_DIR/.github/workflows/render-mermaid.yml" "\
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in \$(git ls-files '*.mmd'); do
            out=\"\${f%.mmd}.svg\"
            mmdc -i \"\$f\" -o \"\$out\"
          done
"

write_file "$CRATE_DIR/.github/workflows/public-api.yml" "\
name: public-api
on: [pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-public-api
      - run: cargo public-api -p ron-accounting2 || true
"

write_file "$CRATE_DIR/.github/workflows/coverage.yml" "\
name: coverage
on: [pull_request]
jobs:
  cov:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-tarpaulin
      - run: cargo tarpaulin -p ron-accounting2 --out Xml || true
"

write_file "$CRATE_DIR/.github/workflows/perf-gate.yml" "\
name: perf-gate
on: [push, pull_request]
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench -p ron-accounting2 || true
"

# 14) helper scripts
write_file "scripts/perf_gate.py" "\
#!/usr/bin/env python3
# Placeholder: compare latest Criterion results vs baseline; fail on >10% regression.
"

write_file "scripts/generate_vectors.rs" "\
fn main() {
    // Placeholder: regenerate canonical vectors for interop tests.
}
"

say "Done. Scaffold created at $CRATE_DIR"
