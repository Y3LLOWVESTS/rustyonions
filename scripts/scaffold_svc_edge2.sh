#!/usr/bin/env bash
# svc-edge2 scaffold builder (ASCII-only)
# Usage:
#   bash scripts/scaffold_svc_edge2.sh [--force]
#
# Notes:
# - Creates directories and placeholder files only (no Rust code).
# - Safe by default: won't overwrite existing files unless --force is passed.

set -euo pipefail

FORCE=0
if [[ "${1:-}" == "--force" ]]; then
  FORCE=1
fi

# Resolve repo root if inside a git repo; otherwise use CWD.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
CRATE_DIR="$REPO_ROOT/crates/svc-edge2"

say() { printf "%s\n" "$*"; }
mkd() { mkdir -p "$1"; say "dir: ${1#$REPO_ROOT/}"; }

write_file() {
  local path="$1"; shift
  local content="$*"
  if [[ -e "$path" && $FORCE -eq 0 ]]; then
    say "skip (exists): ${path#$REPO_ROOT/}"
    return 0
  fi
  mkd "$(dirname "$path")"
  printf "%s" "$content" > "$path"
  say "write: ${path#$REPO_ROOT/}"
}

touch_file() {
  local path="$1"
  if [[ -e "$path" && $FORCE -eq 0 ]]; then
    say "skip (exists): ${path#$REPO_ROOT/}"
    return 0
  fi
  mkd "$(dirname "$path")"
  : > "$path"
  say "touch: ${path#$REPO_ROOT/}"
}

# Guard: ensure crate dir exists
mkd "$CRATE_DIR"

# 1) Top-level files
write_file "$CRATE_DIR/CHANGELOG.md" "--
title: svc-edge2 Changelog
status: draft
--

All notable changes to this crate will be documented here, following SemVer.
"

