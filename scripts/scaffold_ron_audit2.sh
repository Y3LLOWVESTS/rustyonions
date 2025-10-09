#!/usr/bin/env bash
# Scaffolder for crates/ron-audit2 — structure only, no Rust logic.
# Usage: bash scripts/scaffold_ron_audit2.sh [--force]
set -euo pipefail

FORCE=0
if [[ "${1:-}" == "--force" ]]; then
  FORCE=1
fi

ROOT="crates/ron-audit2"

say() { printf '%s\n' "$*" >&2; }

mkd() {
  mkdir -p "$1"
}

# mkf <path>
# Writes stdin to <path> unless it exists and --force is not set.
mkf() {
  local path="$1"
  if [[ -e "$path" && "$FORCE" -ne 1 ]]; then
    say "skip: $path (exists)"
    # consume stdin to avoid broken pipes if caller provided heredoc
    cat >/dev/null || true
    return 0
  fi
  mkdir -p "$(dirname "$path")"
  cat > "$path"
  say "write: $path"
}

# mkt <path> — touch empty
mkt() {
  local path="$1"
  if [[ -e "$path" && "$FORCE" -ne 1 ]]; then
    say "skip: $path (exists)"
    return 0
  fi
  mkdir -p "$(dirname "$path")"
  : > "$path"
  say "touch: $path"
}

say "Scaffolding $ROOT ..."

# 1) Directories
while IFS= read -r d; do
  [[ -z "$d" ]] && continue
  mkd "$ROOT/$d"
done <<'DIRS'
.cargo
docs/api-history/ron-audit
src/canon
src/hash
src/verify
src/bounds
src/sink
src/stream
src/privacy
src/metrics
tests
fuzz/fuzz_targets
loom
benches
testing/perf
testing/chaos
testing/vectors
DIRS

# 2) Top-level files
mkf "$ROOT/README.md" <<'EOF'
# ron-audit2 (scaffold)
Purpose: Append-only audit evidence library (no endpoints, no threads).
This is a structure-only scaffold — no Rust logic is included by design.
EOF

mkf "$ROOT/CHANGELOG.md" <<'EOF'
# Changelog — ron-audit2
## [0.0.0] - scaffold
- Initial structure (no code).
EOF

mkf "$ROOT/LICENSE-APACHE" <<'EOF'
                                 Apache License
                           Version 2.0, January 2004
                 http://www.apache.org/licenses/
EOF

mkf "$ROOT/LICENSE-MIT" <<'EOF'
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software")...
EOF

mkf "$ROOT/CODEOWNERS" <<'EOF'
* @stevan-white
EOF

mkf "$ROOT/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["rustfmt", "clippy"]
EOF

mkf "$ROOT/deny.toml" <<'EOF'
# Keep aligned with workspace deny policy; local file allows per-crate notes.
[advisories]
yanked = "deny"

[bans]
multiple-versions = "deny"

[licenses]
allow = [
  "MIT",
  "Apache-2.0",
  "Unicode-DFS-2016",
  "Unicode-3.0",
  "CC0-1.0",
  "CDLA-Permissive-2.0",
  "OpenSSL",
]
EOF

mkf "$ROOT/.cargo/config.toml" <<'EOF'
[build]
rustflags = ["-Dwarnings"]

[target.'cfg(all())']
# Keep deterministic builds where possible.
rustflags = ["-C", "debuginfo=1"]
EOF

# 3) docs/ (skeletons; paste your finalized docs over these)
mkf "$ROOT/docs/IDB.md" <<'EOF'
# IDB — ron-audit2
Invariants, Principles, Proof. (Paste your completed doc here.)
EOF

mkf "$ROOT/docs/SECURITY.md" <<'EOF'
# SECURITY — ron-audit2
Threat model, boundaries, hardening checklist. (Paste completed content.)
EOF

mkf "$ROOT/docs/CONCURRENCY.md" <<'EOF'
# CONCURRENCY — ron-audit2
Lite-mode for small lib crates. (Paste completed content.)
EOF

mkf "$ROOT/docs/CONFIG.md" <<'EOF'
# CONFIG — ron-audit2
Host-supplied posture/env matrix. (Paste completed content.)
EOF

mkf "$ROOT/docs/INTEROP.md" <<'EOF'
# INTEROP — ron-audit2
DTOs, on-disk formats, test vectors, bus events. (Paste completed content.)
EOF

mkf "$ROOT/docs/OBSERVABILITY.md" <<'EOF'
# OBSERVABILITY — ron-audit2
Metrics, logs, readiness contracts. (Paste completed content.)
EOF

mkf "$ROOT/docs/PERFORMANCE.md" <<'EOF'
# PERFORMANCE — ron-audit2
Perf budgets, gates, reproducible benches. (Paste completed content.)
EOF

mkf "$ROOT/docs/GOVERNANCE.md" <<'EOF'
# GOVERNANCE — ron-audit2
Powers, redactions, appeals, SemVer policy. (Paste completed content.)
EOF

mkf "$ROOT/docs/QUANTUM.md" <<'EOF'
# QUANTUM — ron-audit2
PQ readiness plan & adapters. (Paste completed content.)
EOF

mkf "$ROOT/docs/RUNBOOK.md" <<'EOF'
# RUNBOOK — ron-audit2
Ops drills & SLOs. (Paste completed content.)
EOF

mkf "$ROOT/docs/api-history/ron-audit/v1.0.2.txt" <<'EOF'
# public-api snapshot placeholder for CI; replace with real outputs.
EOF

