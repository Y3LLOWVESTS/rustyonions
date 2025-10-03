#!/usr/bin/env bash
# Quick metrics scrape (placeholder).
set -euo pipefail
curl -s http://127.0.0.1:9301/metrics || echo "metrics endpoint not yet implemented"
