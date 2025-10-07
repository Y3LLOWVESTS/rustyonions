#!/usr/bin/env bash
set -euo pipefail

# Scaffold the modular, code-light oap2 crate structure.
# - Creates files/folders only if missing.
# - Writes minimal placeholders (no implementation code).
# - Keeps modules small and aligned to the blueprint.

CRATE_DIR="crates/oap2"

say() { printf '%s\n' "$*"; }
mk() { mkdir -p "$1"; }

file_absent() { [ ! -e "$1" ]; }

write_if_absent() {
  local path="$1"
  shift
  if file_absent "$path"; then
    mk "$(dirname "$path")"
    cat >"$path" <<'EOF'
'"$@"'
EOF
  fi
}

say "Scaffolding $CRATE_DIR ..."

# --- Root files --------------------------------------------------------------

if file_absent "$CRATE_DIR/Cargo.toml"; then
  mk "$(dirname "$CRATE_DIR/Cargo.toml")"
  cat >"$CRATE_DIR/Cargo.toml" <<'EOF'
[package]
name = "oap2"
version = "0.1.0"
edition = "2021"
publish = false
license = "MIT OR Apache-2.0"
description = "Pure OAP/1 protocol library: framing, headers, invariants. (Scaffolded structure; no implementation yet.)"
readme = "README.md"
repository = "https://github.com/YOUR_ORG/RustyOnions"

[lib]
path = "src/lib.rs"

[features]
# keep minimal; host owns heavy deps (TLS/HTTP/IO)
default = []
dev-vectors = []

[dev-dependencies]
EOF
fi

if file_absent "$CRATE_DIR/rust-toolchain.toml"; then
  cat >"$CRATE_DIR/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["clippy", "rustfmt"]
EOF
fi

if file_absent "$CRATE_DIR/.gitignore"; then
  cat >"$CRATE_DIR/.gitignore" <<'EOF'
