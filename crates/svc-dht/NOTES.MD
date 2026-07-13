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


### BEGIN NOTE - JULY 10 2026 - 12:55 CST

Grounded in the latest Rust-only codebundle and the accumulated BUILD_PLAN_Z notes. `svc-dht` was the central crate for Phase 7: canonical `crab://node` identity, no raw route/IP provider leakage, safe `/dht/node_identity`, safe `/dht/privacy_status`, and focused privacy tests.    

# BUILD_PLAN_Z — svc-dht Change Notes

## Exhaustive Carryover Notes for `crates/svc-dht`

## 0. Current svc-dht Status

Current crate:

```text id="f80jrw"
crates/svc-dht
```

Current BUILD_PLAN_Z role:

```text id="8jao0e"
DHT/provider discovery surface for CrabLink Service Nodes, with canonical crab://node identity and no public IP/socket route leakage.
```

Current status:

```text id="s7bma0"
svc-dht — GREEN / PARKED through Phase 7
```

Known final Phase 7 status:

```text id="3qqnef"
Phase 7A — svc-dht canonical crab://node provider identity: GREEN
Phase 7B — svc-dht public DTO no-IP-leak reviewer: GREEN
Phase 7C — svc-dht typed service node identity config + safe identity DTO: GREEN
Phase 7D — svc-dht privacy status DTO/public route posture: GREEN
```

Known green checks:

```bash id="yewn0p"
cargo test -p svc-dht --test crab_node_identity
cargo test -p svc-dht --test privacy_no_ip_leak
cargo test -p svc-dht --test node_identity_config
cargo test -p svc-dht --test privacy_status
cargo check -p svc-dht --all-targets
cargo clippy -p svc-dht --all-targets --no-deps -- -D warnings
```

Known final workspace checkpoint:

```bash id="8e8znd"
cargo check --workspace
```

Known result:

```text id="ipj3j1"
passed
```

---

## 1. svc-dht’s Role in BUILD_PLAN_Z

`svc-dht` owns the DHT/provider discovery side of the CrabLink node network.

Its BUILD_PLAN_Z responsibility is:

```text id="p4fgq2"
allow service-node discovery
allow provider lookup/provide workflows
expose admin/demo DHT endpoints
preserve OAP/b3/provider discovery boundaries
use canonical crab://node provider identity
avoid raw IP/socket/provider-route leakage
avoid direct user-to-user residential IP exposure
```

It must not become:

```text id="0ef9a4"
a raw socket directory
a residential IP directory
a Tor/onion directory
a wallet/ledger authority
a reward finality source
a public transport-route leak
```

The simplest current rule:

```text id="h2ni7n"
svc-dht may expose logical CrabLink node identity, but not raw public transport identity.
```

---

## 2. Phase 7A — Canonical `crab://node` Provider Identity

## 2.1 What changed

Phase 7A changed the DHT provider identity model to canonical CrabLink node identity:

```text id="fhme0i"
crab://node/<64 lowercase hex>
```

This replaced/blocked legacy placeholder and transport-specific route schemes.

## 2.2 Canonical public identity

The only valid public node/provider identity shape is:

```text id="29q9ig"
crab://node/<64 lowercase hex>
```

Example:

```text id="hmmf9l"
crab://node/00000000000000000000000000000000000000000000000000000000000000a1
```

## 2.3 Rejected schemes

These must not be valid public provider identities:

```text id="qd57fj"
relay://
onion://
service://
local://
tcp://
udp://
ws://
wss://
http://
https://
```

Also rejected:

```text id="u0lczd"
raw socket routes
raw IP provider addresses
transport-specific route strings
```

## 2.4 Important Tor/onion correction

The user explicitly corrected the architecture:

```text id="8iv55w"
We are not using Tor/onion here.
```

Therefore:

```text id="s2pfy5"
Do not reintroduce onion://.
Do not reintroduce Tor-specific node identity.
Do not treat onion routes as the CrabLink node identity scheme.
```

There may still be older `ron-transport` or `svc-overlay` Tor-related artifacts elsewhere in the wider repo, but BUILD_PLAN_Z Phase 7’s svc-dht public identity is `crab://node`, not onion.

## 2.5 Why this matters

This keeps DHT records from becoming:

```text id="tmenct"
public user IP records
raw socket directories
transport URL registries
residential provider maps
```

Instead, DHT provider identity is logical:

```text id="oy4s4r"
CrabLink node identity -> later resolved/transported through safe service-node mechanisms
```

---

## 3. CrabNodeId / Typed Node Identity

## 3.1 Type-level identity

`svc-dht` now uses typed `CrabNodeId` in important HTTP/config state.

The latest codebundle shows `rpc::http::State` contains:

```rust id="js7pph"
pub node_id: CrabNodeId,
```

and the HTTP module imports:

```rust id="znvbix"
types::{B3Cid, CrabNodeId}
```

The module comments state the invariant:

```text id="k3dgn9"
provider identities are crab://node/<64-lowercase-hex>
```

This is directly in `crates/svc-dht/src/rpc/http.rs`. 

## 3.2 Default/fallback test node id

The HTTP state constructor has a default/test node id path:

```text id="at9jgh"
CrabNodeId::from_bytes([0x01; 32])
```

This is used when building a default state for tests/demo endpoints.

## 3.3 Explicit node id constructor

`State::new_with_node_id(...)` was added/used so tests and config can provide a real canonical service-node identity.

Important behavior:

```text id="fqh1eu"
state carries typed CrabNodeId
node_identity endpoint derives node URI from typed id
privacy_status endpoint derives node URI from typed id
provider identity shape is not an arbitrary string
```

---

## 4. Phase 7B — Public DTO No-IP-Leak Reviewer

## 4.1 What changed

Added a privacy review helper for public JSON DTOs:

```text id="mk73k6"
crates/svc-dht/src/privacy.rs
```

Purpose:

```text id="2j4k84"
detect public DTOs that accidentally expose IP fields, socket fields, raw routes, or forbidden schemes
```

## 4.2 What the reviewer catches

The reviewer flags public field names such as:

```text id="4a2e28"
source_ip
viewer_ip
creator_ip
residential_ip
lan_ip
peer_ip
client_ip
remote_ip
remote_addr
socket_addr
ip_addr
```

It also flags forbidden route schemes such as:

```text id="089sxt"
http://
https://
tcp://
udp://
ws://
wss://
local://
relay://
onion://
service://
```

It also catches IPv4 literal leaks in JSON string values, such as:

```text id="asrnx8"
192.168.1.10
10.0.0.8
127.0.0.1 when exposed as a public route identity
```

## 4.3 Allowed identity

The reviewer accepts the canonical logical identity:

```text id="ixlgdm"
crab://node/<64 lowercase hex>
```

## 4.4 Why this matters

This test utility gives us a reusable fail-closed privacy gate for public DHT DTOs.

Future DHT/provider endpoints should be passed through this reviewer in tests so we do not accidentally expose:

```text id="4sylyh"
user IP
peer IP
admin bind
socket address
transport URL
raw service route
```

## 4.5 Tests added

Test file:

