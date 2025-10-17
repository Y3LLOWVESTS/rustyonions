#!/usr/bin/env bash
set -euo pipefail
URL=${1:-http://127.0.0.1:8080/edge/assets/path}
bombardier -c 64 -d 60s -l "$URL"
