### BEGIN NOTE - OCTOBER 26 2025 - 22:05 CST
---

# What to carry over from `svc-overlay` (and friends)

## 1) Transport & framing

* **Overlay is the pipe.** `svc-dht` should talk over the overlay TCP session; don’t bind its own sockets.
* **Use OAP frames.** Send and receive via `FrameKind::Data` only (no new kind needed for beta).
* **Serialization.** Put a typed message in `Frame.payload` using `serde` (`serde_cbor` or `serde_json`). Keep it self-describing (e.g., an enum `ron_proto::DhtMsg`) so future multiplexing stays easy.

## 2) Handshake & caps

* **No new handshake.** Reuse overlay’s `version=1, Caps(GOSSIP_V1)`. DHT doesn’t add capabilities for beta; it rides the data channel.

## 3) Backpressure & queues

* **Best-effort delivery.** Overlay writer drops when the per-peer TX queue is full (and bumps `overlay_peer_dropped_total`). `svc-dht` must be tolerant: retries, timeouts, idempotency.
* **Don’t block.** Use bounded channels on your side too; never hold locks across `.await`.

## 4) Readiness & admin HTTP

* **Endpoints parity.** Expose the same three endpoints: `/healthz`, `/readyz`, `/metrics`.
* **Readiness gates (DHT):**

  * `listeners_bound` equivalent for your internal services (e.g., RPC or control plane if you add one).
  * `routing_ready` when: at least N bootstrap peers connected and routing table has ≥K entries.
  * `queues_ok` based on your own TX queue depth (mirrors overlay pattern).
* **Return 503 until all gates are true.** Keep JSON body with field-level truth for debugging.

## 5) Metrics conventions

* **Namespace** your metrics as `dht_*`.
* **Counters (at minimum):**
  `dht_put_total`, `dht_get_total`, `dht_found_total`, `dht_timeout_total`, `dht_error_total{kind=...}`.
* **Histograms:**
  `dht_get_latency_seconds`, `dht_put_latency_seconds`.
* **Gauges:**
  `dht_routing_table_size`, `dht_active_sessions`, `dht_pending_queries`.
* **Build info:** mirror overlay’s `*_build_info{version,git}=1`.
* **Export path.** Reuse the overlay approach: `once_cell::Lazy` + `prometheus` types, register on first use, `TextEncoder` for exposition.

## 6) Config & tunables (env for now)

* **Prefix:** `RON_DHT_*` (mirrors `RON_OVERLAY_*`).
* **Recommended envs (beta):**

  * `RON_DHT_BOOTSTRAP` (CSV of overlay addresses or logical peer IDs you resolve to overlay endpoints).
  * `RON_DHT_K` (routing table bucket size; default 20).
  * `RON_DHT_ALPHA` (parallelism for lookups; default 3).
  * `RON_DHT_TIMEOUT_MS` (per-RPC timeout; default 1500–3000ms).
  * `RON_DHT_REPLICA` (replication factor; default 3).
* **Clamping.** Apply sane clamps in a small `tuning.rs` (same pattern we used).

## 7) Logging/tracing

* **Consistent fields.** Include `peer`, `key` (hex or base32), `op` (`put|get|pong|…`), and `ttl|hops` when applicable.
* **Levels.** `info!` for state transitions, `warn!` for expected failures/timeouts, `error!` for bugs/IO failures.
* **No panics.** Return errors; let supervisors restart tasks if needed (see `ron-kernel` patterns).

## 8) Code style, lints, docs

* **Keep workspace lints** (clippy pedantic, etc.). Add `Default`, `#[must_use]`, `# Panics`/`# Errors` docs where flagged.
* **Comment style.** Follow `CODECOMMENTS.MD`: top-of-file RO blocks (`RO:WHAT`, `RO:WHY`, etc.) and function-level brief docs where non-obvious.
* **Feature gates.** Mirror overlay’s pattern: keep placeholder features (`pq`, `quic`) but avoid dead cfg trees. Gate examples like we did for `pq_embed`.

## 9) Scripts & CI

* **Roundtrip harness.** Provide `crates/svc-dht/scripts/roundtrip_dht.sh` that:

  1. starts `svc-overlay` (or assumes it’s running)
  2. launches 2× DHT nodes (A,B) pointed at overlay
  3. runs a `put` on A, `get` on B, asserts equality