```text id="ergv3e"
crates/svc-dht/tests/privacy_no_ip_leak.rs
```

Covered tests:

```text id="1328jy"
find_and_debug_outputs_do_not_expose_ip_or_socket_fields
privacy_reviewer_accepts_crab_node_identity_json
privacy_reviewer_flags_ip_fields_and_raw_transport_routes
```

Known green result:

```text id="801uel"
privacy_no_ip_leak: 3 passed
```

---

## 5. Phase 7C — Typed Service Node Identity Config

## 5.1 What changed

`svc-dht` config now supports a typed DHT node URI.

Primary file:

```text id="5fe7c4"
crates/svc-dht/src/config.rs
```

Important environment variable:

```text id="qqybad"
DHT_NODE_URI
```

Compatibility fallback:

```text id="b2dp72"
RON_DHT_NODE_URI
```

Allowed value:

```text id="36z4p0"
crab://node/<64 lowercase hex>
```

Bad values fail closed.

Example rejected value from the notes:

```text id="5af1a5"
tcp://192.168.1.10:7000
```

## 5.2 Env keys protected in tests

The test harness clears/restores relevant env vars, including:

```text id="5b3fyd"
DHT_ADMIN_BIND
DHT_NODE_URI
RON_DHT_NODE_URI
DHT_ALPHA
DHT_BETA
DHT_K
DHT_HOP_BUDGET
DHT_DIAL_TIMEOUT_MS
DHT_IDLE_TIMEOUT_MS
DHT_SEEDS
RON_AMNESIA
```

The newer codebundle shows this in `node_identity_config.rs`. 

## 5.3 `Config::from_env()` behavior

Expected behavior:

```text id="otk2r9"
DHT_NODE_URI set to canonical crab://node URI -> accepted
DHT_NODE_URI set to raw transport URI -> rejected
RON_DHT_NODE_URI fallback -> accepted only if canonical
node_id parsed into CrabNodeId
node_uri() returns canonical crab://node string
```

## 5.4 Tests added

Test file:

```text id="34j1kh"
crates/svc-dht/tests/node_identity_config.rs
```

Test header from the codebundle:

```text id="cuvnbr"
Phase 7C service node identity config/DTO tests.
svc-dht must expose only canonical crab://node identity, never raw routes/IPs.
bad DHT_NODE_URI fails closed before service startup.
```

The codebundle confirms this test imports:

```text id="3rqksx"
Config::from_env
rpc::http::node_identity
privacy::review_public_json
CrabNodeId
```

and defines canonical `NODE_A_URI` / `NODE_A_HEX`. 

Covered tests:

```text id="b16b66"
config_from_env_accepts_canonical_dht_node_uri
config_from_env_rejects_legacy_or_transport_node_uri
node_identity_dto_exposes_only_crab_node_identity
```

Known green result:

```text id="qifid6"
node_identity_config: 3 passed
```

---

## 6. Phase 7C — Safe `/dht/node_identity` Endpoint

## 6.1 Endpoint added

```text id="hr1oa8"
GET /dht/node_identity
```

Primary file:

```text id="d8b7z9"
crates/svc-dht/src/rpc/http.rs
```

## 6.2 Safe DTO fields

Expected response shape:

```json id="khk47k"
{
  "node": "crab://node/<64 lowercase hex>",
  "node_id_hex": "<64 lowercase hex>",
  "identity_scheme": "crab://node",
  "provider_identity_kind": "crab_node",
  "public_inbound": false
}
```

## 6.3 Important exclusions

This endpoint must not include:

```text id="9ibjsw"
ip
ip_addr
remote_addr
socket_addr
admin_bind
http_addr
metrics_addr
tcp_url
udp_url
http_url
transport_url
onion_url
```

## 6.4 Privacy reviewer coverage

The `node_identity_config` test calls:

```text id="hd4pzg"
review_public_json
```

to prove the DTO is privacy-clean.

## 6.5 Meaning

This endpoint gives operators and tests a canonical logical identity for the DHT service node without exposing transport details.

---

## 7. Phase 7D — Safe `/dht/privacy_status` Endpoint

## 7.1 Endpoint added

```text id="plekb4"
GET /dht/privacy_status
```

Primary file:

```text id="eag0uo"
crates/svc-dht/src/rpc/http.rs
```

## 7.2 Purpose

The endpoint reports fail-closed DHT privacy posture for the service-node DHT boundary.

It is not a transport descriptor.

It is not a public route.

It is not a provider socket.

It is a self-describing safety status DTO.

## 7.3 Expected DTO contract

The endpoint reports:

```json id="xrmjcz"
{
  "node": "crab://node/<64 lowercase hex>",
  "privacy_mode": true,
  "privacy_profile": "service_node_dht_public_boundary",
  "public_inbound_enabled": false,
  "user_ip_publication": "forbidden",
  "residential_ip_publication": "forbidden",
  "lan_ip_publication": "forbidden",
  "admin_bind_publication": false,
  "provider_identity_kind": "crab_node",
  "provider_records_use_raw_socket": false,
  "transport_routes_public": false,
  "raw_socket_publication": false,
  "peer_socket_publication": false,
  "service_routes_public": false,
  "dto_ip_fields_allowed": false,
  "route_identity_scheme": "crab://node"
}
```

This contract is captured directly in the notes and was the central Phase 7D outcome. 

## 7.4 Meaning of key fields

```text id="wz8mlk"
privacy_mode = true
```

DHT public status is explicitly privacy-protecting.

```text id="tqg58z"
privacy_profile = service_node_dht_public_boundary
```

This is the named posture for service-node DHT surfaces.

```text id="8u6t8s"
public_inbound_enabled = false
```

This specific DTO does not publish public inbound route details.

```text id="f5g6sh"
user_ip_publication = forbidden
residential_ip_publication = forbidden
lan_ip_publication = forbidden
```

No user/residential/local IP publication.

```text id="pfg5ku"
provider_identity_kind = crab_node
```

Provider identity is logical CrabLink node identity.

```text id="07dgcx"
provider_records_use_raw_socket = false
transport_routes_public = false
raw_socket_publication = false
peer_socket_publication = false
service_routes_public = false
```

No public raw route/socket publication.

```text id="w60chb"
dto_ip_fields_allowed = false
```

DTOs should not include IP fields.

```text id="jr94rw"
route_identity_scheme = crab://node
```

Canonical logical identity scheme.

## 7.5 Tests added

Test file:

```text id="7g1mxf"
crates/svc-dht/tests/privacy_status.rs
```

Covered tests:

```text id="4rsopv"
privacy_status_reports_fail_closed_public_route_posture
privacy_status_dto_has_no_ip_or_transport_route_leaks
```

Known green result:

```text id="d8rz8w"
privacy_status: 2 passed
```

---

## 8. `rpc::http` Changes

## 8.1 Primary file

```text id="3gjcm8"
crates/svc-dht/src/rpc/http.rs
```

The latest codebundle comments summarize the Phase 7 purpose:

```text id="4iobkp"
Admin endpoints + DHT demo endpoints using canonical crab://node providers.
Phase 7 prevents provider records and service identity DTOs from exposing raw IPs or transport-specific schemes.
```

