Perfect—keeping it internal makes sense. Here’s a **paste-ready file** for:

`/docs/blueprints/DEVELOPER_SUITE_BLUEPRINT.md`

---

# DEVELOPER\_SUITE\_BLUEPRINT.md (INTERNAL-ONLY)

> **Scope:** Internal blueprint describing how RustyOnions presents a developer-facing backend suite in two profiles: **Micronode** (DX-first, “Firebase-like”) and **Macronode** (enterprise, “Facebook-class”). Keep this file private; it references external systems for intuition only.

## 0) Executive Summary

RustyOnions offers two **profiles of the same substrate** (fixed 33-crate canon). They share protocols, DTOs, and capability security, so apps can **start on Micronode** and **graduate to Macronode** without rewriting clients.

* **Micronode** — single-binary, developer-friendly, amnesia-first (RAM-only by default), optional persistence. Targets “just build the app” speed with capability-gated storage, index, mailbox, and simple BFF.
* **Macronode** — multi-tenant, policy-governed, geo-aware deployment of the **same APIs** with sharding, quotas, and sandboxed server-side functions for ranking/transcode/ETL.

**Invariant:** No new crates. Features are expressed as **facets** inside existing services (index/search/graph, feed/fanout, media, trust-and-safety, geo), enforced by **Six Concerns** gates (SEC, RES, PERF, ECON, DX, GOV).

---

## 1) Ten Developer Principles

1. **One SDK, two profiles.** `ron-app-sdk` works unchanged on Micronode and Macronode.
2. **Capabilities only.** Every request carries a macaroon; no ambient authority.
3. **Content-addressed truth.** Objects/graphs rendered from verified `b3:<hex>` digests.
4. **Crash-only + backpressure.** Limits at ingress and service boundaries; bounded queues everywhere.
5. **Amnesia as a feature.** RAM-only logs/caches by default in Micronode; opt-in disk.
6. **Uniform observability.** `/metrics`, `/healthz`, `/readyz`, `/version` on all services.
7. **No duplicate owners.** Indices live in `svc-index`; blobs in `svc-storage`; jobs in `svc-mailbox`; policies in `ron-policy`.
8. **Sandboxed server code.** Ranking/transcode/ETL run in `svc-mod` under `svc-sandbox`.
9. **Geo & governance by policy.** Residency/routing are declarative (policy + signed registry maps).
10. **CI with teeth.** Facet labels trigger targeted tests (fuzz, perf, chaos), blocking drift.

---

## 2) The Substrate (shared by both profiles)

* **Kernel & Bus:** `ron-kernel`, `ron-bus`, `ryker` (supervision, bounded channels).
* **Protocol & SDK:** OAP/1 + HTTP; DTOs in `ron-proto`; `ron-app-sdk` (JS/TS, Rust, mobile).
* **Security:** `ron-kms` (keys, zeroize, rotation), `ron-auth` + `svc-passport` (macaroons).
* **Ingress & BFF:** `svc-gateway` (limits/quotas/errors) and `omnigate` (hydrate views for clients).
* **Index & Naming:** `svc-index` (naming + *facets*: graph/search), `ron-naming` (types).
* **Storage & Media:** `svc-storage` (content-addressed blobs; byte-range), `svc-edge` (delivery).
* **Messaging & Jobs:** `svc-mailbox` (at-least-once fanout/ingest; DLQ/idempotency).
* **Mods & Sandbox:** `svc-mod` (plugins: ranking/transcode/ETL) under `svc-sandbox`.
* **Policy & Governance:** `ron-policy` (rules), `svc-registry` (signed topology/regions).
* **Economics (optional):** `ron-ledger`, `ron-accounting`, `svc-wallet`, `svc-rewarder`, `svc-ads` (centralized).
* **Observability & Audit:** `ron-metrics`, `ron-audit`.