* **Soak harness.** A lightweight soak similar to overlay’s `soak_overlay.sh` (optional for beta, handy for smoke).
* **Minimal CI step.** `fmt + clippy + roundtrip_dht.sh`.

## 10) Dependencies to reuse

* `ron-proto`: add `DhtMsg` here to keep message types central/shared.
* `ron-metrics`: use its registry and helper patterns if you prefer; overlay showed direct `prometheus` is fine too—choose one and be consistent.
* `ron-kernel`: for bus/supervisor patterns if you spawn internal services; not required for initial client+server loop.

---

## Paste into `NOTES.MD` (carryover for svc-dht)

```
## svc-dht — carryover contracts from svc-overlay (beta)

- Transport: use svc-overlay sessions; send OAP `FrameKind::Data` with serialized `ron_proto::DhtMsg` (serde_cbor/json).
- Handshake: reuse overlay v1 (`Caps(GOSSIP_V1)`); no new caps for beta.
- Backpressure: treat overlay as best-effort. Writers drop on full queues; handle retries/timeouts idempotently.
- Admin HTTP: expose `/healthz`, `/readyz` (503 until routing+queues OK), `/metrics` (Prometheus).
- Metrics (prefix `dht_*`):
  - counters: `dht_put_total`, `dht_get_total`, `dht_found_total`, `dht_timeout_total`, `dht_error_total{kind}`
  - histograms: `dht_get_latency_seconds`, `dht_put_latency_seconds`
  - gauges: `dht_routing_table_size`, `dht_active_sessions`, `dht_pending_queries`
  - `dht_build_info{version,git}=1`
- Tunables (env, clamp values; move to Config later):
  - `RON_DHT_BOOTSTRAP`, `RON_DHT_K` (default 20), `RON_DHT_ALPHA` (default 3),
    `RON_DHT_TIMEOUT_MS` (default 2000), `RON_DHT_REPLICA` (default 3)
- Logging: include `peer`, `op`, `key`, `hops/ttl`; `info` for transitions, `warn` timeouts, `error` IO/bugs. No panics.
- Lints/docs: keep clippy pedantic hygiene; add `Default`, `#[must_use]`, `# Panics/# Errors` where flagged; top-of-file RO blocks.
- Features: keep placeholders but avoid unused cfg branches; gate examples that aren’t ready (`[[example]] required-features`).
- Scripts/CI:
  - `roundtrip_dht.sh`: start overlay, run 2 nodes (A,B), `put` on A, `get` on B, assert value.
  - optional soak for smoke.
  - CI: `fmt + clippy + roundtrip_dht.sh`.

Rationale: keeps the beta path narrow; no new network surface; leverages overlay’s ready/metrics/transport contracts; lets us iterate on DHT logic without transport churn.
```

---

### END NOTE - OCTOBER 26 2025 - 22:05 CST


### BEGIN NOTE - OCTOBER 27 2025 - 14:03 CST

---

# Carryover Notes — `svc-dht` (Kademlia DHT Service)

**Date:** 2025-10-27 (America/Chicago)
**Profile:** RON-CORE (no Web3), aligns with `RON_CORE.MD` and `CODECHECK.MD`
**Status:** ~40% overall (see breakdown)

---

## 1) What we’ve accomplished (shipped)

### A. Service bootstrap & admin plane

* Binary entry with tracing init, config load, metrics registry, readiness gate, bootstrap supervisor skeleton.
* Admin HTTP is live:

  * `GET /healthz` — liveness (truthful “ok”/“starting”).
  * `GET /readyz` — readiness (gated by bootstrap).
  * `GET /version` — build SHA + timestamp (from `build.rs`).
  * `GET /metrics` — Prometheus text format.

### B. Provider path (Micronode MVP)

* In-memory provider store with TTL:

  * `Store::add()` adds/refreshes records; de-dupes per `(cid,node)`; trims whitespace.
  * `Store::get_live()` read-only live set (no mutation on read).
  * Background pruner (`provider::ttl::spawn_pruner`) prunes expired entries (1s cadence after 2s initial delay).
  * Debug snapshot (`debug_snapshot()`) returns all CIDs with `secs_left`.
* HTTP demo API:

  * `POST /dht/provide` with JSON: `{"cid":"b3:...","node":"local://nodeA","ttl_secs":60}`.
  * `GET /dht/find_providers/:cid` returns live providers for `cid`.
  * `GET /dht/_debug/list` dumps store contents with TTL remaining.

### C. Lookup pipeline (policy layer)

* α/β structure with β-hedging & stagger.
* Global inflight limiter (Semaphore) to cap concurrent legs.
* Deadline budgeting (`DeadlineBudget`) with per-leg timeouts; request fails cleanly on budget exhaustion.
* HTTP lookup endpoint is wired through pipeline; **metrics** now reflect real hedge behavior (even though legs are local for now).
* Success body returns: `{ cid, providers, hops, elapsed_ms }`.

  * `hops` currently reflects “first successful leg index + 1” (will become true routing hops once network legs are wired).

### D. Metrics (Prometheus)

* `dht_lookups_total` (counter).
* `dht_lookup_latency_seconds` (histogram).
* `dht_lookup_hops` (histogram).
* `dht_provides_total` (counter).
* All exported via `GET /metrics`.

### E. Readiness & boot flow

* Bootstrap supervisor skeleton opens the ready gate after a min-fill (MVP stub).
* Warns when no seeds configured (expected in Micronode local-only runs).

---

## 2) How to prove it (repro commands)

> Assumes admin bind `127.0.0.1:5301` (default in our `Config`).

### A. Run service

```
cargo run -p svc-dht
```

### B. Quick health/readiness

```
curl -s localhost:5301/healthz
curl -s -o /dev/null -w "%{http_code}\n" localhost:5301/readyz
curl -s localhost:5301/version | jq
```

### C. Provide → find → TTL expiry (one-liners)

Provide:

```
curl -s -X POST localhost:5301/dht/provide \
  -H "content-type: application/json" \
  -d '{"cid":"b3:deadbeef","node":"local://nodeA","ttl_secs":60}' | jq