The module invariants include:

```text id="75msn3"
deny unknown fields
return 400 on bad input
no lock across .await
provider identities are crab://node/<64-lowercase-hex>
```

The test list in the module comments includes:

```text id="c1yenv"
tests/api_smoke.rs
tests/crab_node_identity.rs
tests/privacy_no_ip_leak.rs
tests/node_identity_config.rs
tests/privacy_status.rs
```

All of that appears in the newest codebundle’s `rpc/http.rs` snippet. 

## 8.2 State change

`State` now carries:

```rust id="wqubvi"
pub node_id: CrabNodeId,
```

This allows HTTP handlers to derive canonical identity safely.

## 8.3 Constructors

`State::new` delegates to:

```text id="5grm4e"
State::new_with_node_id
```

`State::new_with_node_id` stores a typed `CrabNodeId` and all DHT runtime dependencies.

## 8.4 Handler classes

`rpc::http` now owns:

```text id="rizz19"
health/admin status handlers
provider provide/find handlers
node_identity handler
privacy_status handler
lookup/demo DHT handlers
```

## 8.5 Important route behavior

Provider/demo endpoints should emit canonical provider identities, not raw routes.

Bad input should return:

```text id="8z9sy5"
400 Bad Request
```

rather than accepting unsafe provider identity.

---

## 9. `types.rs` Changes / Identity Model

## 9.1 Primary file

```text id="fr5m1h"
crates/svc-dht/src/types.rs
```

## 9.2 Key type

```text id="zj5gyg"
CrabNodeId
```

Expected functionality:

```text id="5tzsdz"
parse canonical crab://node URI
validate 64 lowercase hex node id
convert node id to node hex
convert node id to crab://node URI
reject legacy/transport schemes
```

## 9.3 Related type

```text id="zt4ovl"
B3Cid
```

Still used for content/provider lookup.

## 9.4 Why this matters

Putting canonical node identity in a type prevents stringly-typed route bugs like:

```text id="mcxhgi"
provider = "tcp://192.168.1.10:7000"
provider = "http://127.0.0.1:8080"
provider = "onion://..."
provider = "service://..."
```

The type-level model keeps public identity as:

```text id="2f3o3r"
crab://node/<64 lowercase hex>
```

---

## 10. Provider Record / Provider Store Posture

## 10.1 Provider identity change

Provider identity should now be logical CrabLink identity:

```text id="af5qwt"
crab://node/<64 lowercase hex>
```

not raw service route.

## 10.2 Provider store role

Primary files likely involved:

```text id="lgohew"
crates/svc-dht/src/provider/record.rs
crates/svc-dht/src/provider/store.rs
crates/svc-dht/src/provider/mod.rs
```

The provider store still owns provider records and TTL/lookup behavior, but provider identity must remain canonical.

## 10.3 Important restriction

Provider records must not be used as:

```text id="7hoh1g"
raw TCP route
raw HTTP route
raw websocket route
residential IP directory
admin endpoint directory
metrics endpoint directory
```

## 10.4 Future distinction

Later phases may need a separate service descriptor layer for routable service nodes.

Do not overload provider identity with raw routes.

Correct split:

```text id="61abaf"
provider identity = crab://node/<node-id>
service route resolution = later deliberate/private/safe mechanism
```

---

## 11. Public JSON Privacy Review Model

## 11.1 Primary file

```text id="4m58p9"
crates/svc-dht/src/privacy.rs
```

## 11.2 Intended API

The tests import:

```rust id="hjegee"
use svc_dht::privacy::review_public_json;
```

This function returns a result that indicates whether a JSON public DTO is safe.

## 11.3 Accepted DTO examples

Expected safe:

```json id="2wu5zd"
{
  "node": "crab://node/<64 lowercase hex>",
  "provider_identity_kind": "crab_node"
}
```

## 11.4 Rejected DTO examples

Expected rejected:

```json id="bt6z8h"
{
  "peer_ip": "192.168.1.10"
}
```

Expected rejected:

```json id="sinx6m"
{
  "provider": "tcp://192.168.1.10:7000"
}
```

Expected rejected:

```json id="987seg"
{
  "route": "onion://abc"
}
```

## 11.5 Future use

Any future public DHT DTO should have a focused test like:

```text id="cw0v4y"
review_public_json(&dto).expect("public DTO must not leak IP/socket/transport route")
```

---

## 12. Tests That Now Matter Most

## 12.1 `crab_node_identity.rs`

Command:

```bash id="6z0c00"
cargo test -p svc-dht --test crab_node_identity
```

Known green result:

```text id="tf8o2j"
4 passed
```

Purpose:

```text id="tudf2x"
locks canonical crab://node provider identity and legacy/transport scheme rejection
```

Likely covers:

```text id="bvqu75"
valid crab://node URI accepted
bad scheme rejected
bad casing/hex rejected
provider records use canonical identity
find/debug output stays canonical
```

## 12.2 `privacy_no_ip_leak.rs`

Command:

```bash id="hy5x9e"
cargo test -p svc-dht --test privacy_no_ip_leak
```

Known green result:

```text id="x8qtbz"
3 passed
```

Purpose:

```text id="2wb8nj"
locks public JSON privacy reviewer and no-IP/no-socket/no-raw-route DTO output
```

Covered tests:

```text id="2lldcm"
find_and_debug_outputs_do_not_expose_ip_or_socket_fields
privacy_reviewer_accepts_crab_node_identity_json
privacy_reviewer_flags_ip_fields_and_raw_transport_routes
```

## 12.3 `node_identity_config.rs`

Command:

```bash id="qsm9a5"
cargo test -p svc-dht --test node_identity_config
```

Known green result:

```text id="f85tji"
3 passed
```

Purpose:

```text id="jft6h6"
locks DHT_NODE_URI config, typed CrabNodeId state, and safe /dht/node_identity DTO
```

Covered tests:

```text id="up2ms0"
config_from_env_accepts_canonical_dht_node_uri
config_from_env_rejects_legacy_or_transport_node_uri
node_identity_dto_exposes_only_crab_node_identity
```

## 12.4 `privacy_status.rs`

Command:

```bash id="x4a08o"
cargo test -p svc-dht --test privacy_status
```

Known green result:

```text id="ffvn49"
2 passed
```

Purpose:

```text id="kwqf2t"
locks /dht/privacy_status fail-closed privacy posture and no route/IP leakage
```

Covered tests:

```text id="q3cq8i"
privacy_status_reports_fail_closed_public_route_posture
privacy_status_dto_has_no_ip_or_transport_route_leaks
```

## 12.5 Existing adjacent tests

Existing svc-dht tests to continue watching:

```text id="igz31t"
api_smoke.rs
provider_roundtrip.rs
nodeid_and_store.rs
readiness_bootstrap.rs
deadline_hedge.rs
kbucket_props.rs
asn_diversity.rs
```

These are not all Phase 7-specific, but they are adjacent to DHT correctness.

---

## 13. Scripts / Local Run Changes

Likely touched or updated scripts:

```text id="2c870k"
crates/svc-dht/scripts/two-node-local.sh
crates/svc-dht/scripts/run-local.sh
```