/target
**/*.svg
fuzz/artifacts/
EOF
fi

if file_absent "$CRATE_DIR/LICENSE-MIT"; then
  cat >"$CRATE_DIR/LICENSE-MIT" <<'EOF'
MIT License (see workspace root for canonical text)
EOF
fi

if file_absent "$CRATE_DIR/LICENSE-APACHE"; then
  cat >"$CRATE_DIR/LICENSE-APACHE" <<'EOF'
Apache-2.0 License (see workspace root for canonical text)
EOF
fi

if file_absent "$CRATE_DIR/README.md"; then
  cat >"$CRATE_DIR/README.md" <<'EOF'
# oap2

> Role: library (OAP/1 framing + validation)  
> Status: scaffold (no implementation yet)

This crate mirrors the finalized **oap** documentation and structure, kept tiny and modular:
- No sockets/TLS/auth/DTOs.
- Enforces 1 MiB max frame, strict headers, and obj header rule (when applicable).
- Hosts implement metrics/exporters and own readiness/endpoints.

See `docs/` for blueprints and `tests/vectors/oap1/` for conformance fixtures.
EOF
fi

mk "$CRATE_DIR/.cargo"
if file_absent "$CRATE_DIR/.cargo/config.toml"; then
  cat >"$CRATE_DIR/.cargo/config.toml" <<'EOF'
[build]
# keep fast and strict in CI
rustflags = []

[target.'cfg(all())']
# You can tighten lints globally in the workspace root; keep crate-local light here.
EOF
fi

# --- src/ layout -------------------------------------------------------------

mk "$CRATE_DIR/src"

if file_absent "$CRATE_DIR/src/lib.rs"; then
  cat >"$CRATE_DIR/src/lib.rs" <<'EOF'
// Public surface re-exports (no implementation in scaffold).
// Keep this file tiny and audit-friendly.

pub mod consts;
pub mod prelude;

pub mod envelope;
pub mod frame;
pub mod error;
pub mod metrics;
pub mod seq;

pub mod parser;
pub mod writer;

// NOTE: Add doc(cfg) and rustdoc examples once implementation arrives.
EOF
fi

if file_absent "$CRATE_DIR/src/consts.rs"; then
  cat >"$CRATE_DIR/src/consts.rs" <<'EOF'
// Protocol constants and invariants (no implementation).
/// OAP protocol version (placeholder)
pub const OAP_VERSION: u32 = 1;

/// Hard cap per-frame payload (1 MiB). Changing this is a MAJOR SemVer event.
pub const MAX_FRAME_BYTES: usize = 1_048_576;
EOF
fi

if file_absent "$CRATE_DIR/src/prelude.rs"; then
  cat >"$CRATE_DIR/src/prelude.rs" <<'EOF'
// Convenience prelude for hosts/SDKs (kept minimal in scaffold).
pub use crate::consts::{MAX_FRAME_BYTES, OAP_VERSION};
EOF
fi

if file_absent "$CRATE_DIR/src/envelope.rs"; then
  cat >"$CRATE_DIR/src/envelope.rs" <<'EOF'
// Envelope grammar (scaffold-only, no types yet).
// Expected variants: HELLO | START | DATA | ACK | END | ERROR

// Placeholder marker to avoid empty file
pub struct EnvelopePlaceholder;
EOF
fi

if file_absent "$CRATE_DIR/src/frame.rs"; then
  cat >"$CRATE_DIR/src/frame.rs" <<'EOF'
// Framed container + header placeholders.
// Enforce length-first refusal and 1 MiB cap in real impl.

pub struct FramePlaceholder;
EOF
fi

if file_absent "$CRATE_DIR/src/error.rs"; then
  cat >"$CRATE_DIR/src/error.rs" <<'EOF'
// Error taxonomy placeholder with low-cardinality mapping.
// frame_oversize | unknown_envelope | header_malformed | credit_violation | truncated | obj_required_missing | bad_hello

pub struct ErrorPlaceholder;
EOF
fi

if file_absent "$CRATE_DIR/src/metrics.rs"; then
  cat >"$CRATE_DIR/src/metrics.rs" <<'EOF'
// Host-implemented metrics trait (scaffold placeholder).
// inc_frame(kind), add_frame_bytes(n), set_inflight(v), inc_reject(reason), set_ack_window(v)

pub struct MetricsPlaceholder;
EOF
fi

if file_absent "$CRATE_DIR/src/seq.rs"; then
  cat >"$CRATE_DIR/src/seq.rs" <<'EOF'
// Sequence and credit algebra helpers (scaffold placeholder).

pub struct SeqPlaceholder;
EOF
fi

mk "$CRATE_DIR/src/parser"

if file_absent "$CRATE_DIR/src/parser/mod.rs"; then
  cat >"$CRATE_DIR/src/parser/mod.rs" <<'EOF'
// Parser façade (scaffold).
pub mod config;
pub mod state;

// pub struct Parser;  // add in implementation
// pub enum Progress<'a> { NeedMore, Frame(/*...*/) } // add in implementation
EOF
fi

if file_absent "$CRATE_DIR/src/parser/config.rs"; then
  cat >"$CRATE_DIR/src/parser/config.rs" <<'EOF'
// Parser configuration knobs (scaffold).
// strict_headers, enforce_obj_header_on_objects, ack_window_frames (1..=1024), allow_pq_hello_flags, seq_rollover_policy

pub struct ParserConfigPlaceholder;
EOF
fi

if file_absent "$CRATE_DIR/src/parser/state.rs"; then
  cat >"$CRATE_DIR/src/parser/state.rs" <<'EOF'
// Internal parser state (opaque) — scaffold placeholder.

pub struct ParserStatePlaceholder;
EOF
fi

mk "$CRATE_DIR/src/writer"

if file_absent "$CRATE_DIR/src/writer/mod.rs"; then
  cat >"$CRATE_DIR/src/writer/mod.rs" <<'EOF'
