# RustyOnions – Roadmap & TODO (Updated)
_Last updated: 2025-08-13_

---

## Milestone 1 — Local TCP Overlay ✅
- [x] CLI: `ronode serve/put/get/bench`
- [x] Overlay E2E on TCP (localhost): PUT/GET round-trip
- [x] Store: sled-backed, chunked writes/reads
- [x] Config loader (JSON/TOML) with sane defaults
- [x] Transport seam in CLI (`--transport tcp|tor`)
- [x] Accounting counters crate (CountingStream/Counters) available

---

## Milestone 2 — Tor/Arti Integration
**Phase 1: Outbound** ✅
- [x] Tor outbound via SOCKS (`--socks 127.0.0.1:9150` or `:9050`)
- [x] Arti transport scaffold using `CountingStream`
- [x] Config: `tor_socks` + CLI override
- [x] Verified PUT/GET using `--transport tor` (client via SOCKS)

**Phase 2: Inbound Hidden Service (HS)** 🚧 _Partial_
- [x] Integration test: torified client → TCP server (SOCKS)
- [x] Integration test: client via Tor → HS listener (round-trip)  
- [ ] `ArtiTransport::listen()` to serve over `.onion` with clean start/stop
- [ ] Tor HS key management (ephemeral vs persistent) via config/CLI
- [ ] **Transportize overlay** for both dial & listen (no direct `TcpStream`)
- [ ] CLI accepts `.onion` targets for `put/get`; document usage
- [ ] **Plumb accounting through overlay** so `stats` reflects real traffic

---

## Milestone 3 — Modularization & Feature Flags 🧱 (In Progress)
- [ ] Split responsibilities into crates/modules:
  - [ ] `transport/` — pluggable transports (TcpDev, Arti) behind `Transport` trait
  - [ ] `overlay/` — `OverlayClient` / `OverlayServer` (protocol only)
  - [ ] `storage/` — `Store` trait; sled impl (default)
  - [ ] `messenger/` — E2E sessions, retries, framing
  - [ ] `node-cli/` — CLI parsing & command dispatch
  - [ ] `accounting/` — counters & time windows
- [ ] Replace remaining direct TCP usage with `Transport` everywhere
- [ ] Error handling: `thiserror` in libraries; `anyhow` at CLI boundary
- [ ] Graceful shutdown (tokio signals): flush counters, close Arti/TCP, persist as needed
- **Feature flags**
  - [ ] `dev-transport` (default), [ ] `arti-transport`, [ ] `amnesia`, [ ] `compression`, [ ] `metrics`
- **Config layering**
  - [ ] TOML/JSON file + env + CLI; `ronode --print-config` for effective config

---

## Milestone 4 — DevEx, Docs & CI 🧰
- [ ] GitHub Actions: build, test, `fmt --check`, `clippy -D warnings` (matrix)
- [ ] `justfile` (or Makefile) for common dev tasks
- [ ] `ronode init` — generate commented config (JSON/TOML)
- [ ] Reproducible builds: keep `Cargo.lock` in repo; consider `cargo dist`
- [ ] API docs: `cargo doc`; publish or artifact in CI
- [ ] CONTRIBUTING.md + issue/PR templates; milestone labels
- [ ] Structured logging (`-v/-vv`) and `RUST_LOG` docs

---

## Milestone 5 — Metrics, Stats & Bench 📈
- [ ] Ensure `CountingStream` wraps all transports and overlay paths
- [ ] CLI: `ronode stats --json` (tx/rx totals & windows, peer count, storage, relay contribution)
- [ ] `ronode bench` reports msg/sec, bytes/sec, p50/p95 latency
- [ ] Optional metrics endpoint behind `metrics` feature

---

## Milestone 6 — Security & Crypto Hygiene 🔐
- [ ] Default crypto: X25519 key exchange + ChaCha20-Poly1305 (or AES-GCM) via well-maintained crates
- [ ] Nonce policy: per-session counters or random nonces + replay protection
- [ ] Message framing: versioned header; bounds-checked frame sizes
- [ ] Input validation & DoS safeguards (timeouts, backpressure limits)
- [ ] Fuzzing with `cargo fuzz` for framing/parsers
- [ ] Threat model document (TCP dev vs Tor/HS modes)
- [ ] Zeroize sensitive key material (`zeroize`); minimize log PII

---

## Milestone 7 — Metering & “Responsible Relay” 🚦
- [ ] Configurable caps/thresholds with warnings at 80/90/100%
- [ ] Explicit consent flow to enable relay; default OFF
- [ ] `ronode relay --status` for contribution & caps
- [ ] Summarized, rate-limited accounting logs
- [ ] Display legal notice/disclaimer text when enabling (configurable)

---

## Milestone 8 — Discovery & Networking Evolution 🌐
- [ ] Bootstrap peer list with retry/backoff + jitter
- [ ] Discovery behind trait; optional `libp2p`/Kademlia under feature flag
- [ ] CLI/Config for seed peers; document upgrade path

---

## Milestone 9 — Amnesia Mode & Ephemerality 🧽
- [ ] `--amnesia` runtime profile: RAM-backed caches, temp dirs under a disposable root
- [ ] No persistent logs (warn-level summaries only); redact sensitive fields
- [ ] Aggressive purge timers and on-exit scrubbing; zeroize secrets
- [ ] Document operational caveats and expected performance tradeoffs

---

## Milestone 10 — Storage Improvements 📦
- [ ] `Store` trait abstraction finalized; sled as default impl
- [ ] Optional `zstd` compression per chunk (feature `compression`)
- [ ] Bench: compressed vs uncompressed thresholds
- [ ] Evaluate RocksDB impl behind feature flag **only if profiling indicates bottleneck**

---

## Test Plan Summary ✅
- Unit: chunk hashing; put/get; crypto round-trips; config merge
- Integration: TCP loopback; Tor outbound (SOCKS); HS inbound
- CI: run tests & lints on PRs; artifact logs for failures
- Mocks: Arti/Tor stubs for CI (no real network dependency)

---

## Quick Status
- ✅ Done: Milestone 1; Milestone 2 (Phase 1)
- 🧱 In progress: Milestone 2 (Phase 2 HS) & Milestone 3 (modularization/feature flags)
- ⛳ Next up: Finish Milestone 2 (Phase 2 HS), then Milestones 4–6
