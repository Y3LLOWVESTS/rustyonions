#!/usr/bin/env bash
# Scaffolds the ron-policy2 crate structure exactly matching the requested tree.
# No Rust code is generated—only minimal placeholder files.

set -euo pipefail

CRATE_NAME="ron-policy2"
CRATE_DIR="crates/${CRATE_NAME}"

if [ ! -d "${CRATE_DIR}" ]; then
  echo "error: crate directory '${CRATE_DIR}' not found. Create it first."
  exit 1
fi

echo "Scaffolding ${CRATE_DIR} ..."

# Directories
mkdir -p "${CRATE_DIR}/.cargo"
mkdir -p "${CRATE_DIR}/.github/workflows"
mkdir -p "${CRATE_DIR}/benches"
mkdir -p "${CRATE_DIR}/docs"
mkdir -p "${CRATE_DIR}/examples"
mkdir -p "${CRATE_DIR}/fuzz/fuzz_targets"
mkdir -p "${CRATE_DIR}/schema"
mkdir -p "${CRATE_DIR}/scripts"
mkdir -p "${CRATE_DIR}/specs"
mkdir -p "${CRATE_DIR}/src/ctx"
mkdir -p "${CRATE_DIR}/src/engine"
mkdir -p "${CRATE_DIR}/src/explain"
mkdir -p "${CRATE_DIR}/src/parse"
mkdir -p "${CRATE_DIR}/tests/helpers"
mkdir -p "${CRATE_DIR}/tests/vectors"

# Cargo.toml (placeholder, minimal — adjust later to match workspace pins)
tee "${CRATE_DIR}/Cargo.toml" >/dev/null <<'EOF'
[package]
name = "ron-policy2"
version = "0.0.0"
edition = "2021"
description = "Pure policy library scaffold (no I/O)."
license = "MIT OR Apache-2.0"
publish = false

[lib]
name = "ron_policy2"
path = "src/lib.rs"

[features]
default = []
json = []
toml = []
explain = []
geo = []
quota = []

[dependencies]
# Intentionally empty in scaffold to avoid drift.
EOF

# Root files
tee "${CRATE_DIR}/README.md" >/dev/null <<'EOF'
# ron-policy2
Pure library for policy modeling, parsing, validation, evaluation, and explain traces.
No I/O. No network. No endpoints. Placeholders only in this scaffold.
EOF

tee "${CRATE_DIR}/CHANGELOG.md" >/dev/null <<'EOF'
# CHANGELOG — ron-policy2
## [0.0.0] - scaffold
- Initial directory and docs scaffold (no code).
EOF

tee "${CRATE_DIR}/LICENSE-APACHE" >/dev/null <<'EOF'
Apache License 2.0 placeholder.
EOF

tee "${CRATE_DIR}/LICENSE-MIT" >/dev/null <<'EOF'
MIT License placeholder.
EOF

tee "${CRATE_DIR}/CODEOWNERS" >/dev/null <<'EOF'
* @OPO @SPR @RM
EOF

tee "${CRATE_DIR}/rust-toolchain.toml" >/dev/null <<'EOF'
[toolchain]
channel = "stable"
EOF

tee "${CRATE_DIR}/deny.toml" >/dev/null <<'EOF'
# Prefer workspace root cargo-deny; local overrides can go here if ever needed.
EOF

tee "${CRATE_DIR}/.cargo/config.toml" >/dev/null <<'EOF'
[build]
rustflags = ["-Dwarnings"]
EOF

# docs/
tee "${CRATE_DIR}/docs/IDB.md" >/dev/null <<'EOF'
# Invariant-Driven Blueprint — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/INTEROP.md" >/dev/null <<'EOF'
# INTEROP — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/CONFIG.md" >/dev/null <<'EOF'
# CONFIG — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/SECURITY.md" >/dev/null <<'EOF'
# SECURITY — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/OBSERVABILITY.md" >/dev/null <<'EOF'
# OBSERVABILITY — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/PERFORMANCE.md" >/dev/null <<'EOF'
# PERFORMANCE — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/CONCURRENCY.md" >/dev/null <<'EOF'
# CONCURRENCY — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/GOVERNANCE.md" >/dev/null <<'EOF'
# GOVERNANCE — ron-policy2 (placeholder)
EOF

tee "${CRATE_DIR}/docs/QUANTUM.md" >/dev/null <<'EOF'
# QUANTUM — ron-policy2 (placeholder)
EOF

# schema/ and specs/
tee "${CRATE_DIR}/schema/policybundle.schema.json" >/dev/null <<'EOF'
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "PolicyBundle (placeholder)",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "version": { "type": "string" },
    "rules":   { "type": "array" }
  },
  "required": ["version","rules"]
}
EOF

tee "${CRATE_DIR}/specs/governance.md" >/dev/null <<'EOF'
# Governance Spec (placeholder)
EOF

# src/
tee "${CRATE_DIR}/src/lib.rs" >/dev/null <<'EOF'
// ron-policy2 entry (placeholder)
EOF

tee "${CRATE_DIR}/src/model.rs" >/dev/null <<'EOF'
// model (placeholder)
EOF

tee "${CRATE_DIR}/src/errors.rs" >/dev/null <<'EOF'
// errors (placeholder)
EOF

tee "${CRATE_DIR}/src/features.rs" >/dev/null <<'EOF'
// features (placeholder)
EOF

