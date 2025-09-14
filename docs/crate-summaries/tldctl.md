---

crate: tldctl
path: crates/tldctl
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Library + thin CLI for **packing local files into content-addressed bundles** (BLAKE3 `b3:<hex>`) with manifest and precompressed variants, writing to the store root and index DB, and printing the canonical address.  &#x20;

## 2) Primary Responsibilities

* **Pack**: build a bundle (`Manifest.toml`, `payload.bin`, optional `.br`/`.zst`) and compute the BLAKE3 address (`b3:<hex>.<tld>`). &#x20;
* **Persist**: write bundle to a **store root** (e.g., `.onions`) and update **Index DB** at the configured sled path.&#x20;
* **Report**: print the final address to STDOUT for downstream use (e.g., Gateway GET).&#x20;

## 3) Non-Goals

* No HTTP service surface, quotas, or readiness/metrics (this is a local packer, not a server). (Implied by CLI usage only.)&#x20;
* No DHT/provider routing, overlay streaming, or storage durability; those live in services (`svc-index`, `svc-overlay`, `svc-storage`).&#x20;
* No application-level crypto; it uses **content addressing** only (BLAKE3 `b3:<hex>`).&#x20;

## 4) Public API Surface

* **Re-exports:** none (internal lib consumed by its own bin + tests using “reuse the packing routine”).&#x20;
* **Key functions (implied by usage & docs):**

  * `pack(input, tld, index_db, store_root) -> PackedAddr` (builds bundle, writes to store/index, returns `b3:<hex>.<tld>`).&#x20;
  * Helpers for **precompression** (`.br`, `.zst`) and manifest emission (`Manifest.toml`).&#x20;
* **CLI (bin target):** `tldctl pack --tld <text|…> --input <file> --index-db <path> --store-root <dir>`; prints the computed address.&#x20;

## 5) Dependencies & Coupling

* **Internal crates**

  * **svc-index** (writes/reads the **sled** Index DB path used by services). *Coupling: medium (direct DB access today); replaceable: yes (shift to UDS daemon call).* &#x20;
  * **svc-storage** (indirect—consumes bundles later; no direct link at pack time). *Loose; replaceable: yes.*&#x20;
* **External crates (likely top set, by features used):**

  * `blake3` (addressing), `zstd` / `brotli` (precompression), `toml`/`serde` (manifest I/O), `sled` (Index DB). *All mainstream; moderate risk.* &#x20;
* **Runtime services:** **Filesystem** (store root), **sled DB** (index); **no network/TLS**.&#x20;

## 6) Config & Feature Flags

* **Env compatibility (used in docs/scripts):**

  * `RON_INDEX_DB` — sled DB location used by pack and services.&#x20;
  * `OUT_DIR` / `--store-root` — bundle root (e.g., `.onions`).&#x20;
* **CLI args:** `--tld`, `--input`, `--index-db`, `--store-root`.&#x20;
* **Features:** none documented.

## 7) Observability

* **Logs:** standard CLI logs (stderr).
* **No metrics or health endpoints** (library/CLI only); services provide `/healthz` `/readyz` `/metrics`.&#x20;

## 8) Concurrency Model

* **CLI pipeline** (compute → precompress → write → index update) runs **synchronously**; no server tasks/backpressure here. (Implied by one-shot CLI script flow.)&#x20;

## 9) Persistence & Data Model

* **Bundle layout:** `Manifest.toml`, `payload.bin`, optional `payload.bin.br` / `payload.bin.zst`.&#x20;
* **Addressing:** canonical `b3:<hex>.<tld>`; ETags/URLs expect the **BLAKE3** root. &#x20;
* **Index DB:** use the **same sled path** as services to avoid “phantom 404s”.&#x20;

## 10) Errors & Security

* **Common failure:** **sled lock** if `svc-index` is running (DB opened by both). Recommendation: **pack first**, or make `tldctl` talk to the daemon. &#x20;
* **Auth/TLS:** N/A (local tool). Capabilities and TLS live at Gateway/Omnigate/services.&#x20;
* **PQ-readiness:** not applicable to this local CLI; PQ planning applies to OAP/2 and service planes.&#x20;

## 11) Performance Notes

* **Hot path:** hashing + (optional) compression + disk I/O. Precompression is a **feature** (brotli/zstd artifacts) to optimize downstream delivery.&#x20;
* **Operational tip:** avoid concurrent pack + running `svc-index` on the same sled DB (lock contention).&#x20;

## 12) Tests

* **Direct tests:** not documented.
* **Used in system tests:** guidance explicitly says **“reuse the packing routine from `tldctl`”** to build fixtures in Gateway tests.&#x20;

## 13) Improvement Opportunities

* **Daemon mode / RPC to Index:** add `--use-daemon` or default to **UDS** calls to `svc-index` to eliminate sled locks.&#x20;
* **API polish:** expose a stable `pack()` lib API (already implied by reuse in tests) and document manifest schema.&#x20;
* **CLI unification:** consider merging with or aligning to `ronctl` to reduce CLI surface duplication (single “control” tool).&#x20;
* **DX/Docs:** add a `tldctl(1)` manpage; show env+args + examples from Quick-Start.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Quick-start and smoke docs updated to use `tldctl pack`; bundles now include `.br`/`.zst` alongside `Manifest.toml`/`payload.bin`. &#x20;
* **2025-09-05** — Known-issue documented: sled lock when `svc-index` is running during pack.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.0 — Behavior and args are clear from Quick-Start; manifest schema and lib API need a short spec.&#x20;
* **Test coverage:** 2.0 — Encouraged for reuse in tests but no explicit unit tests referenced.&#x20;
* **Observability:** 1.5 — CLI logs only; no metrics/health. (N/A for a lib tool.)
* **Config hygiene:** 3.0 — Consistent **`RON_INDEX_DB`** and `--store-root` usage is documented; needs daemon fallback to avoid locks. &#x20;
* **Security posture:** 2.5 — Local-only; no auth/TLS concerns; relies on service-plane security when published.&#x20;
* **Performance confidence:** 2.5 — Straightforward I/O + compression; no pathologies noted beyond DB lock contention.&#x20;
* **Coupling (lower is better):** 3.0 — Medium today due to **direct sled** coupling to `svc-index`; can be loosened via UDS daemon option.&#x20;

---
