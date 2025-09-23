
# ✅ Recommendation (short)

**Do it.**

* **Fold `svc-arti-transport` into `ron-transport`** behind an `arti` feature flag.
* **Introduce `svc-dht`** (a runtime service that owns Kademlia/Discv5).
  Net effect: still **33 crates**, cleaner module boundaries, better testability, faster builds, and fewer drift vectors.

---

# Why this merge is correct

## 1) Architectural cohesion (transport belongs in one place)

* **What `svc-arti-transport` does** is a *transport backend* (Tor/Arti SOCKS5 + hidden-service plumbing).
* **What `ron-transport` is** is your *transport abstraction* (TCP/TLS/timeouts/idle).
  Unifying them reflects the design truth: both are the **same axis** (I/O substrate), not different services. This preserves the microkernel invariant: *transport is a pluggable platform facility, not a standalone runtime role*.

**Benefits**

* Keeps the kernel surface small and orthogonal.
* **Feature-gated weight:** Arti’s large dependency tree compiles **only** when `--features arti` is enabled.
* Removes an entire crate rebuild boundary (fewer crate graph edges ⇒ faster incremental builds).

**Risk & Mitigation**

* *Risk:* `ron-transport` bloats.
  *Mitigation:* isolate under `ron_transport::arti::*` with a separate `src/arti/` module tree and **no default features**. CI builds both “tiny” (no arti) and “arti-on”.

---

## 2) DHT must be first-class (modularity, testability, federation)

Your overlay currently “implies” a DHT. That works—but it turns `svc-overlay` into a magnet for scope creep. A dedicated **`svc-dht`**:

* **Elevates DHT to a pluggable service** (PING/PONG, FIND\_NODE, FIND\_VALUE/STORE/PROVIDE, replication, provider records, Discv5 discovery).
* **Enables independent fuzz/proptest/loom** on routing tables, bucket churn, hedged queries, TTL/republish, and eclipse-resistance—including **10k-node sims** required by your Perfection Gates.
* **Strengthens federation**: lets you swap/bridge DHTs (e.g., run pilots, interop with external discovery substrates) without touching overlay routing/gossip.

**Why `svc-dht` (service) vs `ron-dht` (lib)?**

* Today you need a *runtime* peer that talks to a swarm and exposes an API to the rest of the mesh. That’s a **service role**, not just a helper library.
* You can still **nest a `dht` module** inside the service for algorithmics (clean split *within* the crate), and promote to a lib later—*without increasing crate count*.

---

# Six-Concerns alignment (no gaps introduced)

* **Security & Privacy (SEC):**
  `ron-transport` keeps PQ/TLS hooks; `svc-dht` enforces capability-gated RPC, rate caps, eclipse/Sybil guardrails (ID binding to passports, randomized NodeIDs, onion optionality per hop).
* **Resilience & Concurrency (RES):**
  `svc-dht` gets its own supervisor tree, backoff, single-writer bucket discipline; overlay shrinks and hardens.
* **Performance & Scaling (PERF):**
  Parallel α-queries, hedged lookups, bounded fan-out, adaptive timeouts—now testable in isolation. Transport merge reduces crate graph overhead.
* **Economic Integrity (ECON):**
  Per-RPC quotas/fees on DHT ops (STORE/PROVIDE) become enforceable policy (no free spam).
* **DX & Interop (DX):**
  Clear SDK targets: `dht_find`, `dht_store`, `dht_providers`; simpler “enable Tor” by `--features arti`.
* **Governance & Ops (GOV):**
  Dedicated runbook + golden metrics for DHT (bucket counts, hop histograms, success latency, churn), plus policy switches (disable PROVIDE, cap REPLICATE, maintenance mode).

---

# Concrete migration plan (bullet-proof & low-risk)

## A) Merge `svc-arti-transport` → `ron-transport`

