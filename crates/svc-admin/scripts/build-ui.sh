#!/usr/bin/env bash
set -euo pipefail

# Build the svc-admin UI and copy artifacts into static/ (or for embedding).

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

cd "${crate_dir}/ui"
npm install
npm run build

mkdir -p "${crate_dir}/static"
cp -R dist/* "${crate_dir}/static/"
