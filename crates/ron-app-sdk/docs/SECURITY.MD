
````markdown
---
title: Security Notes — ron-app-sdk
crate: ron-app-sdk
owner: Stevan White
last-reviewed: 2025-10-16
status: draft
---

# Security Documentation — ron-app-sdk

This document defines the **threat model**, **security boundaries**, and **hardening requirements** for `ron-app-sdk`.  
It complements the repo-wide **Hardening Blueprint** and **Interop Blueprint**, and should be read alongside `docs/IDB.md`, `docs/CONFIG.md`, and `docs/CONCURRENCY.md`.

> Profile note: `ron-app-sdk` is a **library** (client) with no server sockets. Its primary job is to **enforce canon** at the edge of the application: capability-based auth (macaroons), OAP/1 bounds, strict DTO hygiene, content addressing (BLAKE3), deterministic deadlines, and safe retries with idempotency.

---

## 1) Threat Model (STRIDE)

| Category | Threats (SDK context) | Relevant? | Mitigation (SDK obligations) |
|---|---|:--:|---|
| **S**poofing | Impersonation via missing/forged credentials; malicious gateway URL | **Y** | **Capabilities-only** requests (macaroon tokens) attached to every call; HTTPS/TLS 1.3 for `transport=tls`; `.onion` + local SOCKS for `transport=tor`; gateway URL validation (https:// or .onion); optional pinning hook if the host app uses cert/public key pinning. |
| **T**ampering | In-transit modification of frames; integrity loss on objects | **Y** | TLS; OAP/1 envelope length + checksum guardrails; **BLAKE3** content addressing—verify digest when SDK is the terminal consumer; reject schema-invalid responses (`serde(deny_unknown_fields)`). |
| **R**epudiation | Missing correlation between client calls and node logs | **Y** | Structured spans/logs with correlation IDs propagated; stable error taxonomy; request/response event fields (`endpoint`, `idem_key_redacted`, `corr_id`). |
| **I**nformation Disclosure | Leaking tokens/PII in logs or spans; cache side-channels | **Y** | **Redaction=Safe** by default (no payloads, no secrets); never log macaroon contents; ephemeral caches (TTL+bounded) that **do not** bypass capability checks; optional amnesia posture respected (no persistent SDK state). |
| **D**enial of Service | Unbounded retries; large payloads; decompression bombs; slow I/O | **Y** | **Deadlines everywhere**; full-jitter exponential backoff with caps; OAP/1 frame cap = **1 MiB**; streaming in ~64 KiB chunks; decompression ratio caps enforced downstream (SDK refuses oversize frames); connect/read/write timeouts; fail-fast on backpressure. |
| **E**levation of Privilege | Using SDK to access unauthorized operations | **Y** | Capability (macaroon) verification by the node, and SDK **never** performs ambient/implicit auth; typed errors (`CapabilityDenied/Expired`) are non-retriable; no local policy overrides in SDK. |

---

## 2) Security Boundaries

- **Inbound (to the SDK)**: App calls into `ron-app-sdk` public APIs (`mailbox_*`, `edge_get`, `storage_{get,put}`, `index_resolve`) with **explicit** capability tokens and deadlines. No external network listeners exist in the SDK.
- **Outbound (from the SDK)**: Network requests via `ron-transport` to a gateway/base URL (HTTPS or `.onion` when `transport=tor`). The SDK sends OAP/1 frames and receives DTOs defined in `ron-proto`.
- **Trust Zones**:
  - **App Process** (same trust domain as caller): SDK runs in the app’s address space.
  - **Network Boundary**: TLS or Tor to RustyOnions nodes (Micronode/Macronode).
- **Assumptions**:
  - Downstream nodes enforce their own `/readyz`, quotas, and capability checks.
  - The app provides capabilities securely (env, sealed store, or KMS); SDK never fabricates creds.
  - When “amnesia posture” is ON on the node, the **SDK does not introduce persistence** that could defeat it.

---

## 3) Key & Credential Handling

- **Key/Token Types**:
  - **Capability tokens (macaroons)**: bearer-style, short-TTL; may include caveats.
  - **TLS**: handled by `ron-transport` (client→gateway).
  - **Optional**: per-app secrets (e.g., token fetcher) provided by host app.

- **Acquisition & Storage**:
  - Tokens are supplied by the host app to SDK calls or via a small token provider; **never** stored on disk by the SDK.
  - In-memory only; wrap in `zeroize::Zeroizing` when cached briefly for request construction.
  - **Amnesia principle**: the SDK maintains no persistent store; ephemeral caches are bounded and do not contain raw secrets.

- **Rotation**:
  - Encourage rotation ≤ 30 days for long-lived apps; SDK surfaces `CapabilityExpired` distinctly so the host can refresh.
  - Provide a hook to plug a **token refresh** callback; refresh occurs **before** retries to avoid replaying expired creds.

- **Zeroization & Redaction**:
  - Credentials and sensitive headers are redacted from logs/spans (`redaction=Safe` default).
  - Drop paths zeroize temporary secret buffers after use.

---

## 4) Hardening Checklist (SDK-specific)

- [ ] **Deadlines**: Every public call requires or derives an outer deadline; `timeout(overall)` wraps the whole pipeline.  
- [ ] **Retry caps**: Full-jitter exponential backoff with `base=100ms`, `factor=2.0`, `cap=10s`, `max_attempts=5`; only for idempotent/idem-keyed ops.  
- [ ] **OAP/1 limits**: Reject frames > **1 MiB**; stream payloads in ~64 KiB chunks.  
- [ ] **Schema hygiene**: `serde(deny_unknown_fields)` on all DTOs; reject unknown/extra fields.  
- [ ] **No ambient auth**: Every request carries a capability; tests forbid ambient mode.  
- [ ] **Redaction by default**: No secrets/PII in logs/spans; redact idem keys and correlation details as needed.  
- [ ] **No persistent state**: Only bounded, TTL ephemeral caches; never bypass capability checks.  
- [ ] **Transport choice explicit**: `tls` or `tor` only; `.onion` required for `tor`.  
- [ ] **Config validation**: `gateway_addr` scheme validated; deadlines/attempt limits sane; tor SOCKS must be local.  
- [ ] **Supply chain**: `cargo-deny` clean; MSRV pinned; TLS via `tokio-rustls` only.

---

## 5) Observability for Security

- **Metrics (opt-in `metrics` feature)**:
  - `sdk_requests_total{endpoint, outcome="ok|error", code}`  
  - `auth_failures_total{reason="denied|expired"}`  
  - `tls_handshake_failures_total`  
  - `oap_violations_total{kind="frame|schema"}`  
  - `backoff_retries_total{endpoint,reason}`  
  - `io_timeouts_total{op="connect|read|write|overall"}`

- **Logs/Spans (structured JSON)**:
  - Required fields: `endpoint`, `corr_id`, `attempt`, `deadline_ms`, `node_profile`, `amnesia`, `outcome`, `error_kind`.
  - Secret handling: redact macaroon and idempotency keys; truncate payload lengths, not contents.

- **Health gates (SDK perspective)**:
  - The SDK does not expose `/healthz`; instead it **fails closed** on config/transport errors and returns typed errors to the host.
  - Host apps should wire SDK error rates/timeouts into their readiness gates.

---

## 6) Dependencies & Supply Chain

- **Sensitive crates**:
  - `tokio-rustls` (TLS 1.3 client) — pin at workspace root; no `rustls::ServerConfig` direct use.
  - `serde` + `serde_json` — strict DTO handling (`deny_unknown_fields`).
  - `blake3` — content addressing and terminal verify.
  - `tracing` — structured observability with redaction.

- **Controls**:
  - **cargo-deny**: licenses/bans/advisories/sources must be green.
  - **SBOM**: generated on release; store in `docs/sbom/`; include exact versions and features.
  - **Repro**: CI builds with **MSRV=1.80.0** and stable; public API/semver checks enabled.

---

## 7) Formal & Destructive Validation

- **Property tests**:  
  - Reject >1 MiB frames; reject DTO unknown fields; idempotency: no duplicate effects under retries.
- **Fuzzing**:  
  - OAP/1 envelope/decoder (malformed/edge sizes); ensure panic-free, typed `SchemaViolation`.
- **Loom (dev)**:  
  - Model cancel-safe retry loop under shutdown; assert no deadlocks and bounded exit times.
- **Chaos (host harness)**:  
  - Inject 20% transient 5xx + 2% timeouts; ensure p95 success ≤ 3 attempts; verify `CapabilityExpired` leads to refresh path.
- **Pen-test guidance (host)**:  
  - Attempt credential exfil via logs; verify redaction; attempt replay with expired macaroon—should be non-retriable.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Backup:** repo CODEOWNERS  
- **Disclosure policy:** See repo root `SECURITY.md` (responsible disclosure; private email; target fix SLAs).

---

## 9) Migration & Upgrades

- **Auth or token format changes** → **major** version bump; add migration notes and compat tests.  
- **DTO evolution** is **additive-first**; removing/renaming fields requires major.  
- **Config**: deprecate old keys with warnings; keep env aliases ≥ 1 minor before removal.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[App] -->|capability + deadline| B(ron-app-sdk)
  B -->|OAP/1 frame + TLS/.onion| C[Gateway/Node]
  C -->|DTO (strict schema)| B
  B -.->|reject: no cap / oversize / expired| D[Typed error + metrics]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text:** The app calls the SDK with a capability and deadline. The SDK enforces OAP/1 bounds, redaction, and strict DTO hygiene, then sends over TLS or Tor. Unauthorized/oversize/expired cases are rejected with typed errors and metrics.

---

## 11) Appendix — Secure Defaults Snapshot

* Redaction: **Safe** (on)
* Transport: **TLS** (default), **Tor** (opt-in)
* Idempotency: **enabled** for mutations
* Retries: **full-jitter** capped; non-retriable on auth/schema/OAP
* Deadlines: **required**; no unbounded I/O
* Persistence: **none** (ephemeral caches only, bounded+TTL)
* DTOs: **strict** (`deny_unknown_fields`)

```

