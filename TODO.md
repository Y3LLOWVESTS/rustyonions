
---

### `TODO.md`
```markdown
# RustyOnions — Roadmap & TODO
_Last updated: 2025-08-16_

This TODO replaces the prior roadmap to reflect our current, script-driven validation flow and the recent refactors. (Supersedes the previous TODO.) :contentReference[oaicite:3]{index=3}

---

## Progress Snapshot

- ✅ `testing/test_tcp.sh` — local TCP overlay smoke test **working**
- ✅ `testing/test_tor.sh` — Tor bootstrap smoke test **working** (client only; bridges supported)
- 🧪 Node over Tor — **partial** (hidden service and full e2e still in progress)
- 🔧 Refactor — **ongoing**, generalized pass queued as the next step

---

## Milestones

### Milestone 1 — Local TCP Overlay ✅
- [x] Sled-backed storage and simple redundancy
- [x] CLI path for overlay PUT/GET
- [x] **Script**: `test_tcp.sh` validates a local round-trip
- [x] Basic metrics (local)

### Milestone 2 — Tor/Transport Bring-Up (Phase 1: Client) ✅
- [x] Isolated Tor bootstrap with control-port monitoring
- [x] Bridges (obfs4 and snowflake) supported in script
- [x] Stall diagnostics, cleanup trap, auto-port option
- [x] **Script**: `test_tor.sh` published and validated on macOS

**Phase 2: Hidden Service + Node e2e** 🚧
- [ ] Node serves via `.onion` (ephemeral or persistent keys)
- [ ] Node client PUT/GET over Tor hidden service
- [ ] Control-auth compatibility: cookie vs. no-auth mode
- [ ] Scripted **RUN_NODE_E2E=1** mode (see Tasks below)

### Milestone 3 — Web3 TLD Scaffolds 🌐
*(Speculative; can parallelize once Milestone 2 is green)*  
- [ ] TLD registry prototypes: `.map`, `.route`, `.sso`, `.image`, `.video`, `.web3`
- [ ] Minimal routing rules + ownership metadata draft
- [ ] CLI skeletons for publishing/lookup

### Milestone 4 — Token & Accounting (Design) 🪙
- [ ] Usage metering model (upload/download/retain)
- [ ] Token mint/spend flows (likely Solana or similar)
- [ ] Revenue share splits (site owners, creators, node operators)

### Milestone 5 — Security & Hygiene 🔐
- [ ] Input validation, bounds-checked framing, replay protection
- [ ] Fuzzing harness
- [ ] Zeroize secrets & amnesia-mode hooks

### Milestone 6 — Discovery 📡
- [ ] Optional Kademlia DHT for peer discovery
- [ ] Bootstrap/seed management

---

## Scripts — Tasks & Enhancements

### `testing/test_tcp.sh`
- [ ] Emit structured JSON summary on success/failure (CI-friendly)
- [ ] Add `KEEP_SERVER=1` to leave overlay listener running for manual tests
- [ ] Auto-select a free port if default is busy
- [ ] Save logs under `.tcp_test_logs/<RUN_ID>/`

### `testing/test_tor.sh`
- [x] Cleanup trap; dynamic obfs4 detection; stall diagnostics; `AUTO_PORTS=1`; `KEEP_TOR=1`
- [x] Support **snowflake** (macOS Homebrew provides `snowflake-client`)
- [ ] Auto-detect `snowflake-client` **and** `tor-snowflake` names without user symlink
- [ ] Implement `RUN_NODE_E2E=1`:
  - [ ] Start Tor (respect env knobs)
  - [ ] Launch node `serve --transport tor`
  - [ ] Wait for onion publish
  - [ ] PUT/GET round-trip; print “PUT/GET OK ✅”
  - [ ] Clean shutdown (unless `KEEP_TOR=1`)
- [ ] Make cookie wait time and stall threshold configurable (already partially done; tune defaults after more runs)
- [ ] Optional: leave `DataDirectory` behind for debugging when `KEEP_TOR=1`

---

## Generalized Refactor — Next Step (High Priority)

**Goals**
- Simplify crate boundaries (`overlay`, `transport`, `node`, `accounting`, etc.)  
- Stabilize the transport trait interfaces (TCP, Tor)  
- Isolate Tor specifics (control auth, bootstrap, bridges) behind a clean adapter  
- Reduce binary flags; prefer config (TOML + env overrides)

**Tasks**
- [ ] Define `Transport` trait with clear lifecycle: `init()`, `start()`, `shutdown()`
- [ ] Extract Tor controller into `tor_adapter` module with:
  - [ ] cookie/no-auth modes
  - [ ] bootstrap observer (events, % progress)
  - [ ] bridge configuration object
- [ ] Normalize CLI across transports (`serve`, `put`, `get`, `stats`)
- [ ] Centralize logging + metrics
- [ ] Add integration tests that shell out to `test_tcp.sh`/`test_tor.sh`
- [ ] Remove/rename deprecated modules & unused symbols

---

## Developer Notes

- Prefer **script-driven** validation while refactors land.
- Keep scripts **idempotent** and **CI-friendly** (exit codes, minimal deps).
- When in doubt, **short-circuit to scripts** before diving into app code.

---

## Credits

This roadmap acknowledges contributions from **OpenAI’s ChatGPT** and **xAI’s Grok** alongside Stevan White. Their reviews and generated scaffolds materially improved the testing scripts and transport bring-up flow. :contentReference[oaicite:4]{index=4}
