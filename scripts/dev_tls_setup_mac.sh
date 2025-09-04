#!/usr/bin/env bash
set -euo pipefail

TLS_DIR=${TLS_DIR:-testing/tls}
mkdir -p "$TLS_DIR"

CA_KEY="$TLS_DIR/ca.key"
CA_CRT="$TLS_DIR/ca.crt"
SRV_KEY="$TLS_DIR/server.key"
SRV_CSR="$TLS_DIR/server.csr"
SRV_CRT="$TLS_DIR/server.crt"
SRV_CNF="$TLS_DIR/server.cnf"

echo "[tls] generating dev CA (if missing)"
if [ ! -f "$CA_KEY" ] || [ ! -f "$CA_CRT" ]; then
  openssl genrsa -out "$CA_KEY" 2048
  openssl req -x509 -new -nodes -key "$CA_KEY" -sha256 -days 3650 \
    -subj "/CN=RustyOnions Dev CA" -out "$CA_CRT"
fi

echo "[tls] generating localhost server cert with SANs (if missing)"
if [ ! -f "$SRV_KEY" ] || [ ! -f "$SRV_CRT" ]; then
  cat > "$SRV_CNF" <<EOF
[ req ]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no
[ req_distinguished_name ]
CN = localhost
[ v3_req ]
subjectAltName = @alt_names
[ alt_names ]
DNS.1 = localhost
IP.1 = 127.0.0.1
EOF
  openssl genrsa -out "$SRV_KEY" 2048
  openssl req -new -key "$SRV_KEY" -out "$SRV_CSR" -config "$SRV_CNF"
  openssl x509 -req -in "$SRV_CSR" -CA "$CA_CRT" -CAkey "$CA_KEY" -CAcreateserial \
    -out "$SRV_CRT" -days 825 -sha256 -extensions v3_req -extfile "$SRV_CNF"
fi

KEYCHAIN="${KEYCHAIN:-$HOME/Library/Keychains/login.keychain-db}"

echo "[tls] trusting dev CA in login keychain: $KEYCHAIN"
security add-trusted-cert -d -r trustRoot -k "$KEYCHAIN" "$CA_CRT" 2>/dev/null || true

echo "[tls] done. CA: $CA_CRT  server: $SRV_CRT"
