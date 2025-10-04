

```markdown
---
title: svc-index — Invariant-Driven Blueprint (IDB)
version: 0.1.2
status: final
last-updated: 2025-10-03
audience: contributors, ops, auditors
---

# svc-index — IDB

**Role:** Thin, read-optimized resolver that maps **names** and **`b3:<hex>` content IDs** → **manifests** → **provider sets**.  
**Depends on:** `ron-naming` (types), `ron-proto` (DTOs), `svc-dht` (discovery), `ron-audit` (writes/audit), kernel bus/metrics.  
**Non-goals:** Storing content blobs, implementing DHT internals, overlay logic.

---

## 1) Invariants (MUST)

- **[I-1] Canon boundaries.** `svc-index` provides resolution and provider ranking only. Types/schemas live in `ron-naming`/`ron-proto`. DHT routing lives in `svc-dht`. No kernel/bus primitives are reimplemented here.
- **[I-2] Content addressing.** Identifiers are names or **BLAKE3** CIDs as `b3:<hex>`. Before returning any manifest-derived data, the crate must verify the full digest for any locally cached manifest bytes.
- **[I-3] OAP/1 limits.** Respect protocol invariants: **max_frame = 1 MiB**; storage/streaming chunk size guidance ~**64 KiB** (not a wire limit). Responses fit within these bounds.
- **[I-4] Discovery discipline.** All provider discovery flows through `svc-dht` with bounded fanout: define α (parallel first-wave queries) and β (hedged second wave) and **deadline budgets**. Target **p99 ≤ 5 hops**.
- **[I-5] Concurrency hygiene.** Never hold a lock or a database iterator across `.await`. Copy owned data before async I/O.
- **[I-6] Hardening defaults.** Per-route **timeouts**, **inflight caps**, **body caps (1 MiB)**, and **structured rejects** (400/404/413/429/503). Metrics label cardinality is bounded (no unbounded label values).
- **[I-7] Truthful readiness.** `/readyz` returns 200 only when **config loaded**, **index DB open**, **DHT client connected**, and **bus subscriptions** are active. Reads fail-open only within configured guardrails; all writes/administration fail-closed.
- **[I-8] Amnesia mode.** When enabled, no on-disk spill except allowed ephemeral caches; periodic purge; best-effort zeroization of sensitive material. All metrics include `amnesia_mode{status="on|off"}`.
- **[I-9] Metrics canon.** Expose canonical histograms/counters/gauges: `request_latency_seconds`, `rejected_total{reason}`, `index_resolve_latency_seconds{type}`, `index_dht_lookup_ms`, `index_cache_{hits,misses}_total`, `inflight_requests{route}`.
- **[I-10] Audited mutation.** Any mutating admin path (reindex/backfill/pin/unpin) emits structured audit events via `ron-audit`.
- **[I-11] Capability enforcement.** Admin and facet (e.g., Search/Graph) queries MUST verify capabilities (macaroons/caps) via `ron-auth`. No ambient trust from ingress.
- **[I-12] Single index truth.** Service operates against one configured **RON_INDEX_DB** root only. No multi-root or per-tenant divergence within a single instance.
- **[I-13] Deterministic DTOs (consumer stance).** Use **ron-proto** DTOs; tests enforce `serde(deny_unknown_fields)` when deserializing external input. Any schema change is driven by **ron-proto** semver and migration notes; `svc-index` follows, never forks schemas.
- **[I-14] Negative caching & breakers.** Cache NOT_FOUND with short TTL; per-target circuit breakers for flapping providers to prevent herds.
- **[I-15] Privacy by construction.** The index layer stores **no PII**; provider hints are limited to non-identifying attributes (region, freshness, latency class).
- **[I-16] Logging hygiene.** Structured JSON logs; no secrets in logs; redaction on known-sensitive fields.
- **[I-17] Optional UDS is gated.** If a **`uds`** feature/flag is enabled, enforce `0700` dir / `0600` socket and **SO_PEERCRED** allow-list. If disabled, no UDS is exposed.

---

## 2) Design Principles (SHOULD)

- **[P-1] Read-optimized, write-audited.** Immutable indexes + append-only logs; compactions are explicit, rate-limited, and observable.
- **[P-2] Thin control plane.** Resolve → DHT → rank → return. Anything heavier (facet search/graph) sits behind **feature flags** with separate executors/semaphores.
- **[P-3] Backpressure early.** Apply quotas and inflight caps before touching disk or DHT; degrade readiness before collapse.
- **[P-4] Hedged queries with jitter.** After T ms, launch β hedges; cancel on first satisfactory quorum; add small jitter to avoid pattern lockstep.
- **[P-5] Config uniformity.** Read knobs from kernel config (timeouts, caps, α/β, hedge_ms, cache sizes) with sane defaults; hot-reload safely (announce `ConfigUpdated` on the bus).
- **[P-6] Economic hooks (optional).** If enabled, emit **usage counters only**; **all economic truth** (pricing, billing, settlement) **defers to `ron-ledger`/`ron-accounting`**.
- **[P-7] Deterministic caching.** Use size-bounded LRU with TTLs; expose cache stats; guard against unbounded key growth.
- **[P-8] Minimal allocations.** Prefer zero-copy reads (`Bytes`/`Cow`) and reuse buffers on hot paths; but never at the cost of [I-5].

---

## 3) Implementation (HOW)

### [C-1] Endpoints (HTTP/OAP)
```

GET  /resolve/{id}         # id ∈ {name, b3:<hex>}
GET  /providers/{b3hex}    # providers + freshness/region/latency class
POST /admin/reindex        # capability-gated; audited
GET  /metrics
GET  /healthz
GET  /readyz
GET  /version

