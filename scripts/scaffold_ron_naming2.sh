#!/usr/bin/env bash
set -euo pipefail

CRATE_DIR="crates/ron-naming2"
FORCE=0

if [[ "${1:-}" == "--force" ]]; then
  FORCE=1
fi

ok() { printf "\033[32m✔\033[0m %s\n" "$*"; }
info() { printf "\033[36m•\033[0m %s\n" "$*"; }
warn() { printf "\033[33m!\033[0m %s\n" "$*"; }
err() { printf "\033[31m✘ %s\033[0m\n" "$*" >&2; }

write_file() {
  # write_file <path> <<'EOF'
  local path="$1"
  shift
  if [[ -e "$path" && $FORCE -eq 0 ]]; then
    warn "exists (skip): $path"
    # consume here-doc content to /dev/null to keep caller simple
    cat > /dev/null
    return 0
  fi
  mkdir -p "$(dirname "$path")"
  cat > "$path"
  ok "wrote: $path"
}

info "Scaffolding ron-naming2 in $CRATE_DIR"

# ----- Root files -----
write_file "$CRATE_DIR/Cargo.toml" <<'EOF'
[package]
name = "ron-naming2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Pure library for naming schemas/normalization/encodings with an optional offline CLI (tldctl)."
authors = ["RustyOnions Contributors <dev@rustyonions.local>"]
readme = "README.md"
rust-version = "1.80"
repository = "https://github.com/Y3LLOWVESTS/rustyonions"

[lib]
path = "src/lib.rs"

[[bin]]
name = "tldctl"
path = "src/bin/tldctl.rs"
required-features = ["cli"]

[features]
default = []
cli = []        # enables offline CLI binary (no network)
verify = []     # enables detached verifier/signer traits (interfaces only)

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
# Canonical CBOR; keep lib runtime simple (no net/crypto here)
ciborium = { workspace = true }

[dev-dependencies]
# Keep tests minimal and portable
serde = { workspace = true, features = ["derive"] }

[package.metadata.docs]
# This crate is a pure library; no network or DB access.
EOF

write_file "$CRATE_DIR/README.md" <<'EOF'
# ron-naming2

> Pure library for naming schemas, normalization, and canonical (de)encodings.
> Optional **offline CLI** (`tldctl`) for authoring/linting/packing/signing/verifying.
> **No network, no DB, no services.**

## Scope (recap)
- Deterministic normalization (Unicode/IDNA policy), DTOs, and canonical JSON/CBOR.
- Address helpers for hygiene (e.g., `b3:<lower-hex>` form).
- Offline CLI wrapper for authoring tasks. No runtime crypto in the library.

## Anti-scope
- No HTTP endpoints, no DHT/index, no databases, no background tasks.

See `docs/` for the full set of canonical documents (IDB, INTEROP, SECURITY, PERFORMANCE, CONCURRENCY, QUANTUM, etc.).

## Build
- Library only: `cargo build -p ron-naming2`
- With CLI: `cargo build -p ron-naming2 --features cli`
EOF

write_file "$CRATE_DIR/CHANGELOG.md" <<'EOF'
# Changelog — ron-naming2

All notable changes will be documented here. This project adheres to Semantic Versioning.

## [0.1.0] - scaffold
- Initial scaffold: pure lib, optional CLI, docs structure, tests/benches placeholders, vectors/scripts directories.
EOF

write_file "$CRATE_DIR/LICENSE-APACHE" <<'EOF'
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/
...
EOF

write_file "$CRATE_DIR/LICENSE-MIT" <<'EOF'
MIT License

Copyright (c) 2025 RustyOnions

Permission is hereby granted, free of charge, to any person obtaining a copy...
EOF

write_file "$CRATE_DIR/CODEOWNERS" <<'EOF'
# Reviewers for API/contract changes in ron-naming2
* @Stevan-White
EOF

write_file "$CRATE_DIR/.cargo/config.toml" <<'EOF'
[build]
rustflags = ["-Dwarnings"]

[target.'cfg(all())']
# Keep builds deterministic and clean.

[term]
verbose = true
EOF

write_file "$CRATE_DIR/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["clippy", "rustfmt"]
profile = "minimal"
EOF

write_file "$CRATE_DIR/deny.toml" <<'EOF'
# cargo-deny baseline for ron-naming2 (minimal, expand as needed)
[advisories]
yanked = "deny"

