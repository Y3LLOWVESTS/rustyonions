# RustyOnions — Milestone 2 Test Kit

This kit contains copy‑pasteable scripts to build, then test both **TCP** and **Tor** paths using the new transportized overlay and accounting.

> Assumes you are at the repo root (the folder with `Cargo.toml`).

## Contents
- `testing/build.sh` — Clean build + clippy with warnings-as-errors
- `testing/test_tcp.sh` — Serve locally on TCP and PUT/GET a file
- `testing/test_tor.sh` — Serve via Tor Hidden Service and PUT/GET via `.onion`
- `config.sample.toml` — Example config with TCP addr + Tor SOCKS/Control
- `testing/print_stats.sh` — Tail logs to observe periodic accounting

## Quickstart

```bash
# 1) Copy kit contents into your repo
unzip ro_m2_testkit.zip -d .

# 2) Inspect/adjust config (ports/addresses may differ on your machine)
cat config.sample.toml

# 3) Build
bash testing/build.sh

# 4) TCP smoke test
bash testing/test_tcp.sh

# 5) Tor test (requires Tor running locally: socks at 9050, control at 9051)
bash testing/test_tor.sh
```

### Tor prerequisites
- Tor must be running with ControlPort enabled (e.g., `ControlPort 9051` and `HashedControlPassword` or a cookie).
- The repo's Arti transport expects `socks5_addr = "127.0.0.1:9050"` and `tor_ctrl_addr = "127.0.0.1:9051"` by default. Edit `config.sample.toml` if needed.
- Persistent onion: pass `--hs-key-file .data/hs_ed25519_key` in `serve` (script already shows the flag).

### Expected output
- `ronode serve` (TCP and Tor) logs periodic `stats/*` lines every ~10s.
- `ronode put/get` print per-operation deltas like `stats put tcp: +in=... +out=...`.
- Tor serve logs will include the published `.onion:1777` address. The Tor script captures it.

### Troubleshooting
- If `clippy` fails, fix the lint it points to; we use `-D warnings`.
- If Tor serve doesn’t print an onion address:
  - Ensure Tor is running and control auth is correct.
  - Verify your system clock is sane (Tor is sensitive to time drift).
  - Check firewall rules; HS publication needs directory access.
- If PUT/GET over Tor hangs:
  - Try increasing `connect_timeout_ms` in config.
  - Confirm the onion address/port matches serve output.
