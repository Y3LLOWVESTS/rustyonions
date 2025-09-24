
---

````markdown
---
title: micronode — Invariant-Driven Blueprint (IDB)
version: 0.3.0
status: draft
last-updated: 2025-09-23
audience: contributors, ops, auditors
crate-type: service (bin)
concerns: [SEC, RES, PERF, DX]   # ECON/GOV appear as optional hooks (gated)
profiles: { sibling: macronode, relation: "API/SDK parity; DX-first, amnesia-first" }
msrv: 1.80.0
---

# micronode — Invariant-Driven Blueprint (IDB)

## 1. Invariants (MUST)

- [I-1] **API/SDK parity with macronode.** The client-visible surface (admin plane: `/version`, `/healthz`, `/readyz`, `/metrics`) and app SDK calls are behaviorally identical at the same major version. No divergent endpoints, types, or semantics. :contentReference[oaicite:0]{index=0}
- [I-2] **Amnesia-first by default.** RAM-only caches and logs; on shutdown, **timed key purge** and zeroization are enforced; **no disk writes** unless explicitly opted-in (`MICRO_PERSIST=1`). :contentReference[oaicite:1]{index=1}
- [I-3] **No new crates; facets live in canonical owners.** Graph/Search in `svc-index`, Feed in `svc-mailbox`, Media in `svc-storage`, Trust&Safety in gateway/overlay/policy. Micronode may embed slim/single-shard variants but **must not fork ownership**. :contentReference[oaicite:2]{index=2}
- [I-4] **OAP safety limits are absolute:** **1 MiB** frame cap, **64 KiB** chunk cap, **≤10×** decompression expansion. Violations return structured errors (HTTP 413 / OAP `FrameTooLarge`/`ChunkTooLarge`). :contentReference[oaicite:3]{index=3}
- [I-5] **Concurrency discipline:** never hold a lock across `.await`; only **bounded** queues; explicit timeouts and cooperative cancellation; graceful drain within deadlines. :contentReference[oaicite:4]{index=4}
- [I-6] **Uniform observability:** expose `/version`, `/healthz`, `/readyz`, `/metrics`; metric names are stable post-release; deprecations overlap two minors. :contentReference[oaicite:5]{index=5}
- [I-7] **Runtime matrix parity:** supports both Tokio multi-threaded default and a single-thread test flavor; behavior (timeouts, backpressure, admin plane) must be equivalent within SLO jitter. :contentReference[oaicite:6]{index=6}
- [I-8] **Facet discipline & async heavy work:** Feature-gated **facets** run in canonical owners; heavy compute/fanout goes through `svc-mailbox` (DLQ, idempotency), protected by semaphores. :contentReference[oaicite:7]{index=7}
- [I-9] **PQ-hybrid readiness (project-wide toggle).** If PQ hybrid is enabled (e.g., X25519+Kyber), handshake deadlines apply; key material must use zeroizing types and never cross task boundaries as raw bytes. :contentReference[oaicite:8]{index=8}

## 2. Design Principles (SHOULD)

- [P-1] **One SDK, two profiles.** Devs write once; migrate from micronode → macronode without client changes. :contentReference[oaicite:9]{index=9}
- [P-2] **Capabilities only.** Every request carries a proper macaroon; **no ambient authority**. :contentReference[oaicite:10]{index=10}
- [P-3] **Crash-only + backpressure.** Prefer rejection (`429/503 + Retry-After`) over unbounded buffering; keep queues small, visible, and bounded. :contentReference[oaicite:11]{index=11}
- [P-4] **Privacy by Amnesia (DX elegance).** Zero local footprints in dev loops; instant start/stop; RAM-by-default with clear, explicit opt-in to persist. :contentReference[oaicite:12]{index=12}
- [P-5] **Sandbox optionality.** Mods (ranking/transcode) are optional here and can run in-proc or sandboxed; macronode mandates sandboxing, micronode **does not** by default. :contentReference[oaicite:13]{index=13}
- [P-6] **SLO-first facets.** Graph/Feed/Search/Media adhere to Developer Suite SLOs (p95s below) even in embedded/single-shard form. :contentReference[oaicite:14]{index=14}
- [P-7] **“No duplicate owners.”** Do not re-implement owner logic in micronode; **compose** owners with slim wrappers and feature flags. :contentReference[oaicite:15]{index=15}