# 4) src/ (stubs only; no Rust logic)
mkf "$ROOT/src/lib.rs" <<'EOF'
//! ron-audit2 — structure-only scaffold (no logic here).
//! Public surface to be defined per finalized docs.

pub mod errors;
pub mod dto;
pub mod canon;
pub mod hash;
pub mod verify;
pub mod bounds;
pub mod sink;
pub mod stream;
pub mod privacy;
pub mod metrics;

// A small prelude is handy for hosts (left empty for now).
pub mod prelude {}
EOF

mkf "$ROOT/src/errors.rs" <<'EOF'
//! Typed error contracts (scaffold — no implementations).
//! Keep modules tiny and composable.
EOF

mkf "$ROOT/src/dto.rs" <<'EOF'
//! Canonical DTOs and serde shapes (scaffold).
//! Apply `deny_unknown_fields` in real definitions.
EOF

mkf "$ROOT/src/canon/mod.rs" <<'EOF'
//! Canonicalization rules module (scaffold).
pub mod rules;
pub mod vectors;
EOF

mkf "$ROOT/src/canon/rules.rs" <<'EOF'
//! Canon rules scaffold.
EOF

mkf "$ROOT/src/canon/vectors.rs" <<'EOF'
//! Frozen test vectors scaffold.
EOF

mkf "$ROOT/src/hash/mod.rs" <<'EOF'
//! Hash helpers (e.g., BLAKE3) — scaffold only.
EOF

mkf "$ROOT/src/verify/mod.rs" <<'EOF'
//! Record/chain verification — scaffold only.
pub mod record;
pub mod chain;
EOF

mkf "$ROOT/src/verify/record.rs" <<'EOF'
//! Record verification — scaffold.
EOF

mkf "$ROOT/src/verify/chain.rs" <<'EOF'
//! Chain verification — scaffold.
EOF

mkf "$ROOT/src/bounds/mod.rs" <<'EOF'
//! Centralized limits and checks — scaffold.
EOF

mkf "$ROOT/src/sink/mod.rs" <<'EOF'
//! Sinks (RAM/WAL/export) — scaffold only; traits and adapters live here.
pub mod traits;
pub mod ram;
pub mod wal;
pub mod export;
EOF

mkf "$ROOT/src/sink/traits.rs" <<'EOF'
//! AuditSink / AuditStream trait scaffolds.
EOF

mkf "$ROOT/src/sink/ram.rs" <<'EOF'
//! In-memory sink (Micronode profile) — scaffold.
EOF

mkf "$ROOT/src/sink/wal.rs" <<'EOF'
//! WAL-backed sink (feature-gated in real crate) — scaffold.
EOF

mkf "$ROOT/src/sink/export.rs" <<'EOF'
//! Deterministic export/checkpoint helpers — scaffold.
EOF

mkf "$ROOT/src/stream/mod.rs" <<'EOF'
//! Stream ordering helpers — scaffold.
EOF

mkf "$ROOT/src/privacy/mod.rs" <<'EOF'
//! PII/IP handling policy helpers — scaffold.
EOF

mkf "$ROOT/src/metrics/mod.rs" <<'EOF'
//! Metric name/constants (no registry ownership) — scaffold.
EOF

# 5) tests/ (acceptance test shells)
mkt "$ROOT/tests/append_only.rs"
mkt "$ROOT/tests/canonicalization.rs"
mkt "$ROOT/tests/bounds.rs"
mkt "$ROOT/tests/idempotency.rs"
mkt "$ROOT/tests/multi_writer_ordering.rs"
mkt "$ROOT/tests/privacy_policies.rs"
mkt "$ROOT/tests/export_checkpoints.rs"
mkt "$ROOT/tests/api_compat.rs"

# 6) fuzz/ (cargo-fuzz skeleton)
mkf "$ROOT/fuzz/Cargo.toml" <<'EOF'
[package]
name = "ron-audit2-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[workspace]

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.ron-audit2]
path = ".."

[profile.release]
debug = 1

[[bin]]
name = "fuzz_record_roundtrip"
path = "fuzz_targets/fuzz_record_roundtrip.rs"

[[bin]]
name = "fuzz_canon_vectors"
path = "fuzz_targets/fuzz_canon_vectors.rs"
EOF

mkt "$ROOT/fuzz/fuzz_targets/fuzz_record_roundtrip.rs"
mkt "$ROOT/fuzz/fuzz_targets/fuzz_canon_vectors.rs"

# 7) loom/ (model shell)
mkt "$ROOT/loom/chain_loom.rs"

# 8) benches/ (bench shells)
mkt "$ROOT/benches/hash_b3.rs"
mkt "$ROOT/benches/verify_chain.rs"
mkt "$ROOT/benches/wal_batching.rs"

# 9) testing/ plans and vectors
mkf "$ROOT/testing/perf/perf_plan.md" <<'EOF'
# Performance Plan (scaffold)
- Targets, p95 budgets, datasets, repro steps.
EOF

mkf "$ROOT/testing/chaos/chaos_plan.md" <<'EOF'
# Chaos Plan (scaffold)
- Disk-full, storm, bitrot, recovery criteria.
EOF

mkt "$ROOT/testing/vectors/record_small.json"
mkt "$ROOT/testing/vectors/record_max.json"
mkt "$ROOT/testing/vectors/manifest_example.json"

# 10) Cargo.toml (scaffold — no deps declared here)
mkf "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "ron-audit2"
version = "0.0.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Append-only audit evidence (library only) — scaffold"
repository = "https://example.com/RustyOnions"

[lib]
path = "src/lib.rs"

[features]
# In real crate, features like "wal", "export", "pq-sign" would be additive.
default = []
EOF

say "Done. Scaffold created at $ROOT"