Expected changes:

```text id="tadv32"
use DHT_NODE_URI
use crab://node identity
avoid relay:// / onion:// / service:// examples
include privacy_status/node_identity probes
avoid raw IP route examples as public provider identity
```

Future examples should use:

```bash id="k0ogw0"
DHT_NODE_URI=crab://node/<64 lowercase hex>
```

not:

```bash id="23qjmg"
DHT_NODE_URI=tcp://...
DHT_NODE_URI=onion://...
DHT_NODE_URI=relay://...
```

---

## 14. Main Files Changed / Important Files

## 14.1 Primary source files

```text id="rfpqmq"
crates/svc-dht/src/types.rs
crates/svc-dht/src/privacy.rs
crates/svc-dht/src/config.rs
crates/svc-dht/src/rpc/http.rs
crates/svc-dht/src/main.rs
crates/svc-dht/src/lib.rs
```

## 14.2 Provider-related files to watch

```text id="lhg2uc"
crates/svc-dht/src/provider/record.rs
crates/svc-dht/src/provider/store.rs
crates/svc-dht/src/provider/mod.rs
crates/svc-dht/src/provider/ttl.rs
crates/svc-dht/src/provider/republish.rs
```

## 14.3 Pipeline/readiness files to watch

```text id="od3dsd"
crates/svc-dht/src/pipeline/lookup.rs
crates/svc-dht/src/pipeline/provide.rs
crates/svc-dht/src/readiness.rs
crates/svc-dht/src/health.rs
```

The latest codebundle confirms `readiness.rs` still owns a simple `ReadyGate` with `set_ready()` and `is_ready()`, and `rpc/http.rs` uses it in `State`. 

## 14.4 Primary tests

```text id="dzkxyu"
crates/svc-dht/tests/crab_node_identity.rs
crates/svc-dht/tests/privacy_no_ip_leak.rs
crates/svc-dht/tests/node_identity_config.rs
crates/svc-dht/tests/privacy_status.rs
```

## 14.5 Existing adjacent tests

```text id="j05mh3"
crates/svc-dht/tests/api_smoke.rs
crates/svc-dht/tests/provider_roundtrip.rs
crates/svc-dht/tests/nodeid_and_store.rs
crates/svc-dht/tests/readiness_bootstrap.rs
crates/svc-dht/tests/deadline_hedge.rs
crates/svc-dht/tests/kbucket_props.rs
crates/svc-dht/tests/asn_diversity.rs
```

## 14.6 Scripts

```text id="ndd7rq"
crates/svc-dht/scripts/two-node-local.sh
crates/svc-dht/scripts/run-local.sh
```

---

## 15. Current svc-dht Public Endpoint Contract

## 15.1 Existing status/admin endpoints

svc-dht continues to expose DHT admin/demo endpoints through `rpc::http`.

Core endpoint classes include:

```text id="at4jno"
/healthz or health-style admin
/readyz or ready-style admin
DHT provide/find/demo endpoints
/dht/node_identity
/dht/privacy_status
```

Exact route names beyond `node_identity` and `privacy_status` should be verified in `router()`/`routes()` before patching.

## 15.2 `/dht/node_identity`

Must return only:

```text id="e27thz"
canonical crab://node identity
node hex
identity scheme
provider identity kind
public inbound false
```

Must not return:

```text id="w1cwof"
raw IP
raw socket
admin bind
metrics bind
transport URL
legacy provider scheme
```

## 15.3 `/dht/privacy_status`

Must return fail-closed privacy posture.

Must report:

```text id="91kswi"
privacy_mode = true
user_ip_publication = forbidden
residential_ip_publication = forbidden
lan_ip_publication = forbidden
provider_records_use_raw_socket = false
transport_routes_public = false
raw_socket_publication = false
dto_ip_fields_allowed = false
route_identity_scheme = crab://node
```

## 15.4 Provider outputs

Any provider output that is public or operator-facing should use:

```text id="ebzqi6"
crab://node/<64 lowercase hex>
```

not transport route strings.

---

## 16. What svc-dht Does NOT Do Yet

svc-dht does not yet provide:

```text id="rwpqa0"
final production-grade service route resolution
real NAT traversal/public service descriptor lifecycle
durable registry-backed service descriptor authority
service-node quorum
wallet mutation
ledger mutation
confirmed ROC
reward payout logic
CrabLink user-node residential provider publication
Tor/onion routing for this architecture
```

These are either future phases or explicitly out of scope for the current node identity/privacy boundary.

---

## 17. Non-Regression Checklist

Future svc-dht patches must preserve:

```text id="v2q8e3"
canonical provider identity = crab://node/<64 lowercase hex>
DHT_NODE_URI accepts only canonical crab://node URI
RON_DHT_NODE_URI fallback accepts only canonical crab://node URI
legacy schemes rejected
transport schemes rejected
bad identity fails closed before startup
/dht/node_identity exposes no raw route/IP/socket
/dht/privacy_status exposes no raw route/IP/socket
privacy reviewer rejects IP/socket fields
privacy reviewer rejects forbidden schemes
provider records do not use raw sockets
transport_routes_public = false
raw_socket_publication = false
dto_ip_fields_allowed = false
```

Future svc-dht patches must not add public DTO fields like:

```text id="ap5fu8"
ip
ip_addr
remote_addr
socket_addr
http_addr
metrics_addr
source_ip
viewer_ip
peer_ip
client_ip
residential_ip
lan_ip
admin_bind
transport_url
tcp_url
udp_url
ws_url
onion_url
```

unless the field is explicitly private/internal-only and tested not to surface publicly.

Future svc-dht patches must not reintroduce:

```text id="r3c87y"
relay://
onion://
service://
local://
tcp://
udp://
ws://
wss://
http://
https://
```

as public node/provider identity.

---

## 18. Recommended svc-dht Baseline Commands

Before touching svc-dht again:

```bash id="obc7s4"
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-dht --test crab_node_identity
cargo test -p svc-dht --test privacy_no_ip_leak
cargo test -p svc-dht --test node_identity_config
cargo test -p svc-dht --test privacy_status
cargo test -p svc-dht --test api_smoke
cargo test -p svc-dht --test provider_roundtrip
cargo check -p svc-dht --all-targets
cargo clippy -p svc-dht --all-targets --no-deps -- -D warnings
```

Then:

```bash id="wzxvfl"
cargo check --workspace
```

If testing only Phase 7 regression:

```bash id="n19zal"
cargo test -p svc-dht --test crab_node_identity
cargo test -p svc-dht --test privacy_no_ip_leak
cargo test -p svc-dht --test node_identity_config
cargo test -p svc-dht --test privacy_status
cargo check -p svc-dht --all-targets
cargo clippy -p svc-dht --all-targets --no-deps -- -D warnings
```

---

## 19. How svc-dht Fits the Next Phase

The next official BUILD_PLAN_Z phase after Phase 7 is:

```text id="dh3nqc"
Phase 8 — Service Node Foundation
```

For svc-dht, Phase 8 should probably mean:

