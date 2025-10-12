

---

# svc-dht — final scaffold (no code)

```
svc-dht/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ .cargo/
│  └─ config.toml
├─ .rustfmt.toml
├─ clippy.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ build.rs
│
├─ src/
│  ├─ lib.rs
│  ├─ main.rs
│  │
│  ├─ config.rs
│  ├─ errors.rs
│  ├─ types.rs
│  ├─ invariants.rs
│  │
│  ├─ readiness.rs
│  ├─ health.rs
│  ├─ metrics.rs
│  ├─ tracing.rs
│  │
│  ├─ bootstrap.rs
│  ├─ supervision/
│  │  ├─ mod.rs
│  │  ├─ supervisor.rs
│  │  ├─ backoff.rs
│  │  └─ signals.rs
│  │
│  ├─ peer/
│  │  ├─ mod.rs
│  │  ├─ id.rs
│  │  ├─ bucket.rs
│  │  ├─ table.rs
│  │  └─ selector.rs
│  │
│  ├─ provider/
│  │  ├─ mod.rs
│  │  ├─ record.rs
│  │  ├─ store.rs
│  │  ├─ ttl.rs
│  │  └─ republish.rs
│  │
│  ├─ pq/
│  │  ├─ mod.rs
│  │  ├─ algo.rs
│  │  ├─ verify.rs
│  │  └─ gating.rs
│  │
│  ├─ codec/
│  │  ├─ mod.rs
│  │  ├─ frame.rs
│  │  ├─ decode.rs
│  │  ├─ encode.rs
│  │  └─ limits.rs
│  │
│  ├─ rpc/
│  │  ├─ mod.rs
│  │  ├─ kad.rs
│  │  ├─ discv5.rs
│  │  ├─ bus.rs
│  │  └─ http.rs
│  │
│  ├─ pipeline/
│  │  ├─ mod.rs
│  │  ├─ lookup.rs
│  │  ├─ provide.rs
│  │  ├─ hedging.rs
│  │  ├─ rate_limit.rs
│  │  ├─ asn_guard.rs
│  │  └─ deadlines.rs
│  │
│  ├─ transport/
│  │  ├─ mod.rs
│  │  ├─ clients.rs
│  │  └─ tor.rs
│  │
│  └─ cache/
│     ├─ mod.rs
│     ├─ memory.rs
│     └─ sled_cache.rs
│
├─ tests/
│  ├─ api_smoke.rs
│  ├─ readiness_bootstrap.rs
│  ├─ provider_roundtrip.rs
│  ├─ kbucket_props.rs
│  ├─ asn_diversity.rs
│  ├─ deadline_hedge.rs
│  └─ chaos/
│     ├─ netem.rs
│     ├─ partition.rs
│     └─ soak_churn.rs
│
├─ benches/
│  ├─ lookup_bench.rs
│  └─ README.md
│
├─ fuzz/
│  └─ fuzz_targets/
│     ├─ msg_frame_decode.rs
│     └─ kad_packet_decode.rs
│
├─ loom/
│  ├─ loom_kbucket.rs
│  └─ loom_hedge.rs
│
├─ examples/
│  ├─ find_providers.rs
│  └─ provide.rs
│
├─ docs/
│  ├─ IDB.md
│  ├─ DHT_CONFIG.md
│  ├─ PERFORMANCE.md
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  └─ state.mmd
│
├─ scripts/
│  ├─ run-local.sh
│  ├─ render-mermaid.sh
│  └─ chaos/
│     ├─ netem.sh
│     └─ partition.sh
│
└─ .github/
   └─ workflows/
      ├─ ci.yml
      ├─ mermaid.yml
      ├─ fuzz.yml
      └─ perf.yml
```

---

## Descriptions — file by file

### Root

* **Cargo.toml** — Crate manifest; declares features (`arti`, `sled-cache`, `tls`), pins workspace deps, and exposes binaries.
* **rust-toolchain.toml** — Enforces MSRV at tool level (e.g., `1.80.0`) so CI/dev boxes match.
* **.cargo/config.toml** — Dev conveniences (cargo aliases like `test-all`, resolver tweaks if needed).
* **.rustfmt.toml** — Formatting policy to keep diffs clean across contributors.
* **clippy.toml** — Lint policy with `deny = warnings` and project-specific lints.
* **README.md** — Ops/audit-ready overview (roles, SLOs, metrics, PQ/ASN, hedging, diagrams).
* **CHANGELOG.md** — SemVer history (flags, wire changes, perf-impact notes).
* **LICENSE-APACHE / LICENSE-MIT** — Dual-license headers per workspace policy.
* **build.rs** — Embeds build metadata (git SHA, timestamp) for metrics/traces.

