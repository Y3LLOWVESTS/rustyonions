

````markdown
---
title: ron-accounting — Invariant-Driven Blueprint
version: 0.3.0
status: reviewed
last-updated: 2025-10-14
audience: contributors, ops, auditors
---

# ron-accounting — IDB

> **Role:** Fast, transient metering for bytes/requests/units → sealed, immutable **time-sliced snapshots** with **bounded resources**, **ordered + idempotent exports** to `ron-ledger`, and **first-class backpressure**.  
> **Not a ledger.** Pillar 12 (Economics & Wallets). Concern focus: **ECON / PERF / RES**.

---

## 1) Invariants (MUST)

- [I-1] **Not a ledger.** No balances, settlement, rerating, or money movement. Only transient counters and immutable slices.
- [I-2] **Monotone in-slice.** Per-key counters are non-negative and never decrease *within a window*. New window → new series.
- [I-3] **Window policy (explicit).** Default fixed windows `5m` (configurable `1m..60m`). All keys share boundaries. Changing window length requires restart.
- [I-4] **Idempotent export.** Slice identity is `(slice_id, b3_digest)`. Resubmission with same identity is a downstream no-op and counted as `exports_total{status="dup"}` locally.
- [I-5] **Ordered exports.** Per `(tenant, dimension)`, exports proceed strictly in `seq` order. If `N+1` is ready but `N` is un-ACKed, `N+1` is buffered (bounded). On overflow, shed later slices with observable rejects rather than violating order.
- [I-6] **Hash-link for forensics.** Each slice includes `prev_b3` for its `(tenant, dimension)` chain. This enables verification without asserting economic truth.
- [I-7] **Bounded everything.** In-mem maps, pending rings, exporter queues, and caches are capacity-limited. On limit, **shed** with structured errors (no unbounded growth or blocking).
- [I-8] **Readiness-first.** Public surfaces rely on readiness; degraded mode fails **write-like** operations first (record/export) with structured 503/overload errors.
- [I-9] **Amnesia honored.** Micronode `amnesia=ON`: RAM-only, unflushed slices may be lost on restart by design; posture exposed in metrics/metadata.
- [I-10] **Persistence when enabled.** Macronode (default): `amnesia=OFF`. Sealed slices are durably staged via **bounded WAL** until downstream ACK; compact on ACK; abide size/age caps.
- [I-11] **Protocol hygiene.** Deterministic encoding (canonical field order; `#[serde(deny_unknown_fields)]`). HTTP/OAP helpers enforce `frame ≤ 1 MiB`, `~64 KiB` chunking; oversize → 413/429 with retry hints.
- [I-12] **Async safety.** No lock held across `.await`. One writer per export stream. Cooperative shutdown is mandatory.
- [I-13] **Golden metrics (once).** Required metrics are registered exactly once and safe to clone.
- [I-14] **Auditable rejects.** All sheds/rejects include at least `{tenant, dimension, reason}` (sampled if needed); never log PII.
- [I-15] **Time anchoring & skew tolerance.** Windows align to **UTC** boundaries with ≤ **500 ms** skew tolerance. If the host clock jumps across a boundary, rollover triggers once; duplicate boundary detections are ignored via a **monotone boundary counter**.
- [I-16] **Counter arithmetic safety.** `inc` uses **saturating add**. On saturation, increment `accounting_overflow_total{dimension}` and continue without panic.
- [I-17] **Node-scope ordering & replay bounds.** `seq` is **node-wide monotone** per `(tenant, dimension)`. Exporters keep a **finite replay horizon** per stream (≥ in-flight + retry). Duplicates beyond horizon are ignored without violating order.

---

## 2) Design Principles (SHOULD)

- [P-1] **Hot-path efficiency.** Power-of-two shards; atomics; branch-light `record()`; avoid heap churn (use `Bytes`/POD).
- [P-2] **Slices-first.** Do the minimum per event; heavy work at rollover. Favor simple deltas in windows.
- [P-3] **Exporter isolation.** Failure scopes to affected `(tenant, dimension)`; sealing and unrelated streams continue.
- [P-4] **Backpressure is visible.** Expose `rejected_total{reason}`, `pending_slices`, `export_backlog`, `degraded`; set alertable thresholds (p95/p99 backlog depth).
- [P-5] **Deterministic & portable.** Canonical encoding yields identical `b3` across platforms (endianness/version).
- [P-6] **Profile-aware defaults.** Micronode defaults `amnesia=ON`; Macronode defaults `amnesia=OFF + WAL`. Effective config is logged at **WARN** on start.
- [P-7] **Dimension extensibility.** Dimensions are compile-time constants (e.g., `const DIM_BYTES: &str = "bytes"`); no hot-path dynamic dispatch.
- [P-8] **Error taxonomy.** CapacityExceeded, DegradedExporter, OversizeFrame, SchemaViolation, PersistenceFull, DuplicateExport, OrderOverflow, WalCorrupt.
- [P-9] **I/O batching.** Defer WAL writes to seal time for batch efficiency; fsync policy is explicit and minimal while meeting durability.