```text id="9b1req"
macronode reports DHT readiness truthfully
macronode service-node foundation can see svc-dht status
svc-dht continues exposing safe node_identity/privacy_status
provider status is integrated without raw route leakage
DHT readiness/provider info can be surfaced to macronode/svc-admin safely
```

Recommended approach:

```text id="0myhds"
Start Phase 8 in macronode.
Only touch svc-dht if macronode needs a real DHT readiness/provider status contract.
Preserve all Phase 7 identity/privacy tests.
Do not expand svc-dht into service route publication before the build plan calls for it.
```

Potential Phase 8 svc-dht-adjacent work:

```text id="fp1gf6"
service-node DHT readiness summary DTO
provider store count/status endpoint
safe provider inventory summary
safe DHT config status
macronode reads or embeds DHT status without exposing IP/socket
```

But do not implement this ahead of the macronode Phase 8 slice unless needed.

---

## 20. Good svc-dht Phase Marker to Carry Forward

Use this marker in next-session notes:

```text id="xg1rlf"
svc-dht — BUILD_PLAN_Z status

Completed:
- canonical CrabLink node identity:
  crab://node/<64 lowercase hex>
- legacy/transport schemes rejected:
  relay://
  onion://
  service://
  local://
  tcp://
  udp://
  ws://
  wss://
  http://
  https://
- typed CrabNodeId carried in rpc::http::State
- DHT_NODE_URI config support
- RON_DHT_NODE_URI fallback support
- invalid transport DHT_NODE_URI fails closed
- safe /dht/node_identity endpoint:
  node
  node_id_hex
  identity_scheme=crab://node
  provider_identity_kind=crab_node
  public_inbound=false
- public JSON privacy reviewer:
  rejects IP/socket fields
  rejects raw transport routes
  rejects forbidden schemes
  accepts crab://node identity
- safe /dht/privacy_status endpoint:
  privacy_mode=true
  user/residential/LAN IP publication forbidden
  provider_records_use_raw_socket=false
  transport_routes_public=false
  raw_socket_publication=false
  peer_socket_publication=false
  service_routes_public=false
  dto_ip_fields_allowed=false
  route_identity_scheme=crab://node

Known green:
cargo test -p svc-dht --test crab_node_identity
cargo test -p svc-dht --test privacy_no_ip_leak
cargo test -p svc-dht --test node_identity_config
cargo test -p svc-dht --test privacy_status
cargo check -p svc-dht --all-targets
cargo clippy -p svc-dht --all-targets --no-deps -- -D warnings
cargo check --workspace
```

---

## 21. Short Summary

```text id="wszhcf"
Since starting BUILD_PLAN_Z, svc-dht has been hardened into the safe service-node discovery boundary. It now uses canonical crab://node identity, rejects legacy and raw transport schemes, supports DHT_NODE_URI/RON_DHT_NODE_URI only when canonical, exposes safe /dht/node_identity and /dht/privacy_status endpoints, and includes a public JSON privacy reviewer that catches IP/socket/raw-route leaks. The crate is green through Phase 7 and should not be expanded into raw route publication or service descriptor authority until a later phase explicitly calls for it.
```


### END NOTE - JULY 10 2026 - 12:55 CST


### BEGIN NOTE - JULY 11 2026 - 22:55 CST

Here is the complete crate-specific changelog record for the `svc-dht` work completed during BUILD_PLAN_Z Phase 10.

# `svc-dht` — BUILD_PLAN_Z Phase 10 Changelog Notes

## 1. Crate role

`svc-dht` is the RustyOnions provider-discovery and logical node-identity service.

Its responsibilities include:

```text
maintaining provider records for content identifiers
representing providers through canonical CrabLink node identities
finding currently live providers
refreshing provider TTLs
purging expired provider records
exposing bounded provider lookup and mutation routes
preserving the privacy boundary between logical identity and transport identity
```

During BUILD_PLAN_Z Phase 10, `svc-dht` gained a real exact-provider withdrawal capability used by the service-node prune path.

The resulting responsibility is:

```text
Given one exact content CID and the service's configured CrabNodeId,
remove only that provider record from the local provider store and
report truthfully whether a record was actually removed.
```

This work turned provider withdrawal from an implied future behavior into a real, tested primitive consumed by macronode.

The implementation remains local and bounded. It does not claim that a provider record was withdrawn from every node, replicated index, or future distributed DHT participant.

---

## 2. Scope completed in this session

The Phase 10 `svc-dht` work covered these implementation slices:

```text
Phase 10F2A:
  exact provider-store withdrawal primitive

Phase 10F2B:
  HTTP endpoint for withdrawing the configured local provider

Phase 10F2C1:
  reusable svc-dht router/state construction for embedding

Phase 10F2C2 and later:
  downstream macronode embeds and registers the same provider store
```

The downstream macronode work is documented separately, but it depends directly on the `svc-dht` behavior described here.

Final crate status for this session:

```text
exact provider withdrawal: complete
truthful withdrawal outcomes: complete
neighbor preservation: complete
idempotent repeat behavior: complete
configured-node authority: complete
strict request shape: complete
reusable router: complete
standalone compatibility: preserved
macronode embedding support: complete
focused tests: green
workspace compilation: green
```

---

## 3. Primary files changed

The main files changed in `svc-dht` were:

```text
crates/svc-dht/src/provider/mod.rs
crates/svc-dht/src/provider/store.rs
crates/svc-dht/src/rpc/http.rs
crates/svc-dht/src/lib.rs

crates/svc-dht/tests/provider_withdrawal.rs
crates/svc-dht/tests/provider_withdrawal_http.rs
```

The current crate bundle includes the exact withdrawal primitive, HTTP handler, reusable router, and both focused test suites. 

---

# Part I — Preserved Identity and Privacy Foundation

## 4. Existing canonical node identity

Phase 10 built on the already-established canonical provider identity:

```text
crab://node/<64 lowercase hexadecimal characters>
```

Example:

```text
crab://node/00000000000000000000000000000000000000000000000000000000000000a1
```

The provider store continues to use the typed:

```rust
CrabNodeId
```

rather than arbitrary route strings.

Legacy or transport-specific identities remain rejected, including:

```text
relay://
onion://
service://
local://
tcp://
udp://
http://
https://
raw IP addresses
socket addresses
```

The existing `CrabNodeId` contract was not newly invented in Phase 10, but Phase 10’s withdrawal behavior deliberately reused it rather than accepting a new untyped node field.

The existing identity tests prove canonical round trips and rejection of transport-specific identities. 

---

## 5. Logical identity remains separate from transport

Provider records contain:

```text
content CID
typed CrabNodeId
expiration time
```

They do not contain:

```text
residential IP address
LAN IP address
socket address
TCP listener
HTTP route
Tor/onion address
transport session
wallet address
reward address
```

That distinction remains important after provider withdrawal was added.

The operation removes a logical provider record:

```text
CID + CrabNodeId
```

It does not disconnect a transport socket, expose a user address, or reveal where the node is physically located.

---

# Part II — Exact Provider-Store Withdrawal

## 6. New `ProviderWithdrawalOutcome`

A truthful result enum was added:

```rust
pub enum ProviderWithdrawalOutcome {
    Withdrawn,
    NotFound,
}
```