### `src/` top level

* **lib.rs** — Stable crate surface and re-exports (Config, Metrics, HealthState, Bus facade).
* **main.rs** — Binary entry (parse env/flags, init tracing/metrics, spawn supervisor, serve admin HTTP).

#### Core config, errors, types, invariants

* **config.rs** — All env/flags, profiles (`micronode`/`macronode`), ASN/PQ/hedge defaults in one place.
* **errors.rs** — Error taxonomy with user-hints (`NoBootstrap`, `AsnCap`, `OverSize`, `HopBudget`, etc.).
* **types.rs** — Domain types (B3 CID, PeerInfo, ProviderRecord v1 with `proto_ver`), serde/validation.
* **invariants.rs** — Encoded “must-holds” for tests (no locks across `.await`, single-writer k-bucket, bounded inflight, hedge ≤ deadline).

#### Health, metrics, tracing

* **readiness.rs** — `/readyz` logic: bootstrap quorum + min table fill + optional cache warmup.
* **health.rs** — Lightweight liveness checks for `/healthz`.
* **metrics.rs** — Prometheus registry + canonical metrics (latency/hops/ASN/rejects/RF); stable names/labels/buckets.
* **tracing.rs** — Logging/tracing setup; span fields (`request_id`, `peer`, `cid`, `hops`, `alpha`, `beta`, `deadline_ms`).

#### Boot & supervision

* **bootstrap.rs** — Seed dialing, backoff, and min-fill gate; emits bus health.
* **supervision/mod.rs** — Module index for supervision components.
* **supervision/supervisor.rs** — Supervisory orchestration, task lifecycles, restarts with jitter.
* **supervision/backoff.rs** — Reusable exponential backoff strategies.
* **supervision/signals.rs** — Graceful shutdown (Ctrl-C/TERM).

#### Routing (Kademlia core)

* **peer/mod.rs** — Routing facade.
* **peer/id.rs** — NodeId & distance math; serialization.
* **peer/bucket.rs** — Kademlia k-bucket; eviction/update; **single-writer discipline**.
* **peer/table.rs** — Routing table; `closest()` queries; RTT-aware updates.
* **peer/selector.rs** — Peer selection tuned for α parallelism and β hedging.

#### Provider records

* **provider/mod.rs** — Provider store facade.
* **provider/record.rs** — ProviderRecord schema v1 (Ed25519 + optional PQ), TTL.
* **provider/store.rs** — Storage API (RAM + optional sled cache); verify on admit.
* **provider/ttl.rs** — Expiry pruning workers; counters.
* **provider/republish.rs** — Republish schedule & RF repair signals.

#### Post-Quantum posture

* **pq/mod.rs** — PQ feature surface.
* **pq/algo.rs** — Algorithm selection (ML-DSA vs SPHINCS+) behind traits.
* **pq/verify.rs** — Dual-sign verification; fail-closed when required.
* **pq/gating.rs** — Policy for `PQ_REQUIRE`, `PQ_REQUIRE_ON` cutover & interop window.

#### Wire codec isolation

* **codec/mod.rs** — Index for codec pieces.
* **codec/frame.rs** — OAP/1 frame shape & constants (1 MiB cap, etc.).
* **codec/decode.rs** — Parsers for Kad/Discv5 frames; strict early rejects.
* **codec/encode.rs** — Safe serialization routines (size-aware).
* **codec/limits.rs** — Central size/time bounds shared by handlers & fuzz.

#### RPC surfaces (Kad, Discv5, Bus, Admin HTTP)

* **rpc/mod.rs** — Unified surface index.
* **rpc/kad.rs** — Kad request/response handlers (FIND_NODE/PROVIDERS, STORE/PROVIDE) with hop/time budgets.
* **rpc/discv5.rs** — Discv5 peer discovery integration (optional).
* **rpc/bus.rs** — Kernel bus adapters (topics like `ron://dht/find_providers`).
* **rpc/http.rs** — Admin endpoints: `/healthz`, `/readyz`, `/metrics`.

#### Request pipeline (policies & limits)