````

### [C-2] Resolve pipeline (pseudo-Rust)
```rust
async fn resolve(id: Id) -> Result<Resolved, Error> {
    let key = parse_id(id)?;              // name -> cid pointer or cid
    if let Some(ans) = cache.get(&key) { return Ok(ans); }

    // DHT lookup with bounded α/β and deadlines
    let first = dht.lookup(&key)
        .with_deadline(T1)
        .with_parallelism(alpha)
        .await?;

    let combined = if first.satisfactory() {
        first
    } else {
        let hedged = dht.lookup(&key)
            .with_deadline(T2)
            .with_parallelism(beta)
            .await?;
        hedged.merge(first)
    };

    let ranked = rank_providers(combined, policy_hints());
    let resolved = Resolved { key: key.clone(), providers: ranked };
    cache.put(key, resolved.clone(), ttl_for(&resolved));
    Ok(resolved)
}
````

### [C-3] Config (toml sketch)

```toml
[index]
max_inflight = 512
timeout_ms = 5000
body_limit_bytes = 1048576

[dht]
alpha = 3
beta = 2
hedge_ms = 150
deadline_ms = 1200

[cache]
entries = 100_000
ttl_ms = 10_000
neg_ttl_ms = 2_000

[amnesia]
enabled = false
purge_interval_ms = 30_000

[uds]
enabled = false
path = "/run/ron/index.sock"
```

**Env knobs (recommended):** `RON_INDEX_DB`, `RON_INDEX_ALPHA`, `RON_INDEX_BETA`, `RON_INDEX_HEDGE_MS`, `RON_INDEX_TIMEOUT_MS`, `RON_INDEX_MAX_INFLIGHT`, `RON_AMNESIA=on|off`.

### [C-4] DB layout (keyspace sketch, backend-agnostic)

```
name/<name>                -> b3:<hex> (manifest pointer)
manifest/<b3>              -> manifest bytes (optional cache)
providers/<b3>             -> provider set (freshness, region, latency class)
meta/version               -> schema version
```

### [C-5] Metrics (golden taxonomy)

* `index_resolve_latency_seconds{type=name|cid}`
* `index_dht_lookup_ms`
* `index_cache_hits_total`, `index_cache_misses_total`, `index_cache_size`
* `rejected_total{reason}`, `inflight_requests{route}`
* `amnesia_mode{status="on|off"}` (gauge)

### [C-6] Facets (feature-gated executors)

* **Search**: ingest via mailbox workers; p95 query ≤ 150 ms; ingest lag p95 < 5 s.
* **Graph**: neighbor lookup p95 ≤ 50 ms intra-AZ.
* Both paths must enforce [I-11] capabilities and reuse the same backpressure primitives.

---

## 4) Acceptance Gates (PROOF)

* **[G-1] Unit/property/fuzz**

  * Property tests for name→cid→providers round-trip (bad inputs fuzzed).
  * Fuzz parsers for `b3:<hex>` and DTOs (arbitrary **JSON/CBOR** variants where supported); `serde(deny_unknown_fields)` enforced in tests.
* **[G-2] Concurrency**

  * Loom/async tests proving no lock/iterator is held across `.await` on hot paths.
* **[G-3] DHT integration sims**

  * Simulate α/β + hedging to prove p99 ≤ 5 hops and bounded deadlines; cancellation is drop-safe.
* **[G-4] Hardening self-tests**

  * 2 MiB request → **413**; 600+ concurrent requests hit inflight caps and emit `rejected_total{reason="over_capacity"}`; `/readyz` blocks until DB+DHT+bus are up.
* **[G-5] Perf baselines (reproducible)**

  * Criterion harness with fixed dataset + latency injectors:

    * Micronode local p95: ≤ 50–80 ms.
    * Regional p95: ≤ 150–200 ms with hop bound upheld.
  * Document machine profile and `cargo bench` invocation in `docs/ALL_DOCS.md`.
* **[G-6] Chaos/soak + exhaustion**

  * 24h soak with fault injection (DHT flaps, slow disk), plus **OOM/FD exhaustion** sims show no leaks and scheduled amnesia purges.
* **[G-7] Schema drift gate**

  * CI guard for DTO compatibility (public API diff) and `serde(deny_unknown_fields)` enforced in tests.
* **[G-8] Security & licensing**

  * cargo-deny (licenses, advisories, bans) green; no `unsafe` in this crate.
* **[G-9] Audit verification**

  * Mutating endpoints produce exactly one audit event per op with required fields (who, when, what, capability id).

---

## 5) Anti-Scope (Forbidden)

* Implementing DHT routing tables or bucket maintenance (use `svc-dht`).
* Storing content bytes or acting as a blob store.
* Adding kernel/bus primitives; only consume existing kernel interfaces.
* Accepting requests without capability checks on admin/facets.
* Conflating OAP frame limit (1 MiB) with storage chunk size (~64 KiB).
* Unbounded metric label values or logs with secrets.
* Enabling UDS without the `uds` feature/flag and associated perms.

---

## 6) References

* Full Project Blueprint; 12_PILLARS; SIX_CONCERNS; HARDENING_BLUEPRINT; SCALING_BLUEPRINT; CONCURRENCY_AND_ALIASING_BLUEPRINT.
* OMNIGATE/INTEROP blueprints (OAP/1 constants, DTO norms).
* Crate docs: **ron-naming**, **ron-proto**, **svc-dht**, **ron-audit**, **ron-ledger**/**ron-accounting** (if ECON hooks enabled).

```