The enum is publicly available through the provider module.

### `Withdrawn`

Means:

```text
a provider record existed for the exact normalized CID
and the exact typed CrabNodeId
and that record was removed from the local provider store
```

### `NotFound`

Means:

```text
no record existed for that exact CID and node combination
```

`NotFound` is not treated as an internal error.

It is an idempotent no-op result that allows higher-level code to distinguish:

```text
changed state
from
state was already absent
```

The current implementation and public enum are visible in the final crate source. 

---

## 7. New `Store::withdraw_id`

The provider store gained an operation equivalent to:

```rust
pub fn withdraw_id(
    &self,
    cid: &str,
    node: CrabNodeId,
) -> ProviderWithdrawalOutcome
```

This method performs exact matching across two dimensions:

```text
exact CID
+
exact CrabNodeId
```

A record is removed only when both match.

---

## 8. Exact CID and node semantics

Assume the store contains:

```text
CID A → Node A
CID A → Node B
CID B → Node A
```

Calling:

```text
withdraw_id(CID A, Node A)
```

produces:

```text
CID A → Node B remains
CID B → Node A remains
CID A → Node A is removed
```

This proves the method is neither:

```text
remove every provider for this CID
nor
remove every record belonging to this node
```

It removes one exact relationship.

The focused test explicitly preserves both the neighboring provider and the same node’s record for a different CID. 

---

## 9. Removal includes expired matching records

The explicit withdrawal operation removes matching records regardless of whether the record is still live or has already expired by TTL.

This was intentional.

An explicit object prune must not leave a stale expired record sitting in the local map waiting for the background TTL worker.

The behavior is:

```text
matching live record:
  removed

matching expired record still present in map:
  removed

nonmatching provider:
  preserved

nonmatching CID:
  preserved
```

The existing read APIs still filter expired records when reporting live providers, while explicit withdrawal cleans up the matching stored record itself.

---

## 10. One write-lock mutation

The method acquires the provider map’s write lock, mutates the matching CID bucket, and determines the outcome within that operation.

Conceptually:

```text
normalize CID
acquire provider-map write lock
find exact CID bucket
retain all records except exact node
determine whether anything changed
remove empty CID bucket if necessary
return Withdrawn or NotFound
```

It does not:

```text
perform an asynchronous call while holding the lock
make an HTTP request
contact another service
hold the lock across await
perform a check under one lock and deletion under another
```

This keeps the mutation local and deterministic.

---

## 11. Empty CID bucket cleanup

If withdrawing the exact provider leaves no records under that CID, the now-empty CID entry is removed from the map.

This prevents accumulation of empty provider vectors.

If other providers remain, the CID bucket remains intact.

---

## 12. Idempotent behavior

Repeated withdrawal behaves as:

```text
first exact withdrawal:
  Withdrawn

second exact withdrawal:
  NotFound

later repeated withdrawals:
  NotFound
```

The second request does not claim another change occurred.

This is required for macronode to distinguish:

```text
pruned
already_absent
partial
failed
```

without fabricating a successful mutation.

---

## 13. No collateral mutation

Withdrawing an absent provider does not mutate neighboring records.

For example:

```text
CID A contains only Node B
request asks to remove Node A
```

The result is:

```text
NotFound
```

and Node B remains.

The focused provider test verifies that an absent-target withdrawal leaves the existing neighbor untouched. 

---

# Part III — HTTP Withdrawal Surface

## 14. New endpoint

The reusable `svc-dht` HTTP router gained:

```text
POST /dht/withdraw_local_provider
```

The route is part of the same router used by the standalone service and the embedded macronode runtime. 

The purpose of the endpoint is narrow:

```text
withdraw this service's configured node identity
for one exact canonical content CID
from this local provider store
```

---

## 15. Strict request body

The request body contains only:

```json
{
  "cid": "b3:<64 lowercase hex>"
}
```

The CID is decoded as the canonical typed:

```rust
B3Cid
```

The request does not accept:

```text
node
provider
node_id
node_uri
IP address
socket address
transport route
network scope
wallet identity
reward identity
```

Unknown fields reject through strict serde behavior.

---

## 16. Caller cannot select the provider identity

A major safety rule is:

```text
the HTTP caller supplies the CID
the configured svc-dht State supplies the node identity
```

The handler uses:

```rust
state.node_id
```

as the authoritative provider to withdraw.

This prevents a caller from requesting:

```text
withdraw Node B
```

while connected to Node A’s local administration surface.

A request body such as:

```json
{
  "cid": "b3:...",
  "node": "crab://node/...b2"
}
```

is rejected because `node` is an unknown field.

The HTTP-focused tests explicitly verify this rejection. 

---

## 17. Why configured-node authority matters

Without this rule, the endpoint could become a primitive for deleting arbitrary neighboring provider records.

The dangerous design would be:

```text
caller selects CID
caller selects node
server removes that pair
```

The implemented design is instead:

```text
caller selects CID
server uses its own configured CrabNodeId
server can withdraw only its own local record
```

This preserves provider ownership boundaries.

The endpoint cannot be used by one node to remove another node’s provider entry from its local store through caller-supplied identity.

---

## 18. Successful response

When the exact configured-node record exists, the endpoint returns HTTP 200 with a response shaped around:

```json
{
  "ok": true,
  "status": "withdrawn",
  "scope": "local_provider_store",
  "cid": "b3:...",
  "node": "crab://node/..."
}
```

Important truths in this response:

```text
status says a record was actually removed
scope says local_provider_store
node reports the configured local identity
CID reports the exact requested object
```

It does not say:

```text
network_withdrawn
globally_removed
all_providers_removed
content_deleted
replicas_deleted
index_cleared
reward_revoked
```

---

## 19. Not-found response

When no matching record exists, the endpoint returns HTTP 404 with:

```json
{
  "ok": false,
  "status": "not_found",
  "scope": "local_provider_store",
  "cid": "b3:...",
  "node": "crab://node/..."
}
```

The 404 represents:

```text
that exact local provider relation is absent
```

It does not represent:

```text
the content does not exist
the storage bytes do not exist
no providers exist anywhere
the object is globally unavailable
```

Those are different questions owned by other services or wider network state.

---

## 20. Neighbor preservation through HTTP

The handler tests populate:

```text
CID → configured Node A
CID → neighboring Node B
```

They then call the endpoint while the service is configured as Node A.

Expected result:

```text
Node A removed
Node B remains
HTTP 200
status = withdrawn
scope = local_provider_store
```

The test checks the provider store after the handler returns, not merely the JSON response. 

---

## 21. Repeated HTTP withdrawal

The handler’s idempotency behavior is:

```text
first request:
  HTTP 200
  ok = true
  status = withdrawn

second request:
  HTTP 404
  ok = false
  status = not_found
```

The second response does not print or return fake success.

This behavior is verified in the HTTP-focused suite. 

---

# Part IV — Reusable Router and Runtime State

## 22. New reusable router construction

The `svc-dht` HTTP surface was refactored around a reusable function equivalent to:

```rust
pub fn build_router(state: State) -> Router
```

The complete router includes:

