#!/usr/bin/env bash
set -euo pipefail

# Convenience wrapper: build the UI and sync assets.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

"${crate_dir}/scripts/build-ui.sh"
