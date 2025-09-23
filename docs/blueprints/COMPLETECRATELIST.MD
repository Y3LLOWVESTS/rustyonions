macronode  
micronode  
oap  
omnigate  
ron-accounting  
ron-app-sdk  
ron-audit  
ron-auth  
ron-bus  
ron-kernel  
ron-kms  
ron-ledger  
ron-metrics  
ron-naming  
ron-policy  
ron-proto  
ron-transport  
ryker  
svc-ads  
svc-dht
svc-edge  
svc-gateway  
svc-index  
svc-interop  
svc-mailbox  
svc-mod  
svc-overlay  
svc-passport  
svc-registry  
svc-rewarder  
svc-sandbox  
svc-storage  
svc-wallet

---

### macronode

Production node profile that runs the full service mesh (gateway, overlay, storage, dht, ledger, etc.). Think “batteries-included host” with all ops knobs exposed. **Refactor note:** keep it thin—compose services via config; never embed service logic here.

### micronode

Single-tenant, developer/edge-friendly node profile (SDK + minimal services) for apps and pilots. **Refactor note:** your on-ramp bundle—opt-in features only; default to RAM-only caches and amnesia mode.

### oap

Overlay Access Protocol (HELLO/START/DATA/END framing, capability negotiation, content addressing). **Refactor note:** treat OAP as the “HTTP of RON”—stable envelopes, strict backpressure; no app semantics here.

### omnigate

Unified entrypoint (north-south) that terminates TLS, enforces quotas/fair-queue, and brokers OAP. **Refactor note:** policy/limits live here (SLOs, RPS, bytes); keep it stateless besides counters.

### ron-accounting

Fast counters and usage snapshots (bytes, requests, time-sliced). **Refactor note:** ephemeral metering; do **not** become a ledger—export to `ron-ledger` for durable truth.

### ron-app-sdk

Developer SDK (Rust first) that provides retries, tracing, idempotency, and typed OAP calls. **Refactor note:** zero shared state; deterministic errors; keep transport selection feature-gated.

### ron-audit

Structured audit trails and compliance logs across services. **Refactor note:** append-only mindset; consistent event schema; integrate with metrics but don’t duplicate them.

### ron-auth

Capability issuance/verification (macaroons etc.), session hooks, nonce management. **Refactor note:** pure library boundaries; no network side-effects; defer storage to callers.

### ron-bus

In-process/between-tasks broadcast bus with backpressure metrics. **Refactor note:** guarantee “never hold a lock across `.await`”; expose lag/drop counters (`bus_overflow_dropped_total`).

### ron-kernel

Microkernel (supervision, config hot-reload, lifecycle, health/metrics). **Refactor note:** sacred minimal surface—no app logic, no service policy; only orchestration primitives.

### ron-kms

Key management with PQ-hybrid hooks (X25519+Kyber, Dilithium/Sphincs+ readiness). **Refactor note:** pluggable entropy (OS default; QRNG feature hook later); zeroize on drop; sealed storage behind traits.

### ron-ledger

Durable, immutable economic truth (balances, debits/credits, invariants). **Refactor note:** conservation proofs live here; no transient counters; integrate ZK commitments in phases.

### ron-metrics

Prometheus exporters and readiness/health endpoints. **Refactor note:** one golden histogram/gauge set shared across services; avoid per-crate custom taxonomies.

### ron-naming

Content/names to manifests and addresses; the DNS-like library layer. **Refactor note:** keep it schema/types only—resolution lives in `svc-index`. (note: tldctl has been merged into this crate)

### ron-policy

Governance/override rules and knobs (SLAs, data-locality, rate caps). **Refactor note:** declarative policies (no imperative effects); evaluate in services but author here.

### ron-proto

Shared wire/domain types for OAP/services (DTOs, IDs, manifests). **Refactor note:** zero logic; stability over features; MSRV and semver discipline.

### ron-transport