```text
GET  /healthz
GET  /readyz
GET  /version
GET  /dht/node_identity
GET  /dht/privacy_status
GET  /metrics
GET  /dht/find_providers/:cid
POST /dht/provide
POST /dht/withdraw_local_provider
GET  /dht/_debug/list
```

The router receives an already-created `State`.

It does not create:

```text
a second provider store
a replacement node identity
a new readiness gate
a different lookup context
a disconnected metrics instance
```

The reusable router source explicitly states that standalone and embedded callers share the already-created state. 

---

## 23. Why router reuse was necessary

Before embedding, the standalone service could construct its own complete runtime internally.

Macronode needed to run the same real `svc-dht` service while also retaining handles to:

```text
the provider store
the configured node identity
the readiness lifecycle
```

A reusable router allows:

```text
build one runtime state
retain the real provider-store Arc
construct the real svc-dht router from that state
serve it inside macronode
register the same store with the prune coordinator
```

Without this, macronode might accidentally:

```text
serve one provider store
prune a second provider store
return success while the live record remains
```

The refactor prevents that disconnected-runtime problem.

---

## 24. Standalone behavior preserved

The standalone `svc-dht` service continues to use the same route set and state model.

The router extraction was not a replacement with a reduced test-only router.

It preserved:

```text
health
readiness
version
identity
privacy status
metrics
provider lookup
provider addition
debug listing
```

and added provider withdrawal.

---

## 25. Shared `Arc<Store>`

The runtime state carries:

```rust
Arc<Store>
```

That same provider-store handle can be retained by an embedding host.

This enables the real relationship:

```text
one provider store
  ->
svc-dht HTTP routes
  ->
lookup context
  ->
TTL pruner
  ->
macronode prune coordinator
```

All of those consumers observe the same provider records.

---

## 26. Shared configured `CrabNodeId`

The state also carries the configured:

```rust
CrabNodeId
```

The withdrawal handler and macronode coordinator use the same identity.

This avoids a mismatch where:

```text
svc-dht advertises Node A
but pruning attempts to withdraw Node B
```

The configured local node remains the authority for its own withdrawal.

---

# Part V — Provider Store Semantics Preserved

## 27. Add/refresh behavior

Existing provider insertion remains:

```text
CID + CrabNodeId + TTL
```

If the same node is added again for the same CID, its record is refreshed rather than duplicated.

The withdrawal work did not change this behavior.

---

## 28. TTL behavior

Existing provider records remain expiration-bound.

The TTL worker can purge expired records in the background.

The two removal mechanisms now have different purposes:

```text
TTL purge:
  removes expired provider records over time

explicit withdrawal:
  immediately removes one exact CID/node relation
```

Explicit prune coordination does not wait for TTL expiration.

---

## 29. Read behavior

The read-only provider APIs continue to expose only live records:

```text
get_live_ids
get_live
```

`get_live_ids` returns typed identities.

`get_live` produces canonical `crab://node/...` strings for API and CLI presentation.

No raw socket or transport identity was added as part of withdrawal.

---

# Part VI — Tests Added

## 30. `tests/provider_withdrawal.rs`

A focused provider-store test suite was added.

It contains three principal tests.

### Exact CID and node removal

Proves:

```text
target CID/node is removed
neighbor node under same CID remains
same node under another CID remains
outcome = Withdrawn
```

### Repeated withdrawal

Proves:

```text
first result = Withdrawn
second result = NotFound
no fake repeated success
```

### Absent target preservation

Proves:

```text
attempted absent provider returns NotFound
existing neighboring provider remains
```

The test file’s stated invariants also explicitly exclude raw IP, socket, transport-route, wallet, ledger, and reward behavior. 

---

## 31. `tests/provider_withdrawal_http.rs`

A second focused suite tests the HTTP adapter.

It covers:

```text
handler withdraws only configured local node
neighboring provider remains
success returns HTTP 200
success response says local_provider_store
repeat returns HTTP 404
repeat response says not_found
caller-supplied node identity rejects
```

The test state uses:

```text
canonical B3Cid
canonical CrabNodeId
real Arc<Store>
real ReadyGate
real LookupCtx
```

rather than a fake success string.

---

## 32. Downstream embedded-router test

Macronode later added a test that constructs the real `svc-dht` runtime and router, places two providers in its store, and invokes:

```text
POST /dht/withdraw_local_provider
```

The test proves the embedded router:

```text
withdraws the configured node
preserves the neighbor
returns local scope
uses the same retained provider store
```

That test belongs to macronode, but it validates the embedding surface created in `svc-dht`. 

---

# Part VII — Verification State

## 33. Focused commands

The Phase 10 `svc-dht` behavior was validated with focused commands equivalent to:

```bash
cargo test -p svc-dht \
  --test provider_withdrawal \
  -- --nocapture

cargo test -p svc-dht \
  --test provider_withdrawal_http \
  -- --nocapture
```

The exact store and HTTP behavior passed before macronode integration continued.

---

## 34. Broader regression checks

The crate’s existing identity and privacy tests remained relevant:

```bash
cargo test -p svc-dht --test crab_node_identity
cargo test -p svc-dht --test privacy_no_ip_leak
cargo test -p svc-dht --test node_identity_config
cargo test -p svc-dht --test privacy_status
cargo test -p svc-dht --test provider_roundtrip
```

These protect against accidentally weakening the privacy and canonical-identity posture while adding mutable provider operations.

---

## 35. Quality gates

Relevant quality gates included:

```bash
cargo fmt -p svc-dht

cargo check -p svc-dht

cargo clippy \
  -p svc-dht \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo check --workspace
```

The final Phase 10 workspace check passed after downstream `macronode` and `svc-index` integration.

---

# Part VIII — Downstream Integration

## 36. Macronode

Macronode now embeds the real `svc-dht` runtime instead of running a disconnected placeholder.

It retains:

```text
Arc<ProviderStore>
configured CrabNodeId
real svc-dht router
real listener lifecycle
```

The same provider store is registered with macronode’s prune coordinator.

The resulting internal call is:

```rust
providers.withdraw_id(
    object.as_str(),
    configured_node_id,
)
```

Macronode maps the result into its own independently reported step:

```text
Withdrawn
NotFound
Unavailable
```

The provider result remains separate from:

```text
local storage deletion
svc-index provider-cache invalidation
```

---

## 37. `crabnode prune`

The operator CLI does not call the DHT endpoint directly.

Its normal control path is:

```text
crabnode prune
  ->
protected macronode moderation endpoint
  ->
macronode PruneCoordinator
  ->
registered svc-dht provider store
  ->
Store::withdraw_id
```

This keeps multi-surface operator semantics inside macronode while preserving the reusable DHT HTTP route for direct service testing and future bounded administration.

---

## 38. Relationship to storage

`svc-dht` does not delete bytes.

These are independent operations:

```text
svc-storage:
  deletes the exact local object bytes

svc-dht:
  removes the exact local provider relationship
```

One operation can succeed while the other is unavailable or already absent.

Macronode therefore reports both separately.

---

## 39. Relationship to `svc-index`

`svc-dht` provider withdrawal does not invalidate the `svc-index` provider-response cache.