tee "${CRATE_DIR}/src/ctx/mod.rs" >/dev/null <<'EOF'
// ctx/mod (placeholder)
EOF

tee "${CRATE_DIR}/src/ctx/normalize.rs" >/dev/null <<'EOF'
// ctx/normalize (placeholder)
EOF

tee "${CRATE_DIR}/src/ctx/clock.rs" >/dev/null <<'EOF'
// ctx/clock (placeholder)
EOF

tee "${CRATE_DIR}/src/parse/mod.rs" >/dev/null <<'EOF'
// parse/mod (placeholder)
EOF

tee "${CRATE_DIR}/src/parse/json.rs" >/dev/null <<'EOF'
// parse/json (placeholder)
EOF

tee "${CRATE_DIR}/src/parse/toml.rs" >/dev/null <<'EOF'
// parse/toml (placeholder)
EOF

tee "${CRATE_DIR}/src/parse/validate.rs" >/dev/null <<'EOF'
// parse/validate (placeholder)
EOF

tee "${CRATE_DIR}/src/engine/mod.rs" >/dev/null <<'EOF'
// engine/mod (placeholder)
EOF

tee "${CRATE_DIR}/src/engine/eval.rs" >/dev/null <<'EOF'
// engine/eval (placeholder)
EOF

tee "${CRATE_DIR}/src/engine/index.rs" >/dev/null <<'EOF'
// engine/index (placeholder)
EOF

tee "${CRATE_DIR}/src/engine/obligations.rs" >/dev/null <<'EOF'
// engine/obligations (placeholder)
EOF

tee "${CRATE_DIR}/src/engine/reason.rs" >/dev/null <<'EOF'
// engine/reason (placeholder)
EOF

tee "${CRATE_DIR}/src/engine/metrics.rs" >/dev/null <<'EOF'
// engine/metrics (placeholder)
EOF

tee "${CRATE_DIR}/src/explain/mod.rs" >/dev/null <<'EOF'
// explain/mod (placeholder)
EOF

tee "${CRATE_DIR}/src/explain/trace.rs" >/dev/null <<'EOF'
// explain/trace (placeholder)
EOF

# tests/
tee "${CRATE_DIR}/tests/unit_model_serde_strict.rs" >/dev/null <<'EOF'
// unit_model_serde_strict (placeholder)
EOF

tee "${CRATE_DIR}/tests/unit_eval_determinism.rs" >/dev/null <<'EOF'
// unit_eval_determinism (placeholder)
EOF

tee "${CRATE_DIR}/tests/unit_tighten_only.rs" >/dev/null <<'EOF'
// unit_tighten_only (placeholder)
EOF

tee "${CRATE_DIR}/tests/unit_churn_protection.rs" >/dev/null <<'EOF'
// unit_churn_protection (placeholder)
EOF

tee "${CRATE_DIR}/tests/golden_reasons.rs" >/dev/null <<'EOF'
// golden_reasons (placeholder)
EOF

tee "${CRATE_DIR}/tests/helpers/bundle_load.rs" >/dev/null <<'EOF'
// tests/helpers/bundle_load (placeholder)
EOF

tee "${CRATE_DIR}/tests/vectors/deny_region.json" >/dev/null <<'EOF'
{ "version": "0.0.0", "rules": [ { "deny": "region.denied" } ] }
EOF

tee "${CRATE_DIR}/tests/vectors/body_too_large.json" >/dev/null <<'EOF'
{ "version": "0.0.0", "rules": [ { "deny": "body.too_large" } ] }
EOF

tee "${CRATE_DIR}/tests/vectors/decompress_guard.json" >/dev/null <<'EOF'
{ "version": "0.0.0", "rules": [ { "deny": "decompress.guard" } ] }
EOF

# benches/
tee "${CRATE_DIR}/benches/eval_throughput.rs" >/dev/null <<'EOF'
// benches/eval_throughput (placeholder)
EOF

# fuzz/
tee "${CRATE_DIR}/fuzz/Cargo.toml" >/dev/null <<'EOF'
[package]
name = "ron-policy2-fuzz"
version = "0.0.0"
edition = "2021"
publish = false
EOF

tee "${CRATE_DIR}/fuzz/fuzz_targets/fuzz_bundle_parse.rs" >/dev/null <<'EOF'
// fuzz_bundle_parse (placeholder)
EOF

tee "${CRATE_DIR}/fuzz/fuzz_targets/fuzz_eval.rs" >/dev/null <<'EOF'
// fuzz_eval (placeholder)
EOF

# examples/
tee "${CRATE_DIR}/examples/minimal_allow_deny.rs" >/dev/null <<'EOF'
// minimal_allow_deny (placeholder)
EOF

# CI & scripts
tee "${CRATE_DIR}/.github/workflows/policy.yml" >/dev/null <<'EOF'
name: ron-policy2
on:
  pull_request:
  push:
    branches: [ main ]
jobs:
  placeholder:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "CI placeholder for ron-policy2 (lint/tests/deny/benches/fuzz to be added)."
EOF

tee "${CRATE_DIR}/scripts/ci_invariants.sh" >/dev/null <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Placeholder invariant checks for ron-policy2."
EOF
chmod +x "${CRATE_DIR}/scripts/ci_invariants.sh"

echo "Done. Fully scaffolded ${CRATE_DIR} with all files."
