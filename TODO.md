# RustyOnions – Roadmap & TODO
_Last updated: 2025-08-13_

---

## Milestone 1 — Local TCP Overlay ✅
- [x] Sled-backed storage for public chunks
- [x] TCP listener for overlay plane
- [x] CLI for overlay get/put
- [x] Basic redundancy

---

## Milestone 2 — Tor/Arti Integration (Phase 1: Outbound) ✅
- [x] SOCKS outbound
- [x] Arti transport scaffold
- [x] Verified Tor PUT/GET

**Phase 2: Inbound Hidden Service** 🚧
- [ ] Serve via `.onion`
- [ ] Key management (ephemeral/persistent)
- [ ] Transportize overlay for Tor

---

## Milestone 3 — Web3 TLD Infrastructure 🌐
_The decentralized internet layer_
- [ ] `.map` — Decentralized map chunk hosting
- [ ] `.route` — Live traffic data hosting
- [ ] `.sso` — Decentralized profiles & universal SSO
- [ ] `.image` — Image hosting with crypto ownership metadata
- [ ] `.video` — Video hosting with crypto ownership metadata
- [ ] `.web3` — General-purpose websites on the RustyOnions network

---

## Milestone 4 — Web3 Network Build-Out 🚀
_The vision for a token-powered decentralized web_
- [ ] Implement crypto token (likely Solana-based) for bandwidth economy
- [ ] Track bandwidth usage per node (upload/download)
- [ ] Mint tokens for bandwidth providers (nodes, relay operators, chunk hosts)
- [ ] Burn/spend tokens for bandwidth consumers
- [ ] Automatic micropayments to:
  - Site owners (portion of bandwidth fees)
  - Media owners when `.image` or `.video` content is served on other sites
  - Node operators serving public content
- [ ] Universal SSO via `.sso` — profile + crypto address
- [ ] Social media & forums integration:
  - User profiles linked to crypto addresses
  - Posts/comments eligible for revenue share when viewed
- [ ] API for sites to retrieve profile info from `.sso`
- [ ] Payment splitting between content creators and site owners
- [ ] Tools for developers to build `.web3` sites

---

## Milestone 5 — Modularization & Feature Flags 🧱
- [ ] Split into crates: `transport`, `overlay`, `storage`, `messenger`, `accounting`
- [ ] Feature flags for Tor, compression, amnesia mode
- [ ] Config layering (TOML/JSON/env/CLI)

---

## Milestone 6 — Amnesia Mode 🧽
- [ ] RAM-only caches
- [ ] No persistent logs
- [ ] Aggressive purge timers
- [ ] Ephemeral keys

---

## Milestone 7 — Security & Crypto Hygiene 🔐
- [ ] X25519 + ChaCha20-Poly1305 default
- [ ] Replay protection
- [ ] Bounds-checked framing
- [ ] Fuzz testing & input validation
- [ ] Zeroize secrets

---

## Milestone 8 — Discovery & Networking Evolution 📡
- [ ] Peer discovery via optional Kademlia DHT
- [ ] Bootstrap peer lists
- [ ] Configurable seeds

---

## Milestone 9 — DevEx, Docs & CI 🧰
- [ ] GitHub Actions for build/test
- [ ] CONTRIBUTING.md
- [ ] API docs with `cargo doc`
- [ ] `ronode init` config generator

---

## Milestone 10 — Metrics & Stats 📈
- [ ] `ronode stats --json`
- [ ] Metrics endpoint
- [ ] Benchmarks: msg/sec, bytes/sec, latency

---

## Test Plan ✅
- Unit: chunk hashing, crypto round-trips
- Integration: TCP loopback, Tor outbound, HS inbound
- CI: lint, build, test matrix