1. **Code move:** `svc-arti-transport/src/**/*` → `ron-transport/src/arti/**/*`.
2. **Cargo features:**

   ```toml
   [features]
   default = []
   arti = ["dep:arti-client", "dep:tor-rtcompat", "dep:socks5-client"]
   ```

   Keep Arti deps **optional**; no default.
3. **Public API shape:**

   * Keep `TransportConfig` stable.
   * Add `Endpoint::Onion { v3_id, hs_port, ... }` or `TransportTarget::Tor(SocksTarget)` variants—*feature-gated*.
   * Provide `spawn_tor_transport(cfg, ...)` only under `arti`.
4. **Callers:** Replace `svc_arti_transport` deps with `ron-transport` + `features = ["arti"]`.
5. **Observability:** Add histograms/gauges for circuits, socks connect latency, hs publish time, failure taxonomy.
6. **CI Matrix:**

   * `tiny`: no features (sanity, MSRV).
   * `arti`: build/tests with `--features arti`.
   * deny/clippy/loom where applicable.

## B) Add `svc-dht` (service)

1. **Responsibilities:** Kademlia routing table + Discv5 discovery; ops: `PING/PONG`, `FIND_NODE`, `FIND_VALUE`, `STORE`, `PROVIDE`, `REPLICATE`, `DELETE (policy-gated)`.
2. **Interfaces:**

   * **Bus topics:** `DhtEvent::{PeerJoined,PeerLeft,ValueFound,ProvidersFound,Replicated}`.
   * **OAP endpoints:** `/dht/find`, `/dht/store`, `/dht/providers`.
   * **Config:** `k_bucket`, `alpha`, `parallelism`, `ttl`, `republish_interval`, `hedge_after`, `rate_limits`.
3. **Security:**

   * Capabilities per op (READ vs WRITE).
   * Anti-eclipse: diversified peer sampling, prefix-bounded inserts, per-prefix quotas.
   * Optional onion routing per query when `arti` is enabled in `ron-transport`.
4. **Concurrency model:**

   * One writer per k-bucket; mpsc for ops; timer task for republish/refresh.
   * Bounded queues (drop + metric on backpressure).
5. **Metrics (golden):**

   * `dht_lookup_hops`, `dht_lookup_latency_seconds`, `dht_bucket_occupancy`, `dht_republish_total`, `dht_evict_total`, `dht_rate_limited_total`.
6. **Tests/Proofs:**

   * Property tests: lookup completeness ≤ H hops under churn; replication ≥ R; TTL expiry correctness.
   * Chaos sim: churn %, partition healing time, eclipse resistance.
   * TLA+ model sketch for PROVIDE/REPLICATE invariants (optional but on-brand).

---

# Impact on the rest of the mesh

* **`svc-overlay`** shrinks (no longer hosts DHT state machine), focuses on **session routing + gossip**. Fewer coupled concerns → fewer Heisenbugs.
* **`svc-index` / `svc-storage`** can call `svc-dht` for provider discovery; cleaner “who does what” lines.
* **`ron-app-sdk`** gets straight-through `dht_*` calls (with retries & tracing) → better DX.
* **Build times** improve (one crate less, heavy Arti code gated).
* **Docs** become clearer: your Six-Concerns mapping for `svc-dht` is explicit (SEC/RES/PERF/ECON/GOV), and `ron-transport` remains the single transport home.

---

# QRNG note (bonus perfection)

While not required to ship the merge, add a **`qrng` feature to `ron-kms`** with a trait `EntropySource` (OS + optional quantum backends). Use it for keygen/nonces; keep OS entropy default. It upgrades you from “PQ-ready” to “PQ-native” without destabilizing anything.

---

# Final verdict

* **Merge `svc-arti-transport` into `ron-transport`** under a feature flag.
* **Create `svc-dht`** as a first-class service.
  This is the cleanest path to **more modularity, stronger testability, faster builds, and perfect blueprint alignment**—
