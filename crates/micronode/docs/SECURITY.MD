
---

````markdown
---
title: Security Notes — micronode
crate: micronode
owner: Stevan White
last-reviewed: 2025-09-23
status: draft
---

# Security Documentation — micronode

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `micronode`.  
It complements the repo-wide [Hardening Blueprint](../../docs/HARDENING_BLUEPRINT.md) and [Interop Blueprint](../../docs/INTEROP_BLUEPRINT.md).

---

## 1) Threat Model (STRIDE)

| Category | Threats | Relevant in `micronode`? | Mitigation |
|----------|---------|---------------------------|------------|
| **S**poofing | Fake identities, unauthenticated peers | **Y** | All ingress requires macaroons (short TTL) via `ron-auth`/`svc-passport`; TLS 1.3 via `ron-transport`; Arti/Tor supported |
| **T**ampering | Mutated messages, DB corruption | **Y** | Content addressing (BLAKE3 digests), CAS invariants, checksums on storage reads |
| **R**epudiation | Missing/auditable logs | **Y** | Structured JSON logs with `corr_id`; tamper-evident chains via `ron-audit` |
| **I**nformation Disclosure | PII leakage, key exposure | **Y** | Amnesia mode ON by default (RAM-only, zeroize on drop, no disk spill); redact PII at edges; sealed storage if persistence enabled |
| **D**enial of Service | Flood, slow-loris, compression bombs | **Y** | Hard timeouts (≤5s), inflight cap (512), RPS cap (500), body cap (1 MiB), decompression ratio ≤10× |
| **E**levation of Privilege | Unauthorized ops | **Y** | Capability tokens (macaroons) with role separation; no ambient authority |

**Facet-aware threats (Developer Suite alignment):**
- **Graph/Feed:** malicious graph expansion → bound query fanout.  
- **Search:** poisoned indices → validate manifests before indexing.  
- **Media:** transcoding bombs → enforce caps & checksum validation.  

---

## 2) Security Boundaries

- **Inbound:**  
  - SDK calls via `svc-gateway`/mini-gateway (HTTP↔OAP).  
  - CLI commands (`micronode run`, config flags).  
  - OAP envelopes parsed via `oap`/`ron-proto`.  

- **Outbound:**  
  - Embedded facets: `svc-index`, `svc-storage`, `svc-mailbox`, `svc-edge`.  
  - `ron-metrics` for observability.  
  - Optional `svc-interop` when bridging externally.  

- **Trust Zone:**  
  - Runs in single-tenant, developer context.  
  - Default assumption = **RAM-only, ephemeral** (amnesia).  
  - No privileged syscalls beyond kernel-surfaced APIs.  

- **Assumptions:**  
  - Kernel invariants (bus, metrics, config, health) hold.  
  - Dependent services enforce their own `/readyz` + quotas.  
  - File system isolation when persistence is enabled; default = no disk artifacts.  

---

## 3) Key & Credential Handling

- **Types of keys:** TLS certs (dev/test self-signed by default), macaroon secrets, Ed25519 IDs, PQ-hybrid hooks (X25519+Kyber for KEM, Ed25519+Dilithium for signatures).  
- **Storage:**  
  - **Default (amnesia):** in-memory only; zeroized on drop.  
  - **Optional persistence:** sealed storage with rotation.  
- **Rotation policy:** ≤30 days; revocation converges ≤1s across services.  
- **Zeroization example:**

```rust
use zeroize::Zeroizing;

fn handle_secret(bytes: Vec<u8>) {
    let secret = Zeroizing::new(bytes);
    // use secret[..] safely; drops to zeros automatically
}
````

---

## 4) Hardening Checklist

* [x] 5s timeout on all requests.
* [x] Concurrency cap: 512 inflight.
* [x] RPS cap: 500 per instance.
* [x] Request body cap: 1 MiB.
* [x] Decompression ratio ≤10×, absolute cap enforced.
* [x] UDS sockets (if enabled): dir `0700`, socket `0600`, `SO_PEERCRED` allowlist.
* [x] No direct DB access from tools (route via service API).
* [x] Chaos test: restart under load; `/readyz` flips correctly.

---

## 5) Observability for Security

* **Metrics:**

  * `rejected_total{reason="unauth"|"oversize"|"decompress_cap"|"quota"}`
  * `auth_failures_total{service="micronode"}`
  * `tls_handshake_failures_total`

* **Logs:** Structured JSON; fields: `service`, `reason`, `corr_id`, `peer_addr`; redact PII by policy.

* **Health gates:** `/readyz` fail-closed on degraded state; `/healthz` liveness only.

---

## 6) Dependencies & Supply Chain

* **Critical crates:**

  * `tokio-rustls` (TLS 1.3)
  * `ron-kms` (PQ-ready keys: X25519+Kyber, Ed25519+Dilithium)
  * `ron-auth` + `svc-passport` (capabilities)
  * `ron-proto` (DTO hygiene: `#[serde(deny_unknown_fields)]`)
  * `sled` (when persistence enabled; ephemeral otherwise)

* **Pinned versions:** workspace root Cargo.toml (e.g., tokio-rustls 0.26.x).

* **Supply chain controls:** `cargo-deny` enforced in CI; `cargo-audit` gating advisories.

* **SBOM:** generated at release; stored in `/docs/sbom/micronode/`.

---

## 7) Formal & Destructive Validation

* **Property tests:** reject malformed OAP frames; enforce CAS invariants.
* **Fuzzing:** OAP/1 frame parser (HELLO, START, malformed caps, zstd bombs).
* **Loom tests:** concurrency ordering (esp. config hot-swap, bus readiness).
* **Chaos tests:** restart `micronode` under load; `/readyz` degrades before collapse.
* **TLA+:** spec sketches for amnesia mode (no persistent leakage) and readiness DAG.
* **Red-team suite:** refreshed annually; rotated to cover new facets (Graph, Media).

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* 
* **Disclosure policy:** See repo root `SECURITY.md`.

---

## 9) Migration & Upgrades

* **Breaking changes** to key custody, amnesia defaults, or capability format = **major version bump**.
* **Deprecations:** must include migration path; end-of-support window documented.

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  A[Client SDK] -->|TLS 1.3 + macaroon| B(micronode)
  B -->|Validated request| C[Embedded services: index/storage/mailbox/edge]
  B -->|Key ops| E[ron-kms (PQ-hybrid)]
  B -.->|Reject unauth/oversized/quota| D[Error + Metrics]
  style B fill:#1e3a8a,stroke:#1e40af,color:#fff
  style E fill:#047857,stroke:#064e3b,color:#fff
```

---

## 11) Future Roadmap

* **PQ-hybrid rollout:** default to X25519+Kyber (KEM) and Ed25519+Dilithium (signatures) by 2026.
* **ZK proofs:** evaluate integration for ledger interop and capability attestations.
* **Red-team cadence:** annual refresh of destructive validation suite.
* **Facet expansion:** extend STRIDE + mitigations to new DX facets (Graph, Search, Media).

---

```