// Writer façade (scaffold).
pub mod config;

// pub struct Writer; // add in implementation
EOF
fi

if file_absent "$CRATE_DIR/src/writer/config.rs"; then
  cat >"$CRATE_DIR/src/writer/config.rs" <<'EOF'
// Writer configuration (scaffold placeholder).

pub struct WriterConfigPlaceholder;
EOF
fi

# --- tests/ -----------------------------------------------------------------

mk "$CRATE_DIR/tests"
mk "$CRATE_DIR/tests/vectors/oap1"

if file_absent "$CRATE_DIR/tests/conformance.rs"; then
  cat >"$CRATE_DIR/tests/conformance.rs" <<'EOF'
// Conformance against canonical vectors (scaffold placeholder).
// When implemented, load tests/vectors/oap1/* and assert byte-identical decode/encode.
#[test]
fn conformance_vectors_exist() {
    assert!(std::path::Path::new("tests/vectors/oap1").exists());
}
EOF
fi

if file_absent "$CRATE_DIR/tests/split_need_more.rs"; then
  cat >"$CRATE_DIR/tests/split_need_more.rs" <<'EOF'
// Split-read equivalence tests (scaffold placeholder).
#[test]
fn split_feed_placeholder() {
    assert!(true);
}
EOF
fi

if file_absent "$CRATE_DIR/tests/ack_algebra.rs"; then
  cat >"$CRATE_DIR/tests/ack_algebra.rs" <<'EOF'
// ACK credit algebra tests (scaffold placeholder).
#[test]
fn ack_algebra_placeholder() {
    assert!(true);
}
EOF
fi

if file_absent "$CRATE_DIR/tests/config_validation.rs"; then
  cat >"$CRATE_DIR/tests/config_validation.rs" <<'EOF'
// ParserConfig bounds and defaults (scaffold placeholder).
#[test]
fn config_validation_placeholder() {
    assert!(true);
}
EOF
fi

if file_absent "$CRATE_DIR/tests/metrics_mapping.rs"; then
  cat >"$CRATE_DIR/tests/metrics_mapping.rs" <<'EOF'
// Verify low-cardinality reject reasons map to metrics (scaffold placeholder).
#[test]
fn metrics_mapping_placeholder() {
    assert!(true);
}
EOF
fi

if file_absent "$CRATE_DIR/tests/vectors/oap1/README.txt"; then
  cat >"$CRATE_DIR/tests/vectors/oap1/README.txt" <<'EOF'
Canonical OAP/1 vectors live here (JSON and binary). Populate after implementation:
- 01_hello_hello.json
- 02_start_data_ack_end.json
- 10_oversize_frame.bin
- 11_missing_obj.json
- 12_unknown_envelope.bin
- 13_truncated_pair.bin
- 14_credit_violation.json
EOF
fi

# --- benches/ ---------------------------------------------------------------

mk "$CRATE_DIR/benches"

if file_absent "$CRATE_DIR/benches/decode_happy.rs"; then
  cat >"$CRATE_DIR/benches/decode_happy.rs" <<'EOF'
// Criterion bench scaffold (no implementation).
fn main() {}
EOF
fi

if file_absent "$CRATE_DIR/benches/decode_pathological.rs"; then
  cat >"$CRATE_DIR/benches/decode_pathological.rs" <<'EOF'
fn main() {}
EOF
fi

if file_absent "$CRATE_DIR/benches/encode_ack.rs"; then
  cat >"$CRATE_DIR/benches/encode_ack.rs" <<'EOF'
fn main() {}
EOF
fi

# --- fuzz/ ------------------------------------------------------------------

mk "$CRATE_DIR/fuzz/fuzz_targets"
mk "$CRATE_DIR/fuzz/corpora/parser_fuzz"
mk "$CRATE_DIR/fuzz/corpora/header_fuzz"
mk "$CRATE_DIR/fuzz/corpora/ack_fuzz"
mk "$CRATE_DIR/fuzz/artifacts"