[licenses]
unlicensed = "deny"
copyleft = "deny"
allow = [
  "Apache-2.0",
  "MIT",
  "Unicode-DFS-2016",
  "Unicode-3.0",
  "CC0-1.0",
  "CDLA-Permissive-2.0",
  "BSD-3-Clause",
]
confidence-threshold = 0.8
EOF

# ----- src/ tree -----
write_file "$CRATE_DIR/src/lib.rs" <<'EOF'
#![forbid(unsafe_code)]
//! ron-naming2 — pure library for naming schemas/normalization/encodings.
//!
//! This scaffold intentionally contains **no Rust logic yet**; it establishes
//! a modular file layout to keep code small and maintainable.
//!
//! Public re-exports will be added here as the implementation lands.

pub mod types;
pub mod normalize;
pub mod address;
pub mod version;
pub mod wire;

#[cfg(feature = "verify")]
pub mod verify;
EOF

write_file "$CRATE_DIR/src/types.rs" <<'EOF'
 //! DTOs and schema types (placeholder).
 //!
 //! Add structs like `CanonicalName`, `Label`, `TldEntry`, `TldMap`, etc.
 //! Use `#[serde(deny_unknown_fields)]` on external-facing types.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/normalize.rs" <<'EOF'
 //! Deterministic normalization pipeline (placeholder).
 //!
 //! Entry-point (to be added): `normalize_name(input, options) -> Result<..., ...>`.
 //! Enforce NFC, IDNA/UTS-46 non-transitional, lowercase, confusables policy.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/address.rs" <<'EOF'
 //! Address hygiene helpers (placeholder).
 //!
 //! Example: `is_b3_addr(s: &str) -> bool` validating `b3:<64-lower-hex>` form.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/version.rs" <<'EOF'
 //! Version/provenance reporting (placeholder).
 //!
 //! Expose Unicode/IDNA table versions and policy bundle identifiers.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/wire/mod.rs" <<'EOF'
 //! Canonical wire encoders/decoders (placeholder).
 //!
 //! Provide `to_canonical_bytes` / `from_bytes` and `WireFormat` enum.

 // Intentionally empty: no code per request (scaffold-only).

pub mod json;
pub mod cbor;
EOF

write_file "$CRATE_DIR/src/wire/json.rs" <<'EOF'
 //! Canonical JSON encoding/decoding (placeholder).
 //!
 //! Use stable serde options; deterministic maps/ordering as required.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/wire/cbor.rs" <<'EOF'
 //! Canonical CBOR encoding/decoding (placeholder).
 //!
 //! Keep options stable across versions for reproducible bytes.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/verify/mod.rs" <<'EOF'
 //! Detached verifier/signer traits (placeholder, feature = "verify").
 //!
 //! Keep runtime crypto out of default builds; provide only interfaces here.

 // Intentionally empty: no code per request (scaffold-only).
EOF

write_file "$CRATE_DIR/src/bin/tldctl.rs" <<'EOF'
// Offline CLI (placeholder, feature = "cli").
// Do not add logic yet — this file exists to reserve the bin name and layout.

// fn main() {
//     // Intentionally empty.
// }
EOF

# ----- docs/ tree -----
write_file "$CRATE_DIR/docs/API.md" <<'EOF'
# API.md (placeholder)
Authoritative API surface and stability notes for ron-naming2.
EOF

write_file "$CRATE_DIR/docs/CONCURRENCY.md" <<'EOF'
# Concurrency Model (placeholder)
Pure library: no background tasks, channels, or locks across .await.
EOF

write_file "$CRATE_DIR/docs/CONFIG.md" <<'EOF'
# CONFIG.md (placeholder)
No runtime config in library; CLI may accept file/flags.
EOF

write_file "$CRATE_DIR/docs/GOVERNANCE.md" <<'EOF'
# GOVERNANCE.md (placeholder)
Change control, CODEOWNERS policy, SemVer gates.
EOF

write_file "$CRATE_DIR/docs/IDB.md" <<'EOF'
# IDB.md (placeholder)
Invariant-Driven Blueprint for ron-naming2.
EOF

write_file "$CRATE_DIR/docs/INTEROP.md" <<'EOF'
# INTEROP.md (placeholder)
Wire formats, DTO schemas, vectors; no network endpoints in this crate.
EOF