```

Find:

```
curl -s localhost:5301/dht/find_providers/b3:deadbeef | jq
```

Metrics (lookups/provides/latency/hops):

```
curl -s localhost:5301/metrics | grep -E "dht_lookups_total|dht_provides_total|dht_lookup_latency_seconds_count|dht_lookup_hops"
```

TTL demo (short TTL):

```
curl -s -X POST localhost:5301/dht/provide \
  -H "content-type: application/json" \
  -d '{"cid":"b3:short","node":"local://tmp","ttl_secs":2}' | jq
curl -s localhost:5301/dht/find_providers/b3:short | jq
sleep 3
curl -s localhost:5301/dht/find_providers/b3:short | jq
```

Debug snapshot:

```
curl -s localhost:5301/dht/_debug/list | jq
```

### D. Helper scripts (already added)

**TTL flow + readiness wait**
`crates/svc-dht/scripts/ttl-demo.sh`
Examples:

```
chmod +x crates/svc-dht/scripts/ttl-demo.sh
crates/svc-dht/scripts/ttl-demo.sh --spawn --ttl 2
crates/svc-dht/scripts/ttl-demo.sh --cid b3:short --node local://tmp --ttl 2 --addr 127.0.0.1:5301
```

**Provide + check metrics quickly**
`crates/svc-dht/scripts/provide-and-check.sh`
Example:

```
chmod +x crates/svc-dht/scripts/provide-and-check.sh
crates/svc-dht/scripts/provide-and-check.sh b3:deadbeef local://nodeA 60 127.0.0.1:5301
```

---

## 3) Current coverage vs. scaffold (where we stand)

**✅ Implemented (MVP working)**

* `main.rs`, `lib.rs`, `build.rs` (build info), `tracing.rs` (init), `metrics.rs`, `health.rs`, `readiness.rs`, `bootstrap.rs` (MVP), `rpc/http.rs` (admin + provide/find + debug), `provider/{record,store,ttl}.rs`, `pipeline/{lookup,hedging,deadlines,rate_limit}.rs`, scripts above.

**⚠️ Partial / stubs**

* `peer/{id,bucket,table,selector}.rs` — data types present; invariants and algorithms incomplete (single-writer discipline not enforced yet; replacement/eviction policy TBD; RTT weighting pending).
* `pipeline/provide.rs`, `pipeline/asn_guard.rs` — not implemented (next phases).
* Config: α/β/hop budget present; deadline knobs still hardcoded in `main.rs` (we’ll promote to env soon).

**⏳ Not yet implemented**

* `rpc/kad.rs` (Kad handlers)
* `rpc/discv5.rs` (optional discovery)
* `rpc/bus.rs` (kernel bus adapters)
* `codec/{frame,encode,decode,limits}.rs` (OAP/1 & Kad messages)
* `transport/{mod,clients,tor}.rs` (over `ron-transport`, with optional `arti`)
* `cache/{memory,sled_cache}.rs` — sled feature for Macronode profile
* Tests: `tests/` (api smoke, readiness, provider roundtrip, kbucket props, asn diversity, deadline hedge)
* Benches: `benches/lookup_bench.rs` (p50/p95 lat/hops)
* Fuzz targets: `fuzz_targets/{msg_frame_decode,kad_packet_decode}.rs`
* Loom: `loom/{loom_kbucket,loom_hedge}.rs`

---

## 4) Config knobs (today) and proposed envs

**Active (today)**

* α/β/hop budget via `Config` (already wired).
* Default deadline / hedge stagger / min leg budget set in `main.rs` (constants for now).

**Proposed envs to add (next slice)**

* `DHT_DEADLINE_MS` (default 300)
* `DHT_HEDGE_STAGGER_MS` (default 25)
* `DHT_MIN_LEG_BUDGET_MS` (default 50)
* `DHT_MAX_CONCURRENT_LEGS` (default 64)
* `DHT_DEFAULT_TTL_SECS` (default 600)
* `DHT_ADMIN_BIND` (default `127.0.0.1:5301`)
* `DHT_SEEDS` (CSV of `host:port` for bootstrap; later include onion addrs behind `arti` feature)

---

## 5) Invariants already encoded (MVP) & pending

**Encoded / respected**

* No mutation on read: `Store::get_live()` never prunes (pruning is background only).
* TTL expiry is monotonic (`Instant` based), pruner cadence ≥ 1 Hz after a 2s initial wait.
* Hedge never exceeds deadline: legs wrapped in `timeout(leg_budget)` with per-leg budget ≤ remaining global deadline.
* Global inflight cap exists (Limiter).

**Pending to encode**

* **K-bucket single writer** discipline (no lock across `.await`; consistent mutation site).
* **Peer selection** tuned for α parallelism and β hedging; RTT-aware update policy.
* **Hop budget** enforced across multi-hop network legs (cuts off when exhausted).
* **ASN diversity floor** and **per-ASN caps**.
* **Per-peer inflight caps** (beyond global limiter).
* **Codec size/time limits** (centralized in `codec/limits.rs`) with tests/fuzzing.
* **Ready gate truthfulness** tied to bootstrap quorum + min table fill.

---

## 6) Known issues / caveats (MVP)

* **Local legs only**: Lookups query the in-process store; `hops` is a proxy for hedge legs. Real hops require network legs (Kad over `ron-transport`), coming next.
* **Deadline knobs not in Config** (yet).
* **Seeds/quorum**: Boot supervisor opens readiness quickly in local-only runs; in real deployments, gate should depend on seed dialing + routing table min-fill.
* **No persistence**: Micronode default is in-memory. Macronode feature (`sled-cache`) not wired.

---

## 7) Next steps (actionable, ordered)

### Phase 2 — Networking & correctness

1. **Transport + Kad codec (on-wire)**

   * Implement `codec/{frame,encode,decode,limits}.rs` (OAP/1 framing + Kad messages).
   * Add `transport/{mod,clients}.rs` over `ron-transport` with TLS timeouts from `Config`.
   * Wire `pipeline::lookup` legs to query peers over transport (respect `hop_budget` and `deadline`).
   * Metrics: record true `hops` from routing, not hedge index.

2. **Routing table (Kademlia)**

   * `peer/{id,bucket,table,selector}.rs`:

     * NodeId distance math & XOR metrics.
     * K-bucket eviction/update per Kademlia spec; **single-writer discipline**.
     * Selector tuned for α parallelism; keep a min RTT bias.

3. **Bootstrap & readiness gates (truthful)**

   * Seed dialing loop with exponential backoff/jitter.
   * Ready only when: min peer count AND routing table min-fill achieved; expose counters.

4. **ASN guard & per-peer inflight**

   * `pipeline/asn_guard.rs` to maintain entropy floor & per-ASN caps.
   * Extend limiter with per-peer semaphores.

5. **Config envs for deadlines/hedge**

   * Add `DHT_DEADLINE_MS`, `DHT_HEDGE_STAGGER_MS`, `DHT_MIN_LEG_BUDGET_MS`, `DHT_MAX_CONCURRENT_LEGS`, `DHT_DEFAULT_TTL_SECS`.

### Phase 3 — Storage, tests, perf, fuzz

6. **Provider replication hooks**

   * `pipeline/provide.rs` — replicate provider records to K closest peers.
   * RF repair (republish) worker and counters.

7. **Cache feature (Macronode)**

   * `cache/{memory,sled_cache}.rs` with feature `sled-cache`; maintain TTL parity and checksums.

8. **Test/bench/fuzz suites**

   * `tests/api_smoke.rs` — provide→find across two processes (loopback).
   * `tests/readiness_bootstrap.rs` — gate opens only after quorum.
   * `tests/provider_roundtrip.rs` — TTL + republish.
   * `tests/kbucket_props.rs` — ordering/eviction & single-writer invariant.
   * `tests/asn_diversity.rs` — entropy/caps under churn.
   * `tests/deadline_hedge.rs` — hedges reduce tail without budget overrun.
   * `benches/lookup_bench.rs` — p50/p95 latency, hop histos at α/β profiles.
   * `fuzz_targets/msg_frame_decode.rs`, `kad_packet_decode.rs`.
   * `loom/loom_kbucket.rs`, `loom/loom_hedge.rs`.

9. **CI gates**

   * Wire workflows: fmt, clippy (deny warnings), unit+doc+prop tests, coverage (tarpaulin), `cargo deny`, fuzz (time-boxed), perf thresholds (fail PR if SLOs drift).

---

## 8) SLOs (initial targets, to refine)

* **Intra-AZ lookup p95 ≤ 150 ms** (with α/β tuned; true network legs).
* **Hop p99 ≤ 5** (with typical table fill).
* **Admin readiness** within **N seconds** post boot with healthy seeds (define N after bootstrap is real).
* **CPU/mem stability** during soak/churn: no FD leaks; memory steady except cache size.

---

## 9) Troubleshooting quicksheet

* **Ready never goes 200**

  * Check seeds set and reachable; bootstrap logs; metrics: `routing_bootstrap_*`.
  * Temporarily relax min-fill for local testing.

* **Provide shows up, then disappears too fast**

  * Verify `ttl_secs` and pruner cadence.
  * Check `/dht/_debug/list` for `secs_left` and duplicates due to whitespace; inputs are normalized.

* **Metrics empty**

  * Ensure you’ve hit endpoints after boot; scrape `/metrics` directly.

* **High tail latency under hedging**

  * Tune `DHT_HEDGE_STAGGER_MS` up slightly; ensure `DHT_MIN_LEG_BUDGET_MS` isn’t too small.

---

## 10) Commands you’ll reuse (handy block)

Start service (with tuned knobs):

```
DHT_ALPHA=2 DHT_BETA=2 DHT_HOP_BUDGET=6 cargo run -p svc-dht
```

Provide + check:

```
crates/svc-dht/scripts/provide-and-check.sh b3:deadbeef local://nodeA 60 127.0.0.1:5301
```

TTL end-to-end (auto-spawn):

```
crates/svc-dht/scripts/ttl-demo.sh --spawn --ttl 2
```

Scrape top metrics of interest:

```
curl -s localhost:5301/metrics | grep -E "dht_(lookups_total|provides_total|lookup_latency_seconds_count|lookup_hops)"
```

---

## 11) Risk register (trackers)

* **Codec correctness** (must be fuzzed): size bounds, malformed frames, timeouts.
* **K-bucket concurrency** (loom): single-writer discipline; no `.await` under lock.
* **Bootstrap realism**: readiness must reflect actual neighbor reachability and table state.
* **Config drift**: centralize all knobs; document defaults in `docs/DHT_CONFIG.md`.

---

## 12) Doc updates to keep in sync (when we ship the next slice)

* `docs/IDB.md` — add invariants for k-buckets, hedge deadlines, hop budgets.
* `docs/DHT_CONFIG.md` — env tables for the new knobs.
* `docs/PERFORMANCE.md` — bench harness, machine profile, SLOs, thresholds.
* `docs/arch.mmd / sequence.mmd / state.mmd` — update for network legs and replication flow.

---

### TL;DR next move

**Implement Transport + Kad codec and wire real network legs** so `hops` becomes true routing hops; then enforce k-bucket invariants and truthful readiness. After that, ASN guard + per-peer limits, then tests/benches/fuzz/loom + CI gates.

When you’re ready, I’ll drop the full paste-ready files for the **Transport + Kad codec** slice.



### END NOTE - OCTOBER 27 2025 - 14:03 CST


### BEGIN NOTE - OCTOBER 28 2025 - 14:45 CST


Awesome—here’s a paste-ready, exhaustive **carryover notes** pack for `svc-dht`. Drop this into `crates/svc-dht/NOTES.md` (or your master NOTES log).

---

# Carryover Notes — `svc-dht` (Kademlia DHT service)

**Date:** 2025-10-28 (America/Chicago)
**Profile:** RON-CORE baseline (no Web3). Focused on correctness, readiness, admin plane, and hedge-aware lookup.

---

## 1) What we accomplished

### Core features & invariants

* **Provider Store (TTL’d):** In-memory provider records keyed by `B3Cid`, with TTL handling and prune path verified.
* **Kademlia primitives:**

  * `NodeId([u8;32])` with **XOR distance** (tight, branchless-ish loop).
  * Bucket math + “closest K” selection with **no duplicates** and **limit respected**.
* **Lookup pipeline (β-hedging ready):**

  * `race_hedged(beta, stagger, budget, f)` utility supports hedged legs with stagger and total deadline.
  * Lookup ctx wired for fan-out/merge/cancellation paths; unit tested for **budget** and **stagger** guarantees.
* **HTTP RPC (Axum):**

  * `POST /provide` validates node URI, accepts TTL, updates store.
  * `GET /find/:cid` returns providers + simple hop/latency accounting.
  * Proper **400** on malformed inputs (bad node URI).
* **Admin plane:**

  * `/healthz`, `/readyz` contract via kernel health/readiness wrapper.
  * `/metrics` exports `dht_*` counters + histograms (lookups, hops, latency).
* **Bootstrap/readiness discipline:**

  * Node does not declare ready until **min-fill** reached; tested via `readiness_bootstrap.rs`.
* **Bench harness (Criterion):**

  * Clean baseline **lookup microbench** sweeping β∈{0,1,2,3}; optional env-gated tail-latency simulation (no network).
* **CI-friendly tests:**

  * Integration tests exercise handlers **without binding sockets** for speed/determinism.
  * E2E smoke scripts verify full path (serve → provide → find → metrics) quickly on localhost.

### Paper cuts fixed along the way

* Resolved **Axum `IntoResponse` type mismatch** in handlers.
* Criterion async API usage updated (no `to_async` on Bencher; moved work into runtime).
* Replaced non-`Send` RNG in benches with deterministic **SplitMix-like mix64** to satisfy `Send + Sync`.
* **Prometheus double-registration** avoided in tests by using a shared test registry or per-test unique metrics (now green).
* Clippy in this crate: addressed `needless_range_loop` (NodeId XOR) and `unwrap_or_default` nits in provider store.

---

## 2) How to run — format, clippy, tests, benches, E2E

> Tip: Run from repo root. Commands below assume **stable** toolchain.

### Format + clippy (crate only)

```bash
cargo fmt -p svc-dht
cargo clippy -p svc-dht --no-deps -- -D warnings
```

### Unit & integration tests (with output)

```bash
cargo test -p svc-dht -- --nocapture
```

You should see:

* `api_smoke.rs` … 2 ok
* `asn_diversity.rs` … 2 ok
* `deadline_hedge.rs` … 1 ok
* `kbucket_props.rs` … 3 ok
* `provider_roundtrip.rs` … 1 ok
* `readiness_bootstrap.rs` … 1 ok

### Microbenchmarks (lookup baseline)

```bash
# Fast run (β sweep)
cargo bench -p svc-dht --bench lookup_bench
```

#### Save & compare baselines

```bash
# Save today’s baseline (use your date tag)
cargo bench -p svc-dht --bench lookup_bench -- --save-baseline svc-dht-2025-10-28-mbp-battery