---

## 3) Implementation (HOW)

### [C-1] Core types (sketch)
```rust
pub struct AccountKey { pub tenant: uuid::Uuid, pub subject: u128, pub dimension: &'static str }
pub struct SliceId { pub window_start_s: i64, pub window_len_s: u32, pub seq: u64 } // seq per (tenant,dimension)
pub struct Row { pub key: AccountKey, pub inc: u64 } // non-negative
pub struct SliceMeta { pub prev_b3: [u8;32], pub amnesia: bool, pub host: String, pub version: String }
pub struct Slice { pub id: SliceId, pub rows: Vec<Row>, pub meta: SliceMeta } // #[serde(deny_unknown_fields)]
````

* `dimension` values live in a `dimensions` module as `const &str` to avoid magic strings.
* All adds are **saturating**; overflow increments `accounting_overflow_total{dimension}`.

### [C-2] Sharded counters

* `N = 2^k` shards, `AHashMap`/`DashMap` per shard; fast hash of `(tenant, subject, dimension)`.
* On cap: increment `rejected_total{reason="capacity"}`.

### [C-3] Rollover task

* Cancel-safe tokio task aligns to UTC boundary (with [I-15] guard), seals active shards, builds `Slice`, computes `b3`, attaches `prev_b3`, enqueues into **Pending** (bounded ring).

### [C-4] Exporter

```rust
#[async_trait::async_trait]
pub trait Exporter {
  async fn put(&self, slice: &Slice) -> Result<ExportAck, ExportError>;
}
```

* Maintains per-stream **ordered queue** (by `seq`).
* Retries with **capped, jittered backoff**; classifies `retry_network`, `retry_remote_5xx`, `retry_timeout`.
* Keeps **ACK LRU** for `(slice_id,b3)` (finite replay horizon).
* Enforces order: `N+1` waits for `N`; if backlog exceeds cap → `OrderOverflow` shed with metrics.

### [C-5] WAL (amnesia=OFF)

* **Append-only, length-delimited, checksummed** records (BLAKE3 of body).
* Seal via **atomic rename** (`.tmp` → final); fsync **file on close** and **directory on new file**.
* Caps: total bytes, entries, and age. On ACK → compact (delete or mark reclaimed).
* Recovery: scan; validate checksums; **skip corrupt** frames; resume into Pending; increment `wal_corrupt_total`.

### [C-6] Readiness & health

* `ready = shards_init && rollover_running && exporter_ok`.
* `exporter_ok` is a vector: backlog depth p95 and ack latency p95 under thresholds. Sustained breach flips to degraded.

### [C-7] Metrics (golden + WAL)

* `accounting_record_latency_seconds` (hist)
* `accounting_rejected_total{reason}` (ctr)
* `accounting_pending_slices` (gauge)
* `accounting_export_backlog{tenant,dimension}` (gauge, **sampled** to control cardinality)
* `accounting_exports_total{status=ok|dup|retry_network|retry_remote_5xx|fail}` (ctr)
* `accounting_degraded` (gauge 0/1)
* `accounting_overflow_total{dimension}` (ctr)
* `accounting_wal_size_bytes` (gauge), `accounting_wal_entries` (gauge), `accounting_wal_corrupt_total` (ctr)

### [C-8] HTTP/OAP helpers

* Enforce frame/chunk invariants; 413/429 with `Retry-After` hints. Deterministic DTOs, deny unknowns.

### [C-9] Async patterns

* No `.await` while holding a lock; `CancellationToken` for shutdown; metrics via `OnceLock`; one writer per export connection.

### [C-10] Bus hooks (optional)

* Publish `SliceSealed{slice_id,b3}` and `SliceExported{slice_id}`. Bounded broadcast; overflow → drop oldest + counter.

### [C-11] Fairness policy

* Shed policy: **FIFO** within a stream; cross-tenant: round-robin or WFQ-ish rotation to avoid starvation during exporter backlog.

---

## 4) Acceptance Gates (PROOF)

**Properties & Core**

* [G-1] **Monotonicity property.** Within a window, per-key counters never decrease; negative inc rejected.
* [G-2] **Rollover determinism.** Given fixed config and event stream, two runs produce identical encodings and `b3`.
* [G-3] **Idempotent export.** Replaying `put()` N times for same `(slice_id,b3)` yields single durable entry; local `dup` counts increment.

**Ordering & Replay**

* [G-4] **Ordered export.** Force `N+1` ready before `N`; verify hold-and-wait, or `OrderOverflow` shed when cap exceeded.
* [G-16] **Forked-exporter race.** Two exporters race on same stream; downstream accepts exactly one ordered sequence; duplicates are safe no-ops.
* [G-17] **Replay horizon.** Flood with old duplicates beyond horizon; ensure no order violation, no unbounded state.

**Backpressure & Capacity**

* [G-5] **Backpressure.** Tiny caps → sustained input sheds without deadlock; readiness flips degraded and recovers.
* [G-11] **Perf smoke.** Record p95 ≤ 20 µs; seal 10k rows ≤ 50 ms; exporter sustains ≥ X slices/s without memory growth (X set per CI host).
* [G-22] **Tenant hotspot fairness.** One tenant at 10× load cannot starve others; per-tenant backlog bounded and rotates fairly.

**Amnesia & WAL**

* [G-6] **Amnesia matrix.** `amnesia=ON`: restart loses unflushed by design. `amnesia=OFF`: restart restores pending from WAL; digests unchanged.
* [G-7] **WAL crash-safety.** Kill -9 mid-flush; on restart WAL replays exactly once; compaction after ACK.
* [G-18] **Torn-write & bit-flip.** Corrupt a frame; recovery skips damaged record, increments `wal_corrupt_total`, continues.
* [G-19] **Overflow property.** Force near-`u64::MAX`; verify saturating add and `accounting_overflow_total{dimension}` increments.

**Protocol/DTO & Cross-platform**

* [G-6.1] **DTO hygiene.** Deny unknown fields; framing limits enforced; oversize returns 413/429 with retry hints.
* [G-8] **Cross-arch parity.** x86_64 vs aarch64 yield identical `b3`.

**Chaos & Time**

* [G-9] **Chaos exporter.** Inject 10–30% network failures/latency; system seals; sheds predictably on overflow; drains when healthy.
* [G-15] **Clock skew & jumps.** Simulate ±250 ms drift and abrupt ±2 s jumps across boundaries; exactly one rollover per boundary; identical artifacts pre/post jump.

**End-to-End & Config**

* [G-10] **E2E integration.** Record → Seal → Export to mock ledger → Verify digest, idempotence, ordering, metrics.
* [G-21] **Config hot-flip denial.** Attempt runtime change to `window_len_s` is rejected and logged; process requires restart.

---

## 5) Anti-Scope (Forbidden)

* No balances/settlement/wallet logic; no economic truth or re-rating.
* No unbounded maps/queues; no global locks across awaits.
* No orchestration/service runner creep—library only; adapters live outside.
* No PII in slices/metadata; keys are opaque identifiers.
* No crypto key custody; defer to KMS/ledger layers.

---

## 6) References

* Pillars & Six Concerns (focus: ECON, PERF, RES)
* Hardening & Scaling blueprints (limits, degraded/readiness, perf SLOs)
* OAP/HTTP envelope limits (1 MiB frame, ~64 KiB chunking)
* BLAKE3 spec for content digests
* Async Rust best practices (no lock across await; cooperative shutdown)
* Exponential backoff with jitter (industry standard guidance)

---

## Appendix A — Config Schema (suggested)

```toml
[accounting]
window_len_s = 300          # 5m; restart required to change
shards = 64                 # power of two
capacity_rows = 200_000     # in-flight row cap across shards
pending_slices_cap = 8_192  # bounded ring
amnesia = false             # Micronode default = true; Macronode default = false

[accounting.exporter]
backoff_base_ms = 50
backoff_cap_ms  = 5_000
jitter = true
ordered_buffer_cap = 1_024  # per (tenant,dimension)

[accounting.wal]
enabled = true
max_bytes = "512MiB"
max_entries = 200_000
max_age_s = 86_400
fsync_on_close = true
fsync_dir_on_create = true
```

**Effective config** MUST be logged at WARN on startup. Attempts to hot-flip `window_len_s` must be rejected and logged ([G-21]).

---

## Appendix B — Dimensions (example)

```rust
pub mod dimensions {
    pub const BYTES: &str = "bytes";
    pub const REQUESTS: &str = "requests";
    pub const CPU_UNITS: &str = "cpu_units"; // future-friendly
}
```

```

