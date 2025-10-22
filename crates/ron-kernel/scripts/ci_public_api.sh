#!/usr/bin/env bash
set -euo pipefail
cargo install cargo-public-api >/dev/null 2>&1 || true
cargo public-api -p ron-kernel --simplified
