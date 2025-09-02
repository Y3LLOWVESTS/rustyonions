# RustyOnions — Runbook (TCP & Tor HS)

## 0) Prereqs
- Rust toolchain
- Tor daemon (macOS Homebrew example):
  ```bash
  brew install tor
  # torrc usually lives at: /opt/homebrew/etc/tor/torrc  (Apple Silicon)
  # or:                     /usr/local/etc/tor/torrc      (Intel)
  ```
- torrc should include:
  ```
  SocksPort 127.0.0.1:9050
  ControlPort 127.0.0.1:9051
  CookieAuthentication 1
  ```
  Restart tor: `brew services restart tor`

## 1) Build
```bash
cargo build --workspace --all-targets --all-features
```

## 2) Create a config
Copy this to `config.json` (or copy `config.example.json` to `config.json`):
```json
{
  "socks5_addr": "127.0.0.1:9050",
  "tor_ctrl_addr": "127.0.0.1:9051",
  "overlay_addr": "127.0.0.1:1777",
  "dev_inbox_addr": "127.0.0.1:2888",
  "connect_timeout_ms": 15000,
  "data_dir": ".data/overlay",
  "chunk_size": 65536
}
```

## 3) Baseline test — TCP
Terminal A:
```bash
cargo run -p node -- --log info serve --transport tcp
```
Terminal B:
```bash
./testing/roundtrip_tcp.sh
# Expected: "TCP PUT/GET OK ✅"
```

## 4) Tor Hidden Service (HS)
### 4.1 Start server (HS)
Terminal A:
```bash
RUST_LOG=sled=warn,arti_transport=info,overlay=info,ronode=info \
cargo run -p node -- serve --transport tor
# Look for: "hidden service available at <id>.onion:1777"
```

### 4.2 Client round-trip via HS
Terminal B:
```bash
./testing/roundtrip_hs.sh <id>.onion:1777
# Expected: "HS PUT/GET OK ✅"
```

## 5) Persistent HS (optional)
Keep the same `.onion` across restarts:
```bash
export RO_HS_KEY_FILE="$PWD/.data/hs_key"
RUST_LOG=sled=warn cargo run -p node -- serve --transport tor
# First run writes .data/hs_key; subsequent runs reuse the same address.
```
Rotate address: delete `.data/hs_key` and restart.

## 6) Troubleshooting
- **No SOCKS on 9050** → fix torrc, `brew services restart tor`, then:
  ```bash
  nc -vz 127.0.0.1 9050
  nc -vz 127.0.0.1 9051
  ```
- **Don’t see onion line** → reduce logs:
  ```bash
  RUST_LOG=sled=warn cargo run -p node -- serve --transport tor | tee /tmp/ro.log
  grep -oE '[a-z2-7]{56}\.onion:1777' /tmp/ro.log
  ```
- **Client can’t reach onion** → give Tor 10–20s after HS appears; keep server up; double-check `<id>.onion:1777`.
- **Auth/control errors** → ensure `ControlPort 9051` and `CookieAuthentication 1` are in torrc.

## 7) Clean shutdown
On exit, the server sends `DEL_ONION <ServiceID>` (best effort).  
Ephemeral mode: address disappears on exit.  
Persistent mode: address comes back on next run using the saved key.
