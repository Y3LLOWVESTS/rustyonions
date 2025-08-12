# RustyOnions – Roadmap & TODO
_Last updated: 2025-08-12_

## Milestone 1 — Local TCP Overlay ✅
- [x] CLI: `ronode serve/put/get/bench`
- [x] Overlay E2E on TCP (localhost): PUT/GET round-trip
- [x] Store: sled-backed, chunked writes/reads
- [x] Config loader (JSON/TOML) with sane defaults
- [x] Transport seam in CLI (`--transport tcp|tor`)
- [x] Accounting counters crate (CountingStream/Counters) available

## Milestone 2 — Tor/Arti Integration (Phase 1: Outbound) ✅
- [x] Tor outbound via SOCKS (`--socks 127.0.0.1:9150` or `:9050`)
- [x] Arti transport scaffold using `CountingStream`
- [x] Config: `tor_socks` + CLI override
- [x] Verified PUT/GET using `--transport tor` (client via SOCKS)

---

## **Refactor In Progress** 🧱
We are actively restructuring the codebase before continuing with the remaining parts of Milestone 2. Expect file moves, trait extractions, and reduced monolithic functions.

- [ ] Split responsibilities:
  - [ ] `transport/` — pluggable transports (TcpDev, Arti) behind trait
  - [ ] `overlay/` — `OverlayClient` / `OverlayServer` (protocol only)
  - [ ] `storage/` — pluggable store (sled now; consider SQLite later)
  - [ ] `node-cli/` — CLI parsing & command dispatch
  - [ ] `accounting/` — keep focused on counters & windows
- [ ] Replace any remaining direct TCP usage with transport trait
- [ ] Clear error types & thiserror; remove `anyhow` at boundaries where helpful

---

### Remaining for Milestone 2 (Phase 2: Inbound HS) 🚧
- [ ] **Hidden service listener**: implement `ArtiTransport::listen()` to serve over `.onion`
- [ ] **Transportize overlay**: make overlay use `SmallMsgTransport` for both dial & listen (no direct `TcpStream`)
- [ ] **Plumb accounting through overlay** so `stats` reflects real traffic
- [ ] **Accept .onion targets** in CLI for `put/get` (host:port parsing is fine; document usage)
- [ ] **Integration test (Tor outbound)**: spin TCP listener, torify client via SOCKS, assert round-trip
- [ ] **Integration test (HS)**: spin HS listener, client via Tor, assert round-trip

## Milestone 4 — DevEx & Tooling 🧰
- [ ] `ronode init` — generate commented config (JSON/TOML)
- [ ] `cargo fmt` / `clippy` clean; deny warnings in CI
- [ ] GitHub Actions: build, test, fmt, clippy matrix
- [ ] Add `-v/-vv` structured logging everywhere; document `RUST_LOG`

## Milestone 5 — Benchmarks & Observability 📈
- [ ] `bench` to report msg/sec, bytes/sec, p50/p95 latency
- [ ] Expose counters (tx/rx totals & window) via CLI `stats`
- [ ] Optional metrics endpoint (later)

## Milestone 6 — Security Hardening 🔐
- [ ] Fuzz message framing & overlay ops
- [ ] Input validation & DOS safeguards (max chunk size, timeouts)
- [ ] Threat model doc (Tor mode vs TCP dev mode)

---

## Quick Status Summary
- ✅ Done: TCP overlay E2E, transport seam, Tor outbound via SOCKS, config loader.
- 🟡 In progress: accounting visibility (awaiting transportized overlay).
- ⛳ Next up: **Complete modular refactor**, then resume Milestone 2 Phase 2 with the hidden service listener.
