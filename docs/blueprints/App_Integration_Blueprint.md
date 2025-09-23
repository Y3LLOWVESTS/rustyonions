

# App\_Integration\_Blueprint — 2025 Canon-Aligned

**Status:** FINAL (supersedes prior drafts)
**Last-updated:** 2025-09-21
**Audience:** SDK & App engineers, service owners, SRE, auditors
**Canon:** Exactly **33 crates**; no additions/renames. Enforced by **12 Pillars**. Overlay/DHT split; **Arti merged into `ron-transport`** (feature). **Micronode** is the default local host profile.   &#x20;

---

## 0) Scope / Non-Scope

**Scope (governed here):** How apps integrate through the **SDK** (`ron-app-sdk`), **Ingress** (`svc-gateway` + `omnigate`), and runtime services apps directly touch: `svc-mailbox`, `svc-edge`, `svc-storage`, `svc-index`, discovery via `svc-dht`, sessions via `svc-overlay`, transport via **`ron-transport`**, identity via `svc-passport`/`ron-auth`, keys via `ron-kms`. &#x20;

**Non-Scope (deferred to other blueprints):**

* Kernel orchestration (`ron-kernel`, `ron-bus`, `ryker`).&#x20;
* Economics & wallets (`ron-ledger`, `ron-accounting`, `svc-wallet`, `svc-rewarder`, `svc-ads`, `svc-interop`).&#x20;
* Any non-canon crate or service plane (e.g., `svc-oniond`, `ron-e2e`): **removed**. Transport is a **library** (`ron-transport`) with optional **Arti** backend; app-level crypto patterns live in **SDK + `ron-kms` + `ron-auth`**.&#x20;

---

## 1) Canonical Crates Touched (by Pillar)

* **P6 Ingress & Edge:** `svc-gateway`, `omnigate`, `svc-edge`.&#x20;
* **P7 App BFF & SDK:** `ron-app-sdk`, `oap`, `ron-proto`. (OAP bounds: **1 MiB frame**, **64 KiB chunk**.)&#x20;
* **P8 Node Profiles:** `micronode`, `macronode`. (Micronode → **amnesia** defaults.)&#x20;
* **P9 Content & Naming:** `ron-naming` (schemas only), `svc-index` (runtime lookups).&#x20;
* **P10 Overlay/Transport/Discovery:** `svc-overlay`, `svc-dht`, `ron-transport` (Arti feature-gated).&#x20;
* **P11 Messaging:** `svc-mailbox` (store-and-forward).&#x20;
* **P3 Identity & Keys:** `svc-passport`, `ron-auth`, `ron-kms`.&#x20;

*(Full, immutable list of 33 crates: see COMPLETECRATELIST.MD.)*&#x20;

---

## 2) Executive Summary

Apps integrate via a **stable OAP/1 surface** and a small set of services:

* **SDK** provides typed calls, idempotency, retries with full-jitter, and tracing—no ambient trust.&#x20;
* **Ingress** (`svc-gateway`/`omnigate`) enforces quotas & fair-queue (DRR); exposes HTTP↔OAP.&#x20;
* **Messaging** uses `svc-mailbox` (SEND/RECV/ACK, visibility timeouts, DLQ).&#x20;
* **Assets** via `svc-edge` with **live proxy** and **offline packs** (PMTiles/MBTiles).&#x20;
* **Content addressing** via `svc-storage` + `svc-index`; **provider discovery** via `svc-dht`; **sessions** via `svc-overlay`.&#x20;
* **Transport** is unified under **`ron-transport`** with **Arti** as a cargo feature (no separate transport service).&#x20;

**DX contract:** Everything runs **locally on Micronode** with **amnesia** (RAM-only where possible); the same configuration scales onto Macronode.&#x20;

---

## 3) Integration Principles

1. **Capability-first:** All calls present explicit capabilities (macaroon-style); **no ambient auth**. (`svc-passport`, `ron-auth`.)&#x20;
2. **Opaque payloads:** Platform enforces limits/backpressure; plaintext not required. App crypto patterns: **SDK + `ron-kms`**.&#x20;
3. **Offline-first assets:** `svc-edge` supports live proxy and offline packs.&#x20;
4. **Deterministic envelopes:** OAP/1 enforces **1 MiB / 64 KiB** bounds; DTOs are pure types with `#[serde(deny_unknown_fields)]`.&#x20;
5. **Amnesia by default on Micronode:** ephemeral, RAM-first runtime.&#x20;

---

## 4) Architecture (Control & Data Planes)