write_file "$CRATE_DIR/.gitignore" "/target
**/*.svg
.DS_Store
"

# Licenses (placeholders)
write_file "$CRATE_DIR/LICENSE-MIT" "MIT License

Copyright (c) $(date +%Y) Stevan White

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction...
"
write_file "$CRATE_DIR/LICENSE-APACHE" "Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/
"

# Optional .cargo config
write_file "$CRATE_DIR/.cargo/config.toml" "[build]
rustflags = [\"-Dwarnings\"]

[target.'cfg(all())']
# Reserved for local tweaks
"

# 2) Configs
write_file "$CRATE_DIR/configs/svc-edge.toml" "# Example config for svc-edge2 (no secrets)
bind_addr = \"0.0.0.0:8080\"
metrics_addr = \"127.0.0.1:9909\"

[edge]
mode = \"offline\"          # offline | live
packs = [\"./packs/sample.pmtiles\"]

[edge.allow]
# when mode = \"live\", allow-list hostnames:
hosts = [ ]

[ingress]
max_inflight = 512
rps_limit = 500
body_bytes = \"1MiB\"
decompress_max_ratio = 10
decompress_abs_bytes = \"1MiB\"
"

# 3) Docs (skeletons + mermaid sources)
for f in API CONFIG CONCURRENCY GOVERNANCE IDB INTEROP OBSERVABILITY PERFORMANCE QUANTUM RUNBOOK SECURITY TESTS; do
  write_file "$CRATE_DIR/docs/${f}.md" "# ${f}.md — svc-edge2

Status: draft

This document is a placeholder stub aligned with the RustyOnions templates.
Fill with crate-specific content distilled from ALL_DOCS.md for svc-edge.
"
done

# OpenAPI stub
write_file "$CRATE_DIR/docs/openapi/svc-edge.yaml" "openapi: 3.0.3
info:
  title: svc-edge2
  version: 0.0.0
paths:
  /healthz:
    get:
      responses:
        '200':
          description: ok
  /readyz:
    get:
      responses:
        '200':
          description: ready payload
  /metrics:
    get:
      responses:
        '200':
          description: prometheus text
  /edge/assets/{path}:
    get:
      parameters:
        - name: path
          in: path
          required: true
          schema: { type: string }
      responses:
        '200': { description: bytes }
        '206': { description: partial content }
        '304': { description: not modified }
        '416': { description: invalid range }
"

# Mermaid sources
write_file "$CRATE_DIR/docs/mmd/arch.mmd" "flowchart LR
  subgraph Client/Node
    A[Caller: svc-gateway/Omnigate] -->|HTTP GET| B(svc-edge2)
  end
  B -->|Pack/CAS read| D[(Local packs / CAS)]
  B -->|Live fill (allow-list HTTPS)| C[Origin(s)]
  B -->|Metrics| E[[Prometheus]]
  style B fill:#0b7285,stroke:#083344,color:#fff
"
write_file "$CRATE_DIR/docs/mmd/sequence.mmd" "sequenceDiagram
  actor Client
  participant GW as svc-gateway
  participant E as svc-edge2
  participant CAS as Packs/CAS
  participant O as HTTPS Origin
  Client->>GW: GET /edge/assets/{path}
  GW->>E: GET /edge/assets/{path}
  alt Hit
    E->>CAS: Read (Range/ETag)
    CAS-->>E: Bytes + ETag
    E-->>GW: 200/206 strong ETag
  else Miss
    E->>O: HTTPS GET (allow-list)
    O-->>E: 200 OK
    E->>E: Validate + (opt) CAS write
    E-->>GW: 200 strong ETag
  end
"
write_file "$CRATE_DIR/docs/mmd/state.mmd" "stateDiagram-v2
  [*] --> Idle
  Idle --> Running: bind + packs verified
  Running --> Degraded: inflight/rps > caps
  Degraded --> Running: load drops / replicas added
  Running --> Shutdown: signal drain
  Shutdown --> [*]
"

# Placeholder SVGs (empty; CI renders real SVGs from .mmd)
touch_file "$CRATE_DIR/docs/svg/arch.svg"
touch_file "$CRATE_DIR/docs/svg/sequence.svg"
touch_file "$CRATE_DIR/docs/svg/state.svg"

# 4) Source tree placeholders (no Rust code)
touch_file "$CRATE_DIR/src/lib.rs"
write_file "$CRATE_DIR/src/bin/svc-edge.rs" "// binary entrypoint placeholder (no code yet)
fn main() {
    println!(\"svc-edge2 placeholder\");
}
"
for f in cli config errors metrics state readiness supervisor; do
  touch_file "$CRATE_DIR/src/${f}.rs"
done

for d in admission routes adapters http work security util; do
  mkd "$CRATE_DIR/src/$d"
done

for f in body_cap decompress_guard inflight_cap rps_limit timeout; do
  touch_file "$CRATE_DIR/src/admission/${f}.rs"
done

for f in mod assets health ready prometheus; do
  touch_file "$CRATE_DIR/src/routes/${f}.rs"
done

for f in pack cas live_fill tls; do
  touch_file "$CRATE_DIR/src/adapters/${f}.rs"
done

for f in headers range etag; do
  touch_file "$CRATE_DIR/src/http/${f}.rs"
done

for f in queue worker shutdown; do
  touch_file "$CRATE_DIR/src/work/${f}.rs"
done

for f in cors hsts audit; do
  touch_file "$CRATE_DIR/src/security/${f}.rs"
done

for f in bytes size_parse backoff; do
  touch_file "$CRATE_DIR/src/util/${f}.rs"
done

# 5) Tests and fixtures (skeletons)
mkd "$CRATE_DIR/tests/fixtures/packs"
mkd "$CRATE_DIR/tests/fixtures/http"
touch_file "$CRATE_DIR/tests/fixtures/packs/sample.pmtiles"
write_file "$CRATE_DIR/tests/fixtures/http/warm_hit.response" "HTTP/1.1 200 OK
ETag: \"b3:placeholder\"
Content-Length: 42
"
write_file "$CRATE_DIR/tests/fixtures/http/range_206.response" "HTTP/1.1 206 Partial Content
ETag: \"b3:placeholder\"
Content-Range: bytes 0-65535/999999
"

for f in \
  i_1_hardening_ingress \
  i_4_http_semantics \
  i_5_content_address \
  i_6_amnesia \
  i_8_size_bounds \
  i_9_observability_contract \
  i_10_deterministic_failures \
  i_11_pack_integrity \
  http_contract \
  readiness_logic \
  concurrency_backpressure \
  fuzz_headers
do
  touch_file "$CRATE_DIR/tests/${f}.rs"
done

# 6) Benches
for f in bench_range bench_blake3 bench_pack_read; do
  touch_file "$CRATE_DIR/benches/${f}.rs"
done

# 7) Scripts
write_file "$CRATE_DIR/scripts/dev-run.sh" "#!/usr/bin/env bash
set -euo pipefail
cd \"\$(dirname \"\${BASH_SOURCE[0]}\")/..\"
RUST_LOG=\${RUST_LOG:-info} \\
SVC_EDGE_BIND_ADDR=\${SVC_EDGE_BIND_ADDR:-0.0.0.0:8080} \\
SVC_EDGE_METRICS_ADDR=\${SVC_EDGE_METRICS_ADDR:-127.0.0.1:9909} \\
SVC_EDGE_SECURITY__AMNESIA=\${SVC_EDGE_SECURITY__AMNESIA:-true} \\
cargo run -p svc-edge2 -- \\
  --config ./configs/svc-edge.toml
"
chmod +x "$CRATE_DIR/scripts/dev-run.sh"

write_file "$CRATE_DIR/scripts/render-mermaid.sh" "#!/usr/bin/env bash
set -euo pipefail
cd \"\$(dirname \"\${BASH_SOURCE[0]}\")/..\"
for f in \$(git ls-files 'docs/mmd/*.mmd'); do
  out=\${f/mmd/svg}
  out=\${out%.mmd}.svg
  mkdir -p \$(dirname \"\$out\")
  mmdc -i \"\$f\" -o \"\$out\"
done
"
chmod +x "$CRATE_DIR/scripts/render-mermaid.sh"

write_file "$CRATE_DIR/scripts/perf-smoke.sh" "#!/usr/bin/env bash
set -euo pipefail
URL=\${1:-http://127.0.0.1:8080/edge/assets/path}
bombardier -c 64 -d 60s -l \"\$URL\"
"
chmod +x "$CRATE_DIR/scripts/perf-smoke.sh"

# 8) GitHub Actions (minimal placeholders)
write_file "$CRATE_DIR/.github/workflows/ci.yaml" "name: ci
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all --check
      - run: cargo clippy -p svc-edge2 -- -D warnings
      - run: cargo test -p svc-edge2
"

write_file "$CRATE_DIR/.github/workflows/render-mermaid.yaml" "name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: bash crates/svc-edge2/scripts/render-mermaid.sh
"

write_file "$CRATE_DIR/.github/workflows/public-api.yaml" "name: public-api
on: [push, pull_request]
jobs:
  api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-public-api || true
      - run: cargo public-api -p svc-edge2
"

write_file "$CRATE_DIR/.github/workflows/perf-guard.yaml" "name: perf-guard
on:
  schedule:
    - cron: '0 3 * * *'
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo 'perf guard placeholder'
"

say "Done. Scaffold created under crates/svc-edge2 (FORCE=$FORCE)."
