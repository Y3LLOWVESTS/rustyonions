---

crate: accounting
path: crates/accounting
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Tiny, std-only byte accounting: a `CountingStream<S>` wrapper and shared `Counters` that track total bytes in/out and rolling per-minute buckets (60-slot ring).

## 2) Primary Responsibilities

* Provide a thread-safe in-memory counter for bytes read/written (totals + 60-minute ring).
* Offer a zero-alloc snapshot API for quick export to logs/metrics.
* Make it trivial to instrument any `Read`/`Write` with `CountingStream<S>`.

## 3) Non-Goals

* No Prometheus/HTTP export layer (consumers must expose).
* No quotas/rate-limiting or policy enforcement.
* No persistence or cross-process aggregation.

## 4) Public API Surface

* **Re-exports:** (none)
* **Key types / functions / traits:**

  * `CountingStream<S>`: wraps any `S: Read/Write` and increments counters on IO.

    * `CountingStream::new(inner, counters)`
    * `CountingStream::into_inner()`
    * `CountingStream::counters() -> Counters`
    * Implements `Read` and `Write` (increments on successful ops).
  * `Counters(Arc<Mutex<State>>)`:

    * `Counters::new()`
    * `add_in(u64)`, `add_out(u64)`
    * `snapshot() -> Snapshot`
    * `reset_minutes()` (preserves totals, clears ring)
  * `Snapshot`:

    * `total_in`, `total_out`
    * `per_min_in: [u64; 60]`, `per_min_out: [u64; 60]`
* **Events / HTTP / CLI:** None.

## 5) Dependencies & Coupling

* **Internal crates:** none (intentionally standalone). Stability: **tight isolation**, replaceable: **yes**.
* **External crates (top):**

  * `anyhow` (workspace): not required by current code path; low risk, MIT/Apache.
  * `parking_lot` (workspace): declared; current code uses `std::sync::Mutex`; consider switching. Mature, MIT.
  * `workspace-hack`: build graph hygiene only.
* **Runtime services:** Uses OS time (`SystemTime`) to bucket by epoch minute; no network, storage, or crypto.

## 6) Config & Feature Flags

* **Env vars:** none.
* **Config structs:** none.
* **Cargo features:** none.
* Effect: behavior is deterministic apart from system clock.

## 7) Observability

* In-memory counters + `snapshot()`; no direct logging or metrics export.
* No `/healthz`/`/readyz`; not a service.
* Intended for consumers (gateway/services) to scrape/emit.

## 8) Concurrency Model

* Shared state behind `Arc<Mutex<State>>` (single small critical section per update).
* Minute rotation occurs on write/read accounting when the observed minute changes; intermediate buckets zeroed.
* No async tasks/channels; backpressure is the caller’s concern.
* No internal timeouts/retries; purely local mutation.

## 9) Persistence & Data Model

* **Persistence:** none (process-local).
* **Data model:** `State { total_in, total_out, ring_in[60], ring_out[60], idx, last_minute }`.
* **Retention:** last 60 minutes of per-minute counts.

## 10) Errors & Security

* **Error taxonomy:** updates are infallible; IO errors come from wrapped `Read/Write` and pass through. Mutex poisoning possible with `std::sync::Mutex` (switching to `parking_lot::Mutex` avoids poisoning semantics).
* **Security:** no secrets, authn/z, TLS, or network I/O here. PQ-readiness N/A.

## 11) Performance Notes

* **Hot path:** `add_in/add_out` called on every successful read/write; O(1) updates and occasional minute-rotation work.
* **Targets:** micro-overhead suitable for per-request and stream accounting.
* **Potential wins:** replace `std::sync::Mutex` with `parking_lot::Mutex`; optionally split totals and ring into separate locks or move to atomics with a clock-advance CAS design if contention appears.

## 12) Tests

* **Unit:** verifies counting and snapshot rotation; verifies `reset_minutes` clears rings but preserves totals.
* **Integration/E2E:** none here (should be covered by consumers).
* **Fuzz/loom:** none.

## 13) Improvement Opportunities

* **Switch to `parking_lot::Mutex`** (already a dependency) to avoid poisoning and improve perf.
* **Document bucket semantics** (index 0 meaning, ordering in `Snapshot`, and how gaps are zeroed) to remove ambiguity for dashboards.
* **Add a light Prometheus adapter** (optional module or feature) to export counters without coupling other crates.
* **Clock abstraction for tests** (injectable time source) for deterministic rotation tests.
* **Atomic design option** if needed: per-minute cell as `AtomicU64` + epoch-minute guard, trading complexity for contention reduction.
* **Remove unused deps** (confirm `anyhow` usage; if unused, drop).

## 14) Change Log (recent)

* 2025-09-14 — Ring buffer minute reset preserves totals; rotation behavior validated by unit tests.
* 2025-09-14 — `CountingStream` wires `Read/Write` to counters; snapshot includes 60-minute arrays.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Small surface, could use more doc comments/examples.
* **Test coverage:** 2 — Core behavior covered; needs more edge cases and time abstraction.
* **Observability:** 2 — Snapshots exist; no exporter.
* **Config hygiene:** 4 — No config; predictable defaults.
* **Security posture:** 4 — In-proc counters; minimal risk.
* **Performance confidence:** 3 — Fine for now; lock choice can be improved.
* **Coupling (lower is better):** 1 — Self-contained and easily swappable.