## 3. Implementation (HOW)

### 3.1 Feature Flags (Cargo & runtime)
- `features = ["graph", "search", "feed", "media", "mods-optional"]`
- Env:
  - `MICRO_PERSIST=0|1` (default **0**, amnesia). :contentReference[oaicite:16]{index=16}
  - `RON_MAX_BODY_BYTES=1048576`, `RON_MAX_CHUNK_BYTES=65536`, `RON_DECOMPRESS_RATIO_CAP=10`. :contentReference[oaicite:17]{index=17}
  - `RON_HTTP_ADDR`, `RON_METRICS_ADDR`, `RON_PQ_MODE=off|hybrid`. :contentReference[oaicite:18]{index=18}

### 3.2 Quickstart (copy-paste)
```bash
RUST_LOG=info micronode run
````

```ts
// JS/TS (browser or Node)
import { SDK } from "@ron/app-sdk";
const sdk = new SDK({ baseUrl: "http://127.0.0.1:8080", cap: process.env.RON_CAP });
// Object put/get
await sdk.put("/objects/hello.txt", new TextEncoder().encode("hi"));
const feed = await sdk.get("/feed/home?user=alice");
```

(“One SDK, two profiles” guarantee.)&#x20;

### 3.3 Security defaults

* Bind admin plane to **loopback** by default; non-loopback requires **mTLS** or **Bearer (macaroon)**. No CORS on admin.&#x20;
* Enforce OAP limits at ingress; reject over-caps with structured errors.&#x20;
* Amnesia ON → forbid disk I/O; zeroize caches on shutdown.&#x20;

### 3.4 Facet wiring (owner crates; no forks)

* **Graph/Search** → `svc-index` (single-shard + RAM cache).
* **Feed** → `svc-mailbox` (simple ranking mod optional; DLQ/idempotent enforced).
* **Media** → `svc-storage` (OFF by default; enable explicitly).
* **Trust\&Safety** → quotas/tarpits via `svc-gateway`/`svc-overlay`, evidence via `ron-audit`.
  (Owner discipline per Developer Suite.)&#x20;

### 3.5 Concurrency & backpressure idioms

* Bounded `mpsc` for work; **`try_send` → Busy** on overflow.
* Per-facet semaphores: `graph_expand`, `search_qps`, `feed_fanout`, `media_jobs`.
* No lock across `.await`; snapshot + swap pattern for hot read paths.&#x20;

### 3.6 Architecture (Mermaid) — wiring sketch

```mermaid
flowchart LR
  subgraph Micronode (single binary)
    G[mini-gateway] --> O[omnigate]
    O --> I[svc-index (graph/search, single shard, RAM cache)]
    O --> S[svc-storage (RAM by default)]
    O --> M[svc-mailbox (fanout/DLQ/idempotent)]
    O --> V[svc-overlay (ceilings/tarpits)]
  end
  Micronode -->|/metrics /healthz /readyz /version| Ops[(Operator/CI)]
