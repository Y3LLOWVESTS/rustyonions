#!/usr/bin/env bash
# RO:WHAT — Build (--features tls), start tls_transport, and probe with openssl.
set -euo pipefail

# Ensure certs exist
bash crates/ron-transport/scripts/local/mk_self_signed.sh >/dev/null

CERT="crates/ron-transport/scripts/local/certs/cert.pem"
KEY="crates/ron-transport/scripts/local/certs/key.pem"

# Build with TLS
cargo build -q -p ron-transport --features tls

# Run tls_transport example
LOG_FILE="$(mktemp -t ron_transport_tls.XXXXXX.log)"
RUST_LOG=info cargo run -q -p ron-transport --features tls --example tls_transport -- "$CERT" "$KEY" >"$LOG_FILE" 2>&1 &
PID=$!
trap 'kill $PID >/dev/null 2>&1 || true; rm -f "$LOG_FILE"' EXIT

# Wait for it to start
for _ in {1..50}; do
  if grep -q "tls-transport listening on" "$LOG_FILE"; then break; fi
  sleep 0.1
done

line="$(grep "tls-transport listening on" "$LOG_FILE" | tail -n1)"
PORT="$(awk -F: '{print $NF}' <<<"$line" | tr -d '[:space:]')"
HOST="$(sed -E 's/.* on ([0-9\.]+):[0-9]+/\1/' <<<"$line")"
echo "[ OK ] tls server: ${HOST}:${PORT}"

# Probe with openssl s_client (TLS 1.3), send a line, and exit.
# Expect no HTTP response — this proves handshake success.
printf 'hello over TLS\n' | openssl s_client -quiet -connect "${HOST}:${PORT}" -tls1_3 -servername localhost >/dev/null 2>&1 || true
echo "[ OK ] openssl s_client connected + wrote bytes (no response expected)"