*(Canon note: Arti/Tor lives behind a `ron-transport` feature; overlay has no DHT logic.)*

---

## 3) Profiles

### 3.1 Micronode (DX-first, “Firebase-like”)

* **Packaging:** single binary `micronode` embedding slim facets: gateway (mini), index (in-proc cache), storage (RAM by default), mailbox (single shard).
* **Amnesia:** **ON by default** (RAM caches, ephemeral logs, timed key purge, no disk spill). Enable persistence via `MICRO_PERSIST=1`.
* **Mods:** optional; can run in-proc or sandboxed.
* **Use:** prototypes, indie apps, edge nodes, privacy-sensitive deployments.

**DX quickstart**

```bash
micronode run
# JS/TS (browser or Node SSR):
import { SDK } from "@ron/app-sdk";
const sdk = new SDK({ baseUrl: "https://node.local", cap: process.env.RON_CAP });
await sdk.put("/objects/hello.txt", new TextEncoder().encode("hi"));
const feed = await sdk.get("/feed/home?user=alice");
```

### 3.2 Macronode (enterprise, “Facebook-class”)

* **Packaging:** multi-service mesh (LB→gateway→omnigate→index/storage/mailbox/overlay; plus policy/registry, mods/sandbox).
* **Amnesia:** OFF by default; persistent storage/index; RAM caches with eviction.
* **Mods:** mandatory sandbox for ranking/transcode.
* **Use:** multi-tenant apps, geo-residency, large fan-outs, complex trust-and-safety.

---

## 4) Feature Facets (no new crates)

> Higher-level capabilities implemented as **facets** inside existing owners.

| Facet                                         | Owner crates                                                                                                                | What it does                                                                         | Macronode          | Micronode                     |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------ | ----------------------------- |
| **Graph** (friends/follows, mutuals)          | `svc-index` (graph facet), `svc-storage` (edge manifests), `svc-mailbox` (mutations), `ron-policy`, `omnigate`, `ron-proto` | Adjacency lists, degree/mutuals, write-through caches; privacy via policy            | Sharded/cached     | Single shard, RAM cache       |
| **Feed** (ranking + fanout)                   | `svc-mailbox` (fanout queues), `svc-mod` (ranking plugin), `svc-sandbox`, `omnigate`, `ron-proto`                           | Write-time fanout (small creators)/pull-on-read (large), ranking mod; hydrated views | ON + DLQ + sandbox | ON (simple mod)               |
| **Search** (inverted indexes)                 | `svc-index` (search facet), `svc-mailbox` (ingest), `ron-proto`, `ron-policy`                                               | Real-time term postings, capability-aware filtering                                  | Sharded postings   | Lite postings                 |
| **Media** (transcode/thumbs, edge byte-range) | `svc-storage` (tiers/integrity), `svc-mailbox` (jobs), `svc-mod` (workers), `svc-edge`, `svc-gateway` (quotas)              | Upload→jobs→renditions; integrity checks; Range delivery                             | ON (tiers+workers) | OFF by default (opt-in)       |
| **Trust & Safety** (abuse)                    | `svc-gateway` (quotas), `svc-overlay` (ceilings), `svc-sandbox` (tarpits/decoys), `ron-policy`, `ron-audit`                 | Early drops, tarpits, evidence logs                                                  | ON                 | ON                            |
| **Geo/Residency**                             | `ron-policy` (rules), `svc-registry` (signed region maps), enforced by services                                             | Declarative routing/residency                                                        | Multi-region       | Single region (hints honored) |

**Facet isolation:** In `svc-index`, graph/search run as **feature-gated submodules** with separate executors and semaphores. Heavy work is always **asynchronous via mailbox**.

---

## 5) Ideal Macronode Stack (do you need Node.js?)

**No**—Macronode is a complete backend. Your frontend can be a SPA (or native mobile) using `ron-app-sdk`. Add Node **only** if you want SSR/ISR for SEO or edge middleware.