```

**Render locally (SVG):**

```bash
npm i -g @mermaid-js/mermaid-cli
mmdc -i docs/micronode_facet.mmd -o docs/micronode_facet.svg
```

**CI (GitHub Actions):**

```yaml
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          mkdir -p docs
          for f in $(git ls-files '*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
```

(Render policy aligns with macronode doc patterns.)&#x20;

## 4. Acceptance Gates (PROOF)

### 4.1 CI Labels (switchboard)

* `profile:micronode`, `amnesia:on|off`, `facet:graph|feed|search|media|abuse|geo`, `concern:SEC|RES|PERF|DX` (ECON/GOV optional).&#x20;

### 4.2 Admin plane parity & SLOs

* **Parity diff**: HTTP surface snapshot (`/version|/healthz|/readyz|/metrics`) matches macronode v1 snapshot (no missing/extra fields).&#x20;
* **SLO gate** (30-day targets; CI perf smoke acceptable proxy):
  `/healthz p90≤5ms p99≤20ms`, `/readyz p90≤10ms p99≤50ms`, `/metrics p90≤100ms p99≤250ms`.&#x20;

### 4.3 Amnesia proofs

* With `MICRO_PERSIST=0`: **fs-spy** shows **0 writes**; heap scan confirms zeroized buffers on shutdown. With `MICRO_PERSIST=1`, only declared paths write.&#x20;

### 4.4 OAP limits (property/fuzz)

* Frame >1 MiB → 413; chunk >64 KiB → 413; decompression >10× → reject; round-trip vectors pass (bit-for-bit).&#x20;

### 4.5 Concurrency & drain

* Loom/property tests: no lock across `.await`; bounded queues only; graceful drain **p95≤3s, p99≤5s** under steady load.&#x20;

### 4.6 Facet SLO gates (Developer Suite §6)

* **Graph**: `neighbors(user)` **p95 ≤ 50 ms** (intra-AZ).
* **Feed**: ranking compute **p95 ≤ 300 ms**; fanout **p95 < 2 s** (10–10k followers).
* **Search**: query **p95 ≤ 150 ms**; ingest lag **p95 < 5 s**.
* **Media**: byte-range start **p95 < 100 ms** (if enabled).&#x20;

### 4.7 Trust\&Safety & Geo (explicit)

* **Abuse**: hard quotas + tarpits enforced; rejects emit evidence (`ron-audit`).
* **Geo**: single-region **hints honored**; **≥99.9%** writes land in primary region.&#x20;

### 4.8 PQ-hybrid gates (if `RON_PQ_MODE=hybrid`)

* Handshake deadline **≤2s**; keys held in zeroizing types; no raw key bytes cross tasks.&#x20;

### 4.9 Optional ECON/GOV hooks (if enabled)

* Ledger/ads/wallet contract tests pass (accounting flush cadence honored; quotas enforced).&#x20;

### 4.10 Reviewer Checklist (table)

| Item                              | Proof Artifact                                           | Status |
| --------------------------------- | -------------------------------------------------------- | ------ |
| Admin parity matches macronode v1 | `docs/api-history/micronode/http.json` diff vs macronode | ☐      |
| OAP caps enforced                 | fuzz/property logs + vector suite                        | ☐      |
| Amnesia ON: no writes             | fs-spy report, heap zeroize report                       | ☐      |
| No lock across `.await`           | clippy + loom/property tests                             | ☐      |
| Facet SLOs                        | perf harness CSVs                                        | ☐      |
| Abuse/Geo enforcement             | audit events + residency ratio                           | ☐      |
| PQ hybrid (if on)                 | handshake latency histograms                             | ☐      |

## 5. Anti-Scope (Forbidden)

* ❌ Creating new crates or owners (e.g., a “mini-ledger”). Use existing canon owners only.&#x20;
* ❌ Diverging admin endpoints, names, or semantics from macronode v1.&#x20;
* ❌ Bypassing OAP caps (1 MiB/64 KiB/≤10×) or weakening them.&#x20;
* ❌ Holding locks across `.await`, unbounded queues, or silent retries without deadlines.&#x20;
* ❌ Shipping with admin CORS or unauthenticated non-loopback admin access.&#x20;
* ❌ Trust\&Safety/Geo deviations (e.g., skipping tarpits/evidence, ignoring residency hints).&#x20;
* ❌ Artifact cruft in docs (e.g., hidden citation placeholders). (This file is clean by policy.)

## 6. References

* **Developer Suite Blueprint** — profiles, facets, SLOs, CI labels; “One SDK, two profiles”; amnesia ON by default.&#x20;
* **Macronode Admin/API & Readme** — canonical admin plane and SLOs; OAP caps; security defaults; Mermaid render policy.  &#x20;
* **OAP vectors & limits** — 1 MiB/64 KiB/≤10× caps and error taxonomy.&#x20;
* **Concurrency & Aliasing rules** — no locks across `.await`; bounded queues; drain deadlines.&#x20;

---

## 7. ✅ Definition of Done (DoD)

* Green CI on: unit/property/loom; admin parity diff; OAP fuzz; perf smoke; amnesia proofs; (optional) PQ hybrid; facet SLO harness.&#x20;
* Docs: architecture Mermaid rendered in `docs/`; HTTP/metrics snapshots captured; CHANGELOG updated.&#x20;
* **No artifact cruft in markdown** (explicit check).

```