Transport abstraction (TCP/TLS + optional Tor/Arti backend under `arti` feature). **Refactor note:** keep Arti in `src/arti/` feature-gated; stable `TransportConfig`; no service loops here. (note svc-arti-transport has been merged into this crate)

### ryker

Actor/mailbox runtime utilities with bounded queues and supervision helpers. **Refactor note:** stay ergonomic but minimal; never leak runtime choices into public types.

### svc-ads

Controlled announcement/ad plane with quotas and policy hooks. **Refactor note:** strictly capability-gated injection; integrate with `ron-ledger` for payments and with `ron-policy` for abuse limits.

### svc-dht

First-class Kademlia/Discv5 service (FIND\_NODE/VALUE, STORE/PROVIDE, replication, churn handling). **Refactor note:** isolates routing table + discovery from overlay; expose metrics (hops, success latency, bucket occupancy) and rate limits to resist eclipse/Sybil.

### svc-edge

Static/public asset plane (CDN-like) for content addressed by OAP. **Refactor note:** no business logic; strong caching and byte-range; tie quotas back to `ron-accounting`.

### svc-gateway

HTTP ↔ OAP ingress with fair-queue DRR, quotas, admission control, and `/readyz`. **Refactor note:** enforce limits early; keep request shedding and degraded modes crisp and observable.

### svc-index

Index/registry lookup from names/addresses to providers/storage bundles. **Refactor note:** read-optimized; use `svc-dht` for provider discovery; keep writes audited via `ron-audit`.

### svc-interop

Bridges to external ecosystems (REST/GraphQL/webhooks/foreign SDKs). **Refactor note:** keep boundaries explicit and reversible; never import external auth—translate to capabilities.

### svc-mailbox

Store-and-forward messaging (SEND/RECV/ACK, visibility timeouts, DLQ). **Refactor note:** at-least-once with idempotency keys; dead-letter metrics and reprocessing policies explicit.

### svc-mod

Module/plugin runtime (WASM/native) for controlled extensions. **Refactor note:** tight sandboxing by capability; deny ambient FS/network; lifecycle managed via kernel.

### svc-overlay

Session/routing/gossip for the overlay **minus** DHT (now in `svc-dht`). **Refactor note:** slimmer and safer—focus on connection lifecycles, backpressure, and peer health.

### svc-passport

Identity and credentialing service (issuance/verification for users/nodes). **Refactor note:** bind keys to attestations; avoid PII accumulation; short TTLs + revocation paths.

### svc-registry

Authoritative registry of services/nodes and their signed descriptors. **Refactor note:** governance lives here (multi-sig updates); emit bus events for observers.

### svc-rewarder

Usage-based rewards distribution with commitments/zk proofs over time. **Refactor note:** keep math separate from disbursement IO; integrate with `ron-ledger` for final settlement.

### svc-sandbox

Tarpit/decoy and abuse sink; security pressure-relief for bad traffic. **Refactor note:** never reflect sensitive info; cheap to run, easy to scale out; clean metrics for SOC.

### svc-storage

Content-addressed storage service (chunking, replication, retrieval). **Refactor note:** strict size/decompression caps; replicate policy-driven; discovery via `svc-dht`, indexing via `svc-index`.

### svc-wallet

End-user wallet service for credits/tokens with bounded issuance and transfers. **Refactor note:** small API, hard invariants (no doubles); defer all accounting truth to `ron-ledger`.

---

### Blueprint deltas to remember for the refactor

* **Transport:** Document that `svc-arti-transport` is merged into **`ron-transport`** (feature `arti`), keeping lib/service separation clean.
* **Overlay/DHT split:** Update blueprints so **`svc-dht`** owns Kademlia/Discv5 and **`svc-overlay`** focuses on sessions/gossip; adjust runbooks and metrics accordingly.
* **Index/Naming:** Reaffirm **lib vs service** separation—naming is types/schemas; index is runtime queries.
* **Accounting/Ledger:** Keep counters vs. durable truth separate; fold ZK commitments on the ledger path only.