**Layout**

* CDN → **`svc-gateway`** → **`omnigate`** → **`svc-index`** (graph/search) + **`svc-storage`** + **`svc-mailbox`** + **`svc-overlay`**
* Security: **`ron-auth` + `svc-passport` + `ron-kms`**
* Governance: **`ron-policy` + `svc-registry`**
* Mods: **`svc-mod`** (ranking/transcode) under **`svc-sandbox`**
* Obs/audit: **`ron-metrics`**, **`ron-audit`**
* Econ (optional): **`ron-ledger`**, **`ron-accounting`**, **`svc-wallet`**, **`svc-rewarder`**, **`svc-ads`**

---

## 6) SLOs & Invariants (by facet)

* **Graph:** p95 `neighbors(user)` ≤ 50 ms intra-AZ (≤150 ms cross-AZ). Edge writes idempotent (content address).
* **Feed:** Ranking compute ≤ 300 ms p95 (Macronode), ≤ 300 ms (Micronode cached); fanout p95 < 2 s (10–10k followers).
* **Search:** Query p95 ≤ 150 ms; ingest lag p95 < 5 s from event to posting.
* **Media:** Byte-range start < 100 ms p95; integrity verified for originals & renditions.
* **Abuse:** hard quotas + tarpits; policy violations rejected with evidence events.
* **Geo:** 0 policy violations; ≥99.9% writes land in primary region.

Global: **1 MiB** frame cap; **64 KiB** streaming chunks; **safe decompression** (≤10× + absolute cap); **no locks across `.await`**; bounded queues.

---

## 7) CI & Hardening Gates (labels → tests)

**Labels:** `facet:graph|feed|search|media|abuse|geo`, `profile:micronode|macronode`, `amnesia:on|off`, `concern:SEC|RES|PERF|ECON|DX|GOV`.

**Checks (must pass):**

* **Index facets:** deny locks across `.await`; per-facet inflight caps; DTO fuzz; schema compat.
* **Mailbox:** queue-age histograms; DLQ redelivery tests; idempotency enforcement.
* **Mods:** wasm/native parity for ranking/transcode; CPU/mem ceilings; sandbox required on Macronode.
* **Amnesia matrix:** run with `MICRO_PERSIST=0/1`; verify zero on-disk artifacts when off.
* **Geo:** signature checks for registry maps; policy enforcement (hard fail on violation).
* **Ingress:** quotas/limits/structured errors; `/readyz` degrades writes first.

---

## 8) Minimal Public Doc (redacted)

For public docs, present only:

* “Micronode: single-binary, DX-first suite; Macronode: multi-service, enterprise suite.”
* Avoid external brand analogies; refer to “small” vs “large” deployments.
* Keep the same SDK/API examples; omit private SLOs and facet internals.

---

## 9) Roadmap Hooks

* **M0:** Micronode one-command + SDK “hello world”.
* **M1:** Macronode baseline (gateway/index/storage/mailbox/omnigate) + metrics.
* **M2:** Facets (graph/search/feed/media) behind feature flags; CI labels + gates wired.
* **M3:** Geo policy enforcement; sandboxed mods in prod; chaos/perf thresholds green.

---

## 10) FAQ

**Q:** Can I build everything in JavaScript/TypeScript?
**A:** Yes. Use `ron-app-sdk` from browser/Node/mobile; Macronode/Micronode provide the backend. Add Node only for SSR/ISR if desired.

**Q:** Where do custom server functions live?
**A:** As mods in `svc-mod`, isolated by `svc-sandbox`, triggered via mailbox jobs or invoked by omnigate.

**Q:** How do I move from Micronode to Macronode?
**A:** Split embedded facets into dedicated services (index, storage, mailbox), keep the same SDK calls; configure policies/regions; enable sandboxed mods.

---

**End of DEVELOPER\_SUITE\_BLUEPRINT.md**