write_file "$CRATE_DIR/docs/OBSERVABILITY.md" <<'EOF'
# OBSERVABILITY.md (placeholder)
Metrics/logs not applicable to pure lib; CLI emits stdout/stderr only.
EOF

write_file "$CRATE_DIR/docs/PERFORMANCE.md" <<'EOF'
# PERFORMANCE.md (placeholder)
Bench placeholders in `benches/` to track normalization/encode perf.
EOF

write_file "$CRATE_DIR/docs/QUANTUM.md" <<'EOF'
# QUANTUM.md (placeholder)
PQ posture: library remains crypto-free; detached attestations handled externally.
EOF

write_file "$CRATE_DIR/docs/RUNBOOK.md" <<'EOF'
# RUNBOOK.md (placeholder)
Release steps: update vectors, recompute BLAKE3, optional PQ attestation.
EOF

write_file "$CRATE_DIR/docs/SECURITY.md" <<'EOF'
# SECURITY.md (placeholder)
Threat model: input validation, confusables policy, no I/O, no network.
EOF

write_file "$CRATE_DIR/docs/TESTS.md" <<'EOF'
# TESTS.md (placeholder)
Golden vectors for DTO/wire round-trips; normalization idempotence.
EOF

write_file "$CRATE_DIR/docs/OLD_README.md" <<'EOF'
# OLD_README.md (placeholder)
Historical README content (if needed).
EOF

write_file "$CRATE_DIR/docs/diagrams/arch.mmd" <<'EOF'
%% Mermaid placeholder for architecture diagram
flowchart TD
  A[Caller] --> B[ron-naming2::normalize]
  A --> C[ron-naming2::wire::{json,cbor}]
  D[(Vectors)] --> C
  E[[tldctl (offline)]] -. calls .-> B
  E -. calls .-> C
EOF

write_file "$CRATE_DIR/docs/diagrams/arch.svg" <<'EOF'
<!-- Rendered diagram placeholder; CI may regenerate from arch.mmd -->
<svg xmlns="http://www.w3.org/2000/svg" width="400" height="120">
  <rect x="0" y="0" width="400" height="120" fill="#ffffff" stroke="#ddd"/>
  <text x="16" y="32" font-family="monospace" font-size="14">ron-naming2 architecture (placeholder)</text>
</svg>
EOF

# ----- examples/ -----
write_file "$CRATE_DIR/examples/normalize_roundtrip.rs" <<'EOF'
fn main() {
    // Placeholder example; real code will show normalize -> round-trip invariants.
    println!("ron-naming2 normalize_roundtrip example (placeholder).");
}
EOF

write_file "$CRATE_DIR/examples/encode_decode.rs" <<'EOF'
fn main() {
    // Placeholder example; real code will show JSON/CBOR encode/decode.
    println!("ron-naming2 encode_decode example (placeholder).");
}
EOF

# ----- tests/ -----
write_file "$CRATE_DIR/tests/normalize_idempotence.rs" <<'EOF'
#[test]
fn placeholder_normalize_idempotence() {
    // Intentionally not calling crate code yet; keeps CI green during scaffold.
    assert!(true);
}
EOF

write_file "$CRATE_DIR/tests/dto_wire_vectors.rs" <<'EOF'
#[test]
fn placeholder_dto_wire_vectors() {
    // This will later load testdata/vectors/*.json|*.cbor and assert deterministic bytes.
    assert!(true);
}
EOF

write_file "$CRATE_DIR/tests/address_hygiene.rs" <<'EOF'
#[test]
fn placeholder_address_hygiene() {
    // Will later assert `b3:<hex>` acceptance and reject malformed forms.
    assert!(true);
}
EOF

write_file "$CRATE_DIR/tests/cli_contract.rs" <<'EOF'
#[cfg(feature = "cli")]
#[test]
fn placeholder_cli_contract() {
    // Will later spawn the offline CLI (tldctl) in-process and validate exit codes.
    assert!(true);
}
EOF

# ----- benches/ -----
write_file "$CRATE_DIR/benches/normalize_bench.rs" <<'EOF'
#![allow(unused)]
fn main() {
    // Placeholder bench; replace with Criterion or custom harness when ready.
    println!("normalize_bench placeholder");
}
EOF

write_file "$CRATE_DIR/benches/encode_bench.rs" <<'EOF'
#![allow(unused)]
fn main() {
    // Placeholder bench; replace with Criterion or custom harness when ready.
    println!("encode_bench placeholder");
}
EOF