* **pipeline/mod.rs** — Orchestration layer for request flows.
* **pipeline/lookup.rs** — Lookup FSM (fanout → converge), hop counters.
* **pipeline/provide.rs** — Provide flow (verify → admit → replicate).
* **pipeline/hedging.rs** — β hedges, staggered launches, deadline-aware.
* **pipeline/rate_limit.rs** — Per-peer & global inflight caps; backpressure.
* **pipeline/asn_guard.rs** — ASN entropy floor & per-ASN cap checks.
* **pipeline/deadlines.rs** — Budget accounting and per-leg timeouts.

#### Transport & cache

* **transport/mod.rs** — Thin abstraction over `ron-transport`.
* **transport/clients.rs** — Connection pools & timeout policies; reconnect rules.
* **transport/tor.rs** — `arti` transport specifics (onion addrs, bootstrap).
* **cache/mod.rs** — Cache facade, feature-gated.
* **cache/memory.rs** — RAM cache (Micronode default; amnesia-safe).
* **cache/sled_cache.rs** — Ephemeral sled-backed cache (Macronode).

### `tests/`

* **api_smoke.rs** — End-to-end happy path (bootstrap → provide → find_providers).
* **readiness_bootstrap.rs** — `/readyz` reflects bootstrap quorum & min-fill.
* **provider_roundtrip.rs** — Signed provider record admit→store→serve; TTL expiry.
* **kbucket_props.rs** — Property tests: ordering/eviction; single-writer invariant.
* **asn_diversity.rs** — Entropy floor & per-ASN cap under churn scenarios.
* **deadline_hedge.rs** — Deadline budget & hedge stagger reduce tail, never exceed budget.
* **chaos/netem.rs** — Model latency/jitter/loss; tail resilience checks.
* **chaos/partition.rs** — Split-brain & healing; routing table sanity post-merge.
* **chaos/soak_churn.rs** — Long soak (fds/heap stable), RF counters hold under churn.

### `benches/`

* **lookup_bench.rs** — Repro for p50/p95 latencies & hop histograms at α/β profiles.
* **README.md** — How to run/interpret benches; warm-up advice; machine profile.

### `fuzz/fuzz_targets/`

* **msg_frame_decode.rs** — OAP/1 frame decoder fuzz (oversize/malformed rejection).
* **kad_packet_decode.rs** — Kad packet parser fuzz; guards parser UB/panics.

### `loom/`

* **loom_kbucket.rs** — Asserts single-writer bucket + “no lock across `.await`”.
* **loom_hedge.rs** — Fan-out never exceeds inflight caps; deadlines upheld.

### `examples/`

* **find_providers.rs** — Minimal bus client for `ron://dht/find_providers`.
* **provide.rs** — Minimal announce flow with signed record.

### `docs/`

* **IDB.md** — Invariant-Driven Blueprint (lists invariants + how tests enforce).
* **DHT_CONFIG.md** — Ops-focused flag/env table; profiles & examples.
* **PERFORMANCE.md** — Exact SLO repro: seeds, churn, flags, machine profile, commands.
* **arch.mmd / sequence.mmd / state.mmd** — Mermaid sources for diagrams (rendered in CI).

### `scripts/`

* **run-local.sh** — Local runner with sane defaults (bind/metrics/seeds).
* **render-mermaid.sh** — Renders `.mmd` → `.svg` consistently with CI.
* **chaos/netem.sh** — Apply/remove Linux `tc netem` profiles for local chaos tests.
* **chaos/partition.sh** — Simulate network partition topology locally.

### `.github/workflows/`

* **ci.yml** — fmt, clippy, unit/doc/prop tests, coverage (tarpaulin), `cargo deny`, mutation (cargo-mutants).
* **mermaid.yml** — Render Mermaid `.mmd` files to `.svg` on PRs.
* **fuzz.yml** — Time-boxed fuzz runs; corpus upload on crashes.
* **perf.yml** — Benchmarks with thresholds; **fails PR** if SLOs drift (e.g., intra-AZ p95 > 150 ms, hop p99 > 5).

---

## Why this hits your goals

* **Modularity**: each concern has a home; hot paths are small; codec isolated for fuzzing.
* **Maintainability**: profiles + config live in one place; invariants are executable knowledge.
* **Ops-first**: readiness/metrics/gates, perf CI, chaos/soak hooks, and docs SREs can actually use.
* **Perfection gates**: Loom/fuzz/prop/benches/coverage/mutation all have explicit homes so they aren’t afterthoughts.

