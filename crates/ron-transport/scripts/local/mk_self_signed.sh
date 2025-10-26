#!/usr/bin/env bash
# RO:WHAT — Generate a quick self-signed cert/key for local TLS smoke.
set -euo pipefail

DIR="${1:-crates/ron-transport/scripts/local/certs}"
mkdir -p "$DIR"

CERT="$DIR/cert.pem"
KEY="$DIR/key.pem"

# 365-day self-signed; CN=localhost
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -days 365 \
  -subj "/CN=localhost" \
  -keyout "$KEY" -out "$CERT" >/dev/null 2>&1

echo "[ OK ] wrote $CERT and $KEY"