# ----- testdata/ -----
write_file "$CRATE_DIR/testdata/vectors/names_ascii.json" <<'EOF'
{
  "note": "placeholder ASCII names for normalization vectors",
  "items": ["example", "test", "alpha", "beta"]
}
EOF

write_file "$CRATE_DIR/testdata/vectors/names_unicode_mixed.json" <<'EOF'
{
  "note": "placeholder mixed-script names for confusables/IDNA tests",
  "items": ["ｅxample", "Εxample", "ｅхампⅼе"]
}
EOF

write_file "$CRATE_DIR/testdata/vectors/tldmap_minimal.json" <<'EOF'
{
  "note": "placeholder minimal TLD map",
  "tlds": { "example": { "policy": "placeholder" } }
}
EOF

write_file "$CRATE_DIR/testdata/vectors/tldmap_minimal.cbor" <<'EOF'
placeholder-cbor-binary-will-live-here
EOF

write_file "$CRATE_DIR/testdata/vectors/vectors.manifest.json" <<'EOF'
{
  "bundle": "ron-naming2-vectors",
  "files": [
    "names_ascii.json",
    "names_unicode_mixed.json",
    "tldmap_minimal.json",
    "tldmap_minimal.cbor"
  ],
  "integrity": {
    "algo": "BLAKE3-256",
    "hash_file": "../signatures/vectors.b3.txt"
  }
}
EOF

write_file "$CRATE_DIR/testdata/signatures/vectors.b3.txt" <<'EOF'
# Placeholder; populated by scripts/hash_vectors.sh during release.
EOF

write_file "$CRATE_DIR/testdata/signatures/vectors.attestation.txt" <<'EOF'
# Optional PQ attestation (e.g., Dilithium/SPHINCS+) for the vector bundle.
EOF

# ----- scripts/ -----
write_file "$CRATE_DIR/scripts/hash_vectors.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Portable BLAKE3 hashing: prefer 'b3sum' if available, else fallback to 'shasum -a 256' as placeholder.
# Replace fallback with a real BLAKE3 tool in your env/CI.

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VEC_DIR="$DIR/testdata/vectors"
OUT="$DIR/testdata/signatures/vectors.b3.txt"

if command -v b3sum >/dev/null 2>&1; then
  (cd "$VEC_DIR" && b3sum names_ascii.json names_unicode_mixed.json tldmap_minimal.json tldmap_minimal.cbor) > "$OUT"
  echo "algo=BLAKE3-256" >> "$OUT"
  echo "OK wrote $OUT (BLAKE3)"
else
  (cd "$VEC_DIR" && shasum -a 256 names_ascii.json names_unicode_mixed.json tldmap_minimal.json tldmap_minimal.cbor) > "$OUT"
  echo "algo=SHA-256 (TEMPORARY FALLBACK — replace with BLAKE3 in CI)" >> "$OUT"
  echo "Wrote $OUT (SHA-256 fallback)"
fi
EOF

write_file "$CRATE_DIR/scripts/verify_vectors_attestation.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Placeholder: call your organization-approved verifier here.
# Intended to verify testdata/signatures/vectors.attestation.txt against vectors and vectors.b3.txt.
echo "verify_vectors_attestation.sh: placeholder (no-op)"
EOF

write_file "$CRATE_DIR/scripts/render_mermaid.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Render Mermaid diagrams locally (optional).
# Requires 'mmdc' (mermaid-cli): npm install -g @mermaid-js/mermaid-cli
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/docs/diagrams/arch.mmd"
DST="$ROOT/docs/diagrams/arch.svg"

if command -v mmdc >/dev/null 2>&1; then
  mmdc -i "$SRC" -o "$DST"
  echo "Rendered $DST"
else
  echo "mermaid-cli (mmdc) not found; skipped render."
fi
EOF

# Make scripts executable
chmod +x "$CRATE_DIR/scripts/"*.sh || true

ok "Scaffold complete: $CRATE_DIR"
info "Next steps:"
echo " - Fill in src/* modules with real logic (keep functions small, test-first)."
echo " - Replace placeholder vectors and run scripts/hash_vectors.sh."
echo " - Add real docs under docs/*.md using your combined templates."
echo " - Enable CLI locally with: cargo build -p ron-naming2 --features cli"