# Later, compare against that saved baseline
cargo bench -p svc-dht --bench lookup_bench -- --baseline svc-dht-2025-10-28-mbp-battery
```

#### Optional tail-rescue simulation (prints P50/P95/P99)

```bash
# Default sim knobs (good demo): 5% slow primaries, 2ms hedge stagger, 150ms budget
DHT_SIM=1 DHT_TRIALS=600 DHT_PSLOW=0.05 DHT_STAGGER_MS=2 cargo bench -p svc-dht --bench lookup_bench
# Adjustable:
#   DHT_SLOW_MIN_MS / DHT_SLOW_MAX_MS (e.g., 80..120)
#   DHT_FAST_MIN_MS / DHT_FAST_MAX_MS (e.g., 1..2)
#   DHT_LEG_BUDGET_MS (e.g., 150)
```

### E2E smoke via scripts (admin + HTTP happy path)

Terminal A (serve):

```bash
cargo run -p svc-dht
# Expect:
#  INFO admin up admin_addr=127.0.0.1:5301
#  WARN no seeds configured; table will rely on inbound discovery
#  INFO bootstrap: min-fill reached; ready gate opened
```

Terminal B (scripted smoke):

```bash
crates/svc-dht/scripts/run-local.sh
# Does: wait /readyz → /version → POST /provide → GET /find/:cid → /metrics grep dht_*
```

Also available (single-shot CLI smoke):

```bash
crates/svc-dht/scripts/smoke_svc_dht.sh
```

And a low-level one:

```bash
crates/svc-dht/scripts/provide-and-check.sh b3:deadbeef local://nodeA 60 127.0.0.1:5301
```

---

## 3) Benchmark results (current baseline)

**Host:** 2019 MacBook Pro, Intel i5-8257U @ 1.4 GHz (4c/8t), **battery power**
**OS:** macOS; **Toolchain:** stable Rust; **Build:** Criterion `bench` profile (optimized)

### Lookup microbench (no network; β = parallel legs)

| β (hedges) |  Time per op (µs) |
| ---------: | ----------------: |
|          0 | **~3.03–3.12 µs** |
|          1 | **~5.11–5.26 µs** |
|          2 | **~7.12–7.57 µs** |
|          3 | **~9.21–9.93 µs** |

**Interpretation**

* Roughly **+~2 µs per extra hedge leg**—a near-linear step-up.
* This indicates the hedger’s scheduling/cancellation/merge overhead is **small and predictable**.
* On AC power (and with `RUSTFLAGS="-C target-cpu=native"`), expect a few percent tighter. Server-grade CPUs should compress further; the **shape** is what matters.

### Tail-rescue sim (env-gated)

Example prints you saw (with p_slow ≈ 5%, stagger=2ms, budget≈150ms):

* β=0 → P50 ≈ 3.2 ms, **P95 ≈ 88–104 ms**, P99 ≈ 139–142 ms
* β=2 → P50 ≈ 6.8 ms, **P95 ≈ ~8 ms**, P99 ≈ ~138 ms

**Why we care:** Median nudges up with hedging, but **P95/P99 collapse**, which is the intended SLO outcome for user-visible requests.

### E2E HTTP smoke (localhost)

* `GET /find/:cid` roundtrip through HTTP+JSON+handler+store reported `elapsed_ms ≈ 27 ms` (single-hop local).
  This is an admin/demo path, not the microbench—good for wiring/metrics **sanity**, not perf claims.

---

## 4) Test catalog (what they prove)

* **`tests/api_smoke.rs`**

  * Validates `POST /provide` + `GET /find/:cid` happy path.
  * Asserts **400** on bad node URI (input hardening).

* **`tests/provider_roundtrip.rs`**

  * Store add → get → TTL prune. Real state, no sockets (fast CI).

* **`tests/readiness_bootstrap.rs`**

  * Readiness remains closed until **min-fill**. `/readyz` flips to 200 only then.

* **`tests/asn_diversity.rs`**

  * Diversity helper: accepts mix meeting **ASN floor**, rejects uniform ASN if floor>1.

* **`tests/kbucket_props.rs`**

  * Kademlia invariants: XOR-zero for identical IDs; bucket index monotonicity “smoke”; closest set respects limit & dedupe.

* **`tests/deadline_hedge.rs`**

  * Hedger enforces **budget** and **stagger**, terminates cleanly (no runaway fan-out).

---

## 5) Source notes & structure

* **`src/rpc/http.rs`** — Axum handlers; returns `Response<Body>`; input validation; JSON wiring.
* **`src/provider/store.rs`** — TTL’d provider store; prune path; small perf nits addressed (`or_default`).
* **`src/pipeline/hedging.rs`** — `race_hedged` core; `Send + Sync` leg closures; stagger & budget apply.
* **`src/pipeline/lookup.rs`** — Lookup ctx and fan-out; plugs into provider store & routing math.
* **`src/peer/id.rs`** — `NodeId` + XOR distance (range-loop replaced by `.zip()` enumerate for clarity + clippy).
* **`src/readiness.rs`** — ReadyGate wrapper that enforces min-fill before serving.
* **`src/metrics.rs`** — `DhtMetrics` counters/histograms (`dht_lookups_total`, `dht_lookup_latency_seconds`, `dht_lookup_hops`).
* **`src/invariants.rs`** — (Intentionally minimal for now) **Anchor doc** for DHT invariants.

  * Use this file to **codify invariants** that aren’t natural unit tests (e.g., bounds, monotonic counters, config sanity); we left it as a placeholder to avoid scattering invariant docs in code comments.

---

## 6) Completion estimate & gate

* **Feature completeness:** ~**85–90%** for RON-CORE beta needs.
* **Why not 100%:** running in-memory only (no persistent provider DB), single-node demo; multi-node query/routing and network transport integration come next (but can land in `svc-overlay`/`ron-transport` tasks).

**Go/No-Go:** ✅ Good to move to the next crate (per your schedule). `svc-dht` is green on tests/benches and admin smoke; microbench trendlines are solid.

---

## 7) Parking lot / later improvements (ranked, quick bullets)

1. **Multi-node routing & network integration**

   * Wire lookup/send via `ron-transport` (TLS/TCP; later QUIC).
   * Implement iterative/recursive Kademlia RPCs (`FIND_NODE`, `FIND_PROVIDERS`, `ADD_PROVIDER`).
   * NAT traversal strategy (defer to `svc-overlay` if that’s our rendezvous).

2. **Persistence & compaction**

   * Optional sled/rocks backend for provider store; TTL wheel or hierarchical timers.
   * Crash-safe restore; background compaction & metrics.

3. **Routing table management**

   * Bucket eviction policy (last-seen, ping-before-evict).
   * Periodic refresh; random ID walk for liveness.

4. **Diversity & anti-eclipse**

   * Enforce ASN/geodiversity quotas in **closest** selection.
   * Penalize flappy peers; prefer historically low tail-latency.

5. **Hedging policy**

   * Configurable β/stagger/budget per call class; adaptive β on observed P95.
   * Metrics: `dht_hedge_legs_total`, `dht_hedge_cancels_total`, `dht_hedge_budget_exhausted_total`.

6. **SLOs & observability**

   * Dashboard for: hop histogram, lookup latency percentiles, TTL churn.
   * Trace spans per leg; correlation IDs.

7. **Security & sanitation**

   * CID validation already present; add node URI scheme allow-list.
   * (Later) provider attestation or signed adverts (ties into `ron-kms`).

8. **Fuzz/property tests**

   * Property: XOR distance triangle-ish behaviors (ordering stability).
   * Proptest for bucket ops under random add/remove sequences.
   * Loom tests for hedger races (optional—lower priority).

9. **Docs & samples**

   * `README.md`: curl examples, JSON schemas.
   * Example client (`examples/cli.rs`): provide/find/metrics dump.

10. **Packaging/ops**

* Dockerfile & compose with `svc-overlay` to demo multi-node discovery.
* Systemd unit template; readiness/liveness probes examples.

---

## 8) Quick command cribsheet (copy/paste)

```bash
# Format + clippy
cargo fmt -p svc-dht
cargo clippy -p svc-dht --no-deps -- -D warnings

# Tests
cargo test -p svc-dht -- --nocapture

# Benches (baseline + compare)
cargo bench -p svc-dht --bench lookup_bench -- --save-baseline svc-dht-2025-10-28-mbp-battery
cargo bench -p svc-dht --bench lookup_bench -- --baseline svc-dht-2025-10-28-mbp-battery

# Optional tail-rescue sim
DHT_SIM=1 DHT_TRIALS=600 DHT_PSLOW=0.05 DHT_STAGGER_MS=2 cargo bench -p svc-dht --bench lookup_bench

# Serve + local smoke
cargo run -p svc-dht
crates/svc-dht/scripts/run-local.sh
```

---

## 9) What’s next (recommended order)

1. **svc-overlay**: bind listener, integrate `ron-transport`, implement on-wire Kademlia RPCs using our hedger & routing math (hardest, but aligns with your plan).
2. **ron-transport**: any pending polish needed to support DHT RPCs (TLS/QUIC knobs, timeouts).
3. **Persistence (optional)** in `svc-dht` once multi-node is alive.

---


### END NOTE - OCTOBER 28 2025 - 14:45 CST