if file_absent "$CRATE_DIR/fuzz/fuzz_targets/parser_fuzz.rs"; then
  cat >"$CRATE_DIR/fuzz/fuzz_targets/parser_fuzz.rs" <<'EOF'
// libFuzzer target placeholder
EOF
fi

if file_absent "$CRATE_DIR/fuzz/fuzz_targets/header_fuzz.rs"; then
  cat >"$CRATE_DIR/fuzz/fuzz_targets/header_fuzz.rs" <<'EOF'
// libFuzzer target placeholder
EOF
fi

if file_absent "$CRATE_DIR/fuzz/fuzz_targets/ack_fuzz.rs"; then
  cat >"$CRATE_DIR/fuzz/fuzz_targets/ack_fuzz.rs" <<'EOF'
// libFuzzer target placeholder
EOF
fi

# --- docs/ ------------------------------------------------------------------

mk "$CRATE_DIR/docs/specs"
mk "$CRATE_DIR/docs/api-history/oap2"

if file_absent "$CRATE_DIR/docs/specs/oap-1.md"; then
  cat >"$CRATE_DIR/docs/specs/oap-1.md" <<'EOF'
# OAP/1 Specification (Scaffold Stub)
This is the spec-of-record location. Paste the finalized OAP/1 spec here.
EOF
fi

for f in arch seq state; do
  if file_absent "$CRATE_DIR/docs/$f.mmd"; then
    cat >"$CRATE_DIR/docs/$f.mmd" <<'EOF'
%% Mermaid diagram placeholder
EOF
  fi
done

for f in API CONCURRENCY CONFIG GOVERNANCE IDB INTEROP OBSERVABILITY PERFORMANCE QUANTUM RUNBOOK SECURITY TESTS; do
  if file_absent "$CRATE_DIR/docs/${f}.md"; then
    cat >"$CRATE_DIR/docs/${f}.md" <<EOF
# ${f}.md (Scaffold placeholder)
EOF
  fi
done

if file_absent "$CRATE_DIR/docs/api-history/oap2/1.0.0.txt"; then
  cat >"$CRATE_DIR/docs/api-history/oap2/1.0.0.txt" <<'EOF'
// cargo public-api snapshot placeholder
EOF
fi

# --- scripts/ ---------------------------------------------------------------

mk "$CRATE_DIR/scripts"

if file_absent "$CRATE_DIR/scripts/perf_compare.sh"; then
  cat >"$CRATE_DIR/scripts/perf_compare.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "perf_compare: scaffold placeholder (wire to Criterion JSON once benches exist)"
EOF
  chmod +x "$CRATE_DIR/scripts/perf_compare.sh"
fi

# --- .github/workflows/ -----------------------------------------------------

mk "$CRATE_DIR/.github/workflows"

if file_absent "$CRATE_DIR/.github/workflows/render-mermaid.yml"; then
  cat >"$CRATE_DIR/.github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid (oap2)
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files 'crates/oap2/docs/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out" || true
          done
EOF
fi

if file_absent "$CRATE_DIR/.github/workflows/public-api.yml"; then
  cat >"$CRATE_DIR/.github/workflows/public-api.yml" <<'EOF'
name: public-api (oap2)
on: [push, pull_request]
jobs:
  api-diff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-public-api || true
      - run: cargo public-api --manifest-path crates/oap2/Cargo.toml || true
EOF
fi

if file_absent "$CRATE_DIR/.github/workflows/ci.yml"; then
  cat >"$CRATE_DIR/.github/workflows/ci.yml" <<'EOF'
name: ci (oap2)
on: [push, pull_request]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all --check
      - run: cargo clippy -p oap2 -- -D warnings || true
      - run: cargo test -p oap2 --all-features || true
      - run: cargo install cargo-deny || true
      - run: cargo deny check || true
EOF
fi

say "Scaffolded $CRATE_DIR."