That became a third distinct Phase 10 operation:

```text
svc-storage local bytes
svc-dht provider record
svc-index provider cache
```

Each component owns its own state and truthful outcome.

---

# Part IX — Security and Authority Boundaries

## 40. What `svc-dht` now owns

After this Phase 10 session, `svc-dht` owns:

```text
typed CrabNodeId provider identity
local in-memory provider records
provider TTL and expiration
provider add/refresh
live provider lookup
expired-provider purge
exact CID/node withdrawal
truthful Withdrawn/NotFound outcomes
configured-node-only HTTP withdrawal
local provider-store response scope
reusable HTTP router construction
```

---

## 41. What `svc-dht` does not own

The crate does not own:

```text
physical object bytes
storage pruning
moderation precedence
global deny policy
local block policy
quarantine policy
owner tombstones
manifest-pointer deletion
svc-index cache invalidation
network-wide provider withdrawal
network-wide content deletion
wallet mutation
ledger mutation
reward planning
reward payout
confirmed ROC
ROX minting or burning
bridge settlement
```

---

## 42. No arbitrary-provider deletion authority

The HTTP caller cannot choose which node identity to remove.

This means the route is not:

```text
delete any provider record
```

It is:

```text
withdraw this configured service node's own local provider record
```

That distinction must be preserved in future networked implementations.

---

## 43. No network-wide propagation

The current withdrawal behavior modifies only:

```text
this process's local provider store
```

It does not:

```text
broadcast a withdrawal
contact seed nodes
remove remote records
invalidate every cache
force peers to forget the provider
publish a global tombstone
wait for network consensus
```

Macronode’s Phase 10 prune response explicitly reports:

```text
network_propagation = false
```

This is a truthful limitation, not an implementation failure hidden behind success.

---

## 44. No public inbound route claim

A provider identity being present in the DHT does not necessarily mean:

```text
a public TCP port is exposed
a residential IP is visible
the node accepts arbitrary inbound traffic
the provider URI is directly dialable as a raw transport route
```

The public identity is logical:

```text
crab://node/<node-id>
```

Transport resolution remains a separate privacy-sensitive layer.

---

## 45. No economic authority

Provider presence or withdrawal does not itself determine ROC rewards.

The crate does not:

```text
award a service reward when a provider is added
revoke ROC when a provider is withdrawn
treat a provider record as delivery proof
treat a provider record as availability proof
issue wallet instructions
write ledger receipts
```

Future evidence systems may reference provider facts, but reward eligibility must still pass through evidence, accounting, policy, quorum, wallet, and ledger boundaries.

---

# Part X — Behavioral Notes and Edge Cases

## 46. `NotFound` is a valid completed check

A provider withdrawal returning `NotFound` means:

```text
the local store was available
the exact CID/node relationship was checked
nothing needed removal
```

This is different from:

```text
provider store unavailable
```

Macronode uses this distinction when calculating:

```text
already_absent
versus
partial or failed
```

---

## 47. Neighbor records must survive forever as an invariant

Future changes must preserve:

```text
withdrawing Node A from CID X
must not remove Node B from CID X
```

This is essential in a decentralized provider model.

A prune initiated by one operator cannot erase another provider’s logical record merely because both serve the same content.

---

## 48. Same-node, different-CID records must survive

Future changes must also preserve:

```text
withdrawing Node A from CID X
must not remove Node A from CID Y
```

Provider withdrawal is object-specific.

It is not a global node de-registration operation.

---

## 49. Withdrawal and node shutdown are different

Withdrawing one provider record does not:

```text
shut down svc-dht
remove the node identity
clear all provider records
mark the node unhealthy
change readiness
disconnect peers
```

It removes one exact provider relationship.

---

## 50. Withdrawal and moderation are different

A content object may be:

```text
blocked but provider record still present temporarily
pruned and provider record withdrawn
not blocked but manually unprovided
tombstoned and later withdrawn
```

`svc-dht` does not decide moderation state.

It executes the exact provider-store mutation requested by its authorized caller.

---

# Part XI — Future Work

## 51. Networked provider propagation

A later provider-repair or distributed-DHT phase may add:

```text
signed provider advertisements
remote provider propagation
withdrawal propagation
provider record versioning
republish suppression after prune
peer acknowledgment
eventual-consistency reconciliation
```

None of those are implemented or claimed by the current local withdrawal primitive.

Future propagation must preserve the current exact identity:

```text
CID + CrabNodeId
```

and must not introduce raw IP publication.

---

## 52. Provider repair interaction

Later repair workers must respect explicit withdrawal and moderation posture.

An unsafe future loop would be:

```text
operator prunes object
provider record is withdrawn
repair worker immediately republishes provider record
```

Future repair behavior will need canonical policy and persistence-state checks before republishing or restoring a provider record.

---

## 53. Persistent provider store

The current provider store is RAM-backed and amnesia-friendly.

A future persistent implementation must preserve:

```text
exact CID/node removal
idempotent NotFound
neighbor preservation
same-node/different-CID preservation
TTL semantics
configured-node authority
no raw transport leakage
```

It must not report `Withdrawn` merely because a deletion request was queued.

---

## 54. Authentication boundary

The direct `svc-dht` endpoint is a bounded service route, but operator-level authorization is principally enforced through macronode’s protected administration plane.

Future standalone deployments should ensure mutable DHT administration routes are not exposed as unauthenticated public internet APIs.

The current route’s configured-node restriction limits the mutation scope, but it is not a substitute for deployment-layer access control.

---

# Part XII — Final Crate State

## 55. Final completed behavior

At the end of BUILD_PLAN_Z Phase 10, `svc-dht` can now answer and enforce:

```text
Which canonical CrabLink nodes provide this CID locally?
Can this configured node withdraw its exact provider record?
Did the withdrawal actually change state?
Can repeated withdrawal report NotFound instead of fake success?
Can neighboring providers remain untouched?
Can the same node's other content records remain untouched?
Can standalone and embedded runtimes use the same real router and store?
```

The answer to each is now yes.

---

## 56. Final status label

Use this status in future notes:

```text
svc-dht BUILD_PLAN_Z Phase 10 work:
COMPLETE / GREEN / PARKED

Exact local provider withdrawal:
COMPLETE

Configured-node-only HTTP withdrawal:
COMPLETE

Reusable standalone/embedded router:
COMPLETE

Canonical crab://node identity:
PRESERVED / GREEN

Raw IP and transport-route privacy boundary:
PRESERVED / GREEN

Network-wide withdrawal propagation:
NOT IMPLEMENTED / NOT CLAIMED
```

---

## 57. One-line summary

```text
svc-dht now provides a real, truthful, idempotent operation for withdrawing one exact canonical CID/provider relationship belonging to the configured local CrabLink node, while preserving neighboring providers, transport privacy, local-only scope, and strict separation from storage, index, moderation, wallet, ledger, and reward authority.
```

The next crate-specific changelog is `svc-index`, covering exact provider-cache invalidation, shared authoritative cache clones, neighbor and resolve-cache preservation, tests, and integration with macronode’s three-surface prune coordinator.


### END NOTE - JULY 11 2026 - 22:55 CST