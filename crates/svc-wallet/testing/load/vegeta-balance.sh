#!/usr/bin/env bash
set -euo pipefail
echo "GET http://127.0.0.1:8080/v1/balance?account=alice&asset=USD" | vegeta attack -duration=10s | vegeta report