**Data Plane:**
App → `ron-app-sdk` → **Ingress** (`svc-gateway`/`omnigate`) → OAP/1 → { `svc-mailbox` | `svc-edge` | `svc-storage` | `svc-index` } → **Overlay** (`svc-overlay`) ↔ **Discovery** (`svc-dht`) via **`ron-transport`** (TCP/TLS/Arti).&#x20;

**Control Plane:**
Caps issued by `svc-passport` and verified by `ron-auth`; node/service descriptors via `svc-registry`/policy via `ron-policy`; golden metrics via `ron-metrics`. &#x20;

---

## 5) Reference Flows (with acceptance)

### A) Store-and-Forward Message (opaque)

1. App obtains cap from `svc-passport`; SDK attaches it.
2. `ron-app-sdk` → `svc-gateway` (cap checked, quotas) → **`svc-mailbox`** `SEND(topic, bytes, idem_key)`
3. Consumer `RECV(visibility_ms)` → processes → `ACK(msg_id)`; if not ACKed before visibility expiry, message re-queues; **DLQ** on poison pills (policy).
   **Acceptance:** At-least-once; idempotency respected; **p95 enqueue+dequeue < 50 ms (local)**. &#x20;

### B) Public Asset Fetch (tiles/fonts)

1. App GET `/edge/assets/...` via ingress.
2. `svc-edge`:

   * **Hit** → return from CAS with correct headers.
   * **Miss (Live)** → allow-list + rate-limits → HTTPS fetch → validate (ETag) → write CAS → return.
   * **Offline Pack** → serve from PMTiles/MBTiles; zero egress.
     **Acceptance:** Strong caching; byte-ranges; **p95 hit < 40 ms (local)**.&#x20;

### C) Content Address Lookup

1. App holds content id (`b3:<hex>`).
2. SDK queries **`svc-index`** → providers via **`svc-dht`**.
3. SDK fetches from **`svc-storage`** via ingress.
   **Acceptance:** DHT lookups **p99 ≤ 5 hops**; bounded retries with jitter.&#x20;

### D) Capability Issuance/Use

1. App authenticates to **`svc-passport`**, gets short-lived token.
2. Token presented at ingress; verified by **`ron-auth`** downstream as needed.
   **Acceptance:** No ambient trust; short TTLs; rotation without downtime.&#x20;

---

## 6) SDK Contract (Rust-first sketch)

```rust
// Lives in ron-app-sdk (sketch)
pub struct Client { /* ... */ }

impl Client {
    pub async fn send_message(&self, topic: &str, bytes: &[u8], idem_key: &str) -> Result<MsgId>;
    pub async fn recv_message(&self, topic: &str, visibility_ms: u32) -> Result<Option<Message>>;
    pub async fn ack_message(&self, msg_id: &MsgId) -> Result<()>;

    pub async fn get_asset(&self, path: &str) -> Result<Vec<u8>>;        // via svc-edge
    pub async fn get_blob(&self, cid: &str) -> Result<Vec<u8>>;          // via svc-storage
    pub async fn resolve(&self, name_or_cid: &str) -> Result<Vec<Provider>>; // via svc-index + svc-dht
}
```

**SDK invariants:** adds idempotency keys by default (configurable), emits spans across hops, enforces OAP bounds, retries with full-jitter.&#x20;

---

## 7) Configuration (Micronode / Macronode)

**Micronode** (dev-friendly defaults; illustrative):

```toml
# micronode.toml (excerpt)
[transport]                        # ron-transport
backend = "tcp"                    # "tcp" | "tls" | "arti" (enable feature on ron-transport)
idle_timeout_ms = 15000
read_timeout_ms = 5000
write_timeout_ms = 5000

[mailbox]                          # svc-mailbox
listen = "127.0.0.1:9410"
max_inflight = 128
visibility_timeout_ms = 30000
ack_deadline_ms = 15000
dlq_dir = "/tmp/ron/dlq"           # ephemeral for amnesia mode

[edge]                             # svc-edge
mode = "offline"                   # "live" | "offline"
packs = ["/data/tiles/local.pmtiles"]
allow = ["fonts.gstatic.com", "api.maptiler.com"]

[gateway]                          # svc-gateway
bind = "127.0.0.1:9080"

[observability]                    # ron-metrics
metrics = "127.0.0.1:9600"

[security]
amnesia = true                     # Micronode default
```

**Macronode/Omnigate**: same logical settings scaled out; quotas/DRR & degraded `/readyz` required at ingress. &#x20;

---

## 8) Security Model

