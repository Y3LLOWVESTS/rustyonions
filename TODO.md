# RustyOnions – Development Roadmap / TODO

## Milestone 1 – Core Scaffolding ✅
- [x] Workspace + crate structure
- [x] Overlay plane: sled-backed chunk store + TCP listener
- [x] Small-message transport trait + dev TCP implementation
- [x] Bandwidth metering (CountingStream)
- [x] CLI commands: overlayput/get, msgsend, stats, relay stub

---

## Milestone 2 – Tor / Arti Integration
- [ ] Add `ro-arti` crate implementing `SmallMsgTransport` using Arti
- [ ] Hidden service inbox per user
- [ ] Onion dial for private messages
- [ ] Wrap Arti streams with CountingStream for bandwidth stats
- [ ] Config for HS data dir, bootstrap cache

---

## Milestone 3 – E2E Encryption for Messages
- [ ] Implement Noise protocol (XX or IK) or libsodium sealed boxes
- [ ] Public key exchange over secure channel
- [ ] Encrypt all payloads before sending over Tor

---

## Milestone 4 – Relay Contribution System
- [ ] Tor relay helper process (middle relay)
- [ ] Apply dynamic bandwidth caps = 2× recent Tor usage
- [ ] CLI commands: relay start/stop/status
- [ ] Configurable contribution ratio

---

## Milestone 5 – Overlay Enhancements
- [ ] Content-addressable chunk storage with BLAKE3 hashes
- [ ] Replication factor enforcement
- [ ] Anti-correlation placement (avoid same /16 or ASN if possible)
- [ ] Manifest system for multi-chunk objects

---

## Milestone 6 – UX / CLI Improvements
- [ ] Pretty table output for `stats`
- [ ] Verbose logging toggle
- [ ] Config editor command

---

## Milestone 7 – Security & Testing
- [ ] Fuzz test message parser
- [ ] Integration tests for overlay + Tor transport
- [ ] Simulated multi-node environment for load testing

---

**Legend**  
✅ = complete  
[ ] = not started
