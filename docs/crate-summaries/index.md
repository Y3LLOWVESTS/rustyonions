---

crate: index
path: crates/index
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny library that maintains and queries the mapping from a content address (`b3:<hex>`) to currently known providers, enforcing TTLs and signatures, so callers can resolve where to fetch bytes without speaking the DHT directly. &#x20;

## 2) Primary Responsibilities

* Resolve `b3:<hex>` → provider list (node\_id + addr), honoring TTL and limits.&#x20;
* Accept provider announcements and validate Ed25519 signatures + expirations.&#x20;
* Offer a fast, local cache and persistence backing used by `svc-index` and tools like `tldctl pack`. &#x20;

## 3) Non-Goals

* Running a network-facing API (that’s `svc-index`; this crate is the embeddable core).&#x20;
* Implementing DHT lookups or peer gossip (delegated to `svc-dht`/discovery).&#x20;
* Storing or serving object bytes (that’s Storage/Overlay).&#x20;

## 4) Public API Surface

* **Re-exports:** (none yet; keep minimal)
* **Key types / fns (proposed for clarity):**

  * `Provider { node_id: String, addr: SocketAddr, expires_at: u64, sig: Vec<u8> }` (matches announce schema).&#x20;
  * `Index::announce(hash: &str, provider: Provider) -> Result<()>` (validates TTL/signature).&#x20;
  * `Index::resolve(hash: &str, limit: usize) -> Vec<Provider>` (sorted/stable, cap by `limit`).&#x20;
  * `Index::gc(now_epoch: u64)` (purges expired records).
* **Events / HTTP / CLI:** none in the lib; `svc-index` exposes `GET /resolve` and `POST /announce` using this crate.&#x20;

## 5) Dependencies & Coupling

* **Internal crates:**

  * *ron-kernel* (loose; for health/metrics patterns if embedded) — replaceable: **yes**.&#x20;
  * *svc-index* (service wrapper uses this lib; tight but directional).&#x20;
* **External crates (expected top 5):**

  * `sled` (embedded KV store for provider sets). Risk: moderate (maintenance), permissive license. (Implied by `RON_INDEX_DB` workflows.)&#x20;
  * `blake3` (addressing; verification alignment). Low risk.&#x20;
  * `ed25519-dalek` (signature verification on announces). Moderate risk; widely used.&#x20;
  * `serde` (+`rmp-serde`/`rmpv` for compact storage) — low risk. (Used broadly across services.)&#x20;
  * `parking_lot` (locks) — low risk.
* **Runtime services:** local disk (Sled DB), system clock (TTL), optional bus for health. &#x20;

## 6) Config & Feature Flags

* **Env vars:** `RON_INDEX_DB` → path to the Sled database used by pack/index/services (must match across tools).&#x20;
* **Cargo features (suggested):**

  * `verify-sig` (on by default) toggles Ed25519 checks.&#x20;
  * `inmem` switches to an in-memory map for tests.
* **Constants alignment:** `b3:<hex>` addressing, OAP/1 `max_frame = 1 MiB` (docs alignment; not used directly here). &#x20;

## 7) Observability

* Expected to surface counters via the host service (`svc-index`): `requests_total{code}`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`.&#x20;
* Health/readiness endpoints exist at the service layer; the lib should expose cheap `stats()` for wiring.&#x20;

## 8) Concurrency Model

* Library is synchronous over a `RwLock` around the store (Sled is thread-safe); no background tasks required.
* Backpressure and timeouts live in the service layer; the lib enforces `limit` and TTL checks to keep ops O(log n) per key. (Service backpressure/429 rules per blueprint.)&#x20;

## 9) Persistence & Data Model

* **Store:** Sled at `RON_INDEX_DB`. Keep keys small; one key per object hash.&#x20;
* **Suggested keys:**

  * `prov/<b3hex>` → `Vec<Provider>` (sorted by freshness; dedup by `node_id`).
  * `meta/<b3hex>` → compact stats (optional).
* **Retention:** purge on read or periodic GC using `expires_at` from announces.&#x20;

## 10) Errors & Security

* **Error taxonomy (service level):** map to JSON envelope `{code,message,retryable,corr_id}`; 400/404/413/429/503. (Lib returns typed errors for the service to translate.)&#x20;
* **Security controls:**

  * Verify announce signatures (Ed25519) and reject expired records.&#x20;
  * Addressing is **BLAKE3-256** `b3:<hex>`; services must verify digest before returning bytes. (Consistency with the rest of the system.)&#x20;
* **PQ-readiness:** none required here; PQ plans land in OAP/2 and e2e layers.&#x20;

## 11) Performance Notes

* Hot path is `resolve(hash, limit)` — aim for p95 < 40 ms intra-node (matches internal API SLOs).&#x20;
* Keep provider vectors small (≤ 16) per `limit` to avoid copying cost.&#x20;

## 12) Tests

* **Unit:** TTL pruning; signature validation; dedup/sort order.
* **Integration:**

  * With `svc-index` HTTP: `/resolve` limit handling; `/announce` rejects bad sigs/expired TTLs.&#x20;
  * With `tldctl` pack → resolve → gateway fetch (ensures a single `RON_INDEX_DB` path to kill phantom 404s).&#x20;
* **E2E:** covered indirectly by `gwsmoke` (Gateway↔Index↔Overlay↔Storage read path).&#x20;
* **Fuzz/loom:** N/A (lib is data-structure focused); optional proptests for announce/resolve invariants.

## 13) Improvement Opportunities

* **Gaps / tech debt:**

  * No canonical on-disk schema defined yet (document key prefixes + version byte).
  * Sled-lock footgun when tools and daemon share the DB; prefer daemonized access.&#x20;
* **Overlap & redundancy signals:**

  * Announce/resolve logic is also sketched in `svc-discovery`/DHT; ensure this lib remains the **single source** for provider-record validation to avoid drift.&#x20;
* **Streamlining:**

  * Add an in-proc cache layer (LRU) with TTL to reduce Sled hits.
  * Provide a tiny UDS façade so `tldctl` can talk to the daemon instead of opening Sled directly (eliminates lock contention).&#x20;
  * Ship golden metrics in `svc-index` using the standard set.&#x20;

## 14) Change Log (recent)

* **2025-09-14** — First deep-dive crate analysis; aligned with `svc-index` spec (resolve/announce, TTL/sig).&#x20;
* **2025-09-05–09-06** — Gateway↔Index↔Overlay read path proven by `gwsmoke`; integration tests for gateway read-path landed (context for index consumers). &#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — spec is crisp, but types/traits need to be finalized in-code.&#x20;
* **Test coverage:** 2 — E2E exists around it; direct unit/integration for this lib are TBD.&#x20;
* **Observability:** 3 — patterns defined; needs wiring in `svc-index`.&#x20;
* **Config hygiene:** 3 — `RON_INDEX_DB` pattern is established; formal config struct not yet standardized.&#x20;
* **Security posture:** 3 — Ed25519 verify + TTL planned; PQ not in scope here.&#x20;
* **Performance confidence:** 3 — local lookups should meet SLOs; needs micro-bench + LRU.&#x20;
* **Coupling (lower is better):** 2 — clean separation from DHT and gateway; used by `svc-index`.&#x20;