* **Caps only** (no ambient trust): issuance via `svc-passport`, verification via `ron-auth`.&#x20;
* **PQ-ready keys** in `ron-kms`; secrets zeroized; migration path to hybrid KEM/signatures.&#x20;
* **DTO hygiene:** `ron-proto` is pure types; OAP bounds enforced.&#x20;
* **Transport** is a lib; Arti feature-gated; **no transport service loops**.&#x20;

---

## 9) Resilience & Performance (integration SLOs)

* **Mailbox** enqueue/dequeue **p95 < 50 ms** local; at-least-once + idempotent.&#x20;
* **Edge** cache hit **p95 < 40 ms** local.&#x20;
* **DHT** provider lookup **p99 ≤ 5 hops**; retries with jitter; timeouts bounded.&#x20;
* **Ingress** sheds load early (DRR); `/readyz` indicates degraded state under quota exhaustion.&#x20;

---

## 10) Observability & Audit

* **Golden metrics** via `ron-metrics` (latency, errors, saturation, queue depth); `/healthz`, `/readyz` everywhere.&#x20;
* **Audit events** for cap issuance and index updates via `ron-audit`.&#x20;

---

## 11) Interop & Federation

* `svc-interop` provides reversible bridges (REST/GraphQL/webhooks). **Never import external auth; translate to caps.**&#x20;
* Naming vs Index separation: `ron-naming` (schemas) vs `svc-index` (runtime).&#x20;

---

## 12) Backward-Compatibility & Deltas

* **Removed:** any `svc-oniond` / `ron-e2e*` references (non-canon). Align transport under **`ron-transport` (Arti feature)**; app crypto patterns live in **SDK + `ron-kms` + `ron-auth`**. &#x20;
* **Affirmed:** Overlay/DHT split (`svc-overlay` vs `svc-dht`), Micronode **amnesia**, OAP bounds. &#x20;

---

## 13) Acceptance Checklist (must-pass gates)

**DX & SDK**

* [ ] SDK emits spans for every hop; retries are full-jitter; **idempotency on** (configurable).&#x20;
* [ ] OAP frame ≤ **1 MiB**; chunk ≤ **64 KiB**; DTOs deny unknown fields.&#x20;
* [ ] Quickstart runs against **Micronode** with **amnesia**; same config works on Macronode.&#x20;

**Security**

* [ ] Every call is capability-bound (no ambient trust); token TTLs short; rotation tested.&#x20;
* [ ] Keys sourced from `ron-kms`; zeroization verified.&#x20;

**Resilience/Perf**

* [ ] Mailbox at-least-once + idempotent; **p95 < 50 ms** local.&#x20;
* [ ] Edge cache hit **p95 < 40 ms**; offline packs serve without network.&#x20;
* [ ] DHT lookup **p99 ≤ 5 hops** under steady state; bounded retries.&#x20;

**Observability & Audit**

* [ ] `/healthz` & `/readyz` exposed; degraded modes observable at ingress.&#x20;
* [ ] Audit events emitted for cap issuance and index updates.&#x20;

---

## 14) “Hello, RON” — Minimal Dev Walkthrough (Micronode)

1. Start **Micronode** with `security.amnesia=true`, `edge.mode=offline`, `mailbox.listen=127.0.0.1:9410`.&#x20;
2. App (SDK) calls:

   * `send_message(topic="ping", idem="abc123", bytes=b"...")`
   * `get_asset("/tiles/0/0/0.png")` (offline pack)
   * `resolve("b3:<cid>")` → providers via **`svc-index`**/**`svc-dht`** → `get_blob` from **`svc-storage`**
3. Validate SLOs via **`ron-metrics`**; check ingress `/readyz` under load shedding.&#x20;

---

## 15) CI Hooks (make it bite)

* PR labels trigger pillar jobs (P6/P7/P8/P9/P10/P11/P3/P5).
* Lints assert **no ambient auth**, OAP bounds, DTO hygiene, **no lock across `.await`** in touched services, and `/readyz` presence.
* Sim jobs: DHT hop-bound; mailbox idempotency/visibility; edge offline pack smoke.&#x20;

---

## 16) Glossary

* **OAP/1:** Overlay Access Protocol; neutral envelopes over transport.&#x20;
* **Amnesia mode:** Ephemeral, RAM-first runtime defaults (Micronode).&#x20;
* **CAS:** Content-addressed storage (svc-storage).&#x20;

---

### End Matter

This blueprint is **canon-pure**: it references only the **33 crates**, honors the **overlay/DHT split**, and places onion/Tor under **`ron-transport`** where it belongs. It encodes enforceable gates mapped to the **12 Pillars**, locking out drift.  &#x20;

---
