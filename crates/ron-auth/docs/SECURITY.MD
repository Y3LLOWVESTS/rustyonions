````markdown
---
title: Security Notes — ron-auth
crate: ron-auth
owner: Stevan White
last-reviewed: 2025-10-04
status: draft
---

# Security Documentation — ron-auth

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-auth`.  
It complements the repo-wide **Hardening Blueprint**, **Interop Blueprint**, and `docs/IDB.md` / `docs/CONFIG.md` for this crate.

> **Posture:** `ron-auth` is a **pure library** for capability **verification and attenuation** (macaroon-style).  
> **No network/disk I/O.** Key custody lives in `ron-kms`; issuance/rotation lives in `svc-passport`.  
> Verification is **offline**, constant-time where relevant, and **fail-closed**.

---

## 1) Threat Model (STRIDE)

| Category | Threats (attacker goals) | Relevant in `ron-auth`? | Mitigations (this crate) | Residual/Host Mitigations |
|---|---|:---:|---|---|
| **S**poofing | Forge valid tokens; spoof tenant/KID; misuse custom caveats | **Y** | Keyed **BLAKE3** MAC chain; **tenant+KID** binding; constant-time compare; **fail-closed** on unknown KID; caveat registry w/ **deny unknown** | Host must use authenticated transport (TLS) and peer auth (service boundary) |
| **T**ampering | Modify token/caveat order; truncate/extend; equivocation of encodings | **Y** | Deterministic **CBOR**, strict decode; **order-sensitive** chaining; strict size/complexity bounds; Base64URL w/o padding | Host should reject tokens outside policy windows |
| **R**epudiation | No audit trail for allow/deny | **Y** (host-level) | Returns **typed reasons** for deny (non-secret); exposes redaction guidance (`b3(token)[..8]`) | Host logs structured JSON with correlation id; do **not** log tokens/secrets |
| **I**nfo Disclosure | Leak keys/tokens via logs, panics, or debug printing | **Y** | No logging; secrets under **zeroize**; no `Debug`/`Display` for secret bytes; stable `AuthError` w/o secrets | Host must scrub logs, enable amnesia mode if required, control crash dumps |
| **D**oS | Oversized/many caveats; adversarial CBOR; pathological custom caveats | **Y** | Hard caps: **≤4096 B**, **≤64 caveats**; O(n) verification; strict parse; deny unknown custom caveats by default | Host applies rate limits/RPS caps upstream; isolates CPU budgets per request |
| **E**oP | Bypass least privilege; broaden scope; cross-tenant key misuse | **Y** | **Conjunctive** caveats; explicit scope; tenant binding; **amnesia** and **policy digest** caveats; **fail-closed** defaults | Host separates tenants; enforces amnesia mode and governance policy inputs |

**Out of scope for this crate:** transport-level TLS, socket exhaustion, storage tampering—handled by services (gateway/overlay/index/storage) and `ron-kms`.

---

## 2) Security Boundaries

- **Inbound surface (public API):**  
  - `verify(token_b64, ctx, key_provider) -> Decision`  
  - Attenuation/builders for capabilities (no broadening, only narrowing)  
  - Optional **custom caveat registrar** (write-once, then frozen)
- **Outbound dependencies:**  
  - `MacKeyProvider` trait (opaque **key handle** for keyed BLAKE3)  
  - Optional `SigAdapter` (feature `pq-hybrid`) for cross-org signature **envelopes**  
- **Trust zone:** in-process; treat **all token bytes and request context as attacker-controlled** inputs.  
- **Assumptions:**  
  - Keys are provisioned and guarded by `ron-kms` (opaque handles, rotation/KID map).  
  - Hosts enforce transport security (TLS), quotas, and authentication of peers.  
  - If **Amnesia(true)** is required, the host accurately reflects `amnesia = ON` and satisfies its operational guarantees (RAM-only, no persistent logs).  
  - Governance policy digest is provided by host when required by tokens.

---

## 3) Key & Credential Handling

- **Key types (seen indirectly):** symmetric MAC keys (keyed BLAKE3); optional public keys for PQ-hybrid envelopes (Ed25519 + Dilithium2).  
- **Ownership & storage:** keys **never** reside in `ron-auth`; only opaque `MacHandle` is used. Any transient key-derived material is **zeroized** on drop.  
- **Rotation:** tokens bind **KID**; verifiers must accept **current + N previous** and **deny unknown**. Host rotates at ≤30-day cadence (policy), revocation by removing KID from provider map.  
- **Amnesia:** when enabled at host, it implies **RAM-only** key handles and no persistence; `Amnesia(true)` caveat enforces binding.  
- **Zeroization:** all secret buffers use `zeroize` (or are kept behind key handles); `Debug/Display` forbidden for secret types.

---

## 4) Hardening Checklist (crate-specific)

- [ ] **Fail-closed** on any parse/caveat/unknown-kid error  
- [ ] **Deterministic CBOR** + Base64URL (no padding)  
- [ ] **Constant-time** MAC equality (`subtle`)  
- [ ] **Order-sensitive** MAC chain with **domain separation** and per-token nonce  
- [ ] **Strict bounds:** token ≤4096 B; ≤64 caveats; O(n) evaluation  
- [ ] **Tenant binding** + explicit scope + conjunctive caveats (no implicit OR)  
- [ ] **No I/O**; no async in hot path; **no global mutable state**  
- [ ] **Zeroize** secrets; no `Debug/Display` for secret bytes  
- [ ] **Unknown custom caveats → Deny** (default)  
- [ ] **PQ-hybrid** envelope support **feature-gated** (`pq-hybrid`), off by default  
- [ ] **Mint isolation:** `mint-internal` feature is **doc(hidden)** and disallowed in prod builds

*(Service-level items like `/readyz`, sockets, RPS caps are N/A here.)*

---

## 5) Observability for Security

`ron-auth` does not emit logs/metrics itself; it returns **typed reasons** so hosts can instrument:

**Suggested host metrics**
- `auth_verify_total{result="allow|deny", reason}`  
- `auth_parse_error_total{kind}` (`b64`, `cbor`, `bounds`)  
- `auth_unknown_kid_total{tenant,kid}`  
- `auth_caveat_fail_total{name}` (`exp`, `nbf`, `aud`, `tenant`, `amnesia`, `policy_digest`, …)  
- `auth_verify_duration_us` (histogram)

**Logging guidance**
- Structured JSON with `corr_id`, `tenant`, **no raw token**.  
- If correlation is required, log only a **redacted digest** (e.g., first 8 bytes of `b3(token_bytes)`).

---

## 6) Dependencies & Supply Chain

- **Cryptography & safety-critical**:  
  - `blake3` (keyed mode) — MAC primitive  
  - `subtle` — constant-time equality  
  - `zeroize` — secret zeroization  
- **Encoding**:  
  - `serde`, `serde_cbor` (deterministic encoding), `base64` (URL-safe)  
- **Erroring & types**:  
  - `thiserror` for non-leaky errors

**Controls**
- Pinned at workspace root; `cargo-deny` (licenses, bans, advisories, sources) in CI.  
- `#![forbid(unsafe_code)]`; `cargo geiger` zero.  
- SBOM generated at release; stored under `docs/sbom/`.

---

## 7) Formal & Destructive Validation

- **Property tests**:  
  - Re-encoding determinism; bounds enforcement; order sensitivity (reordering caveats invalidates).  
  - **Conjunction** semantics: any failing caveat denies.
- **Fuzzing**:  
  - Base64URL/CBOR token parser; malformed/hostile inputs; ensure no panics/OOM.  
- **Loom (dev-only)**:  
  - Model concurrent `verify()` calls against config snapshot replacement (host-style) to ensure no torn reads.  
- **Chaos (host-oriented)**:  
  - Under load, hot-swap config (e.g., KID removal); expect deny with `unknown_kid` and no panics.

---

## 8) Security Contacts

- **Owners:** Stevan White  
- **Escalation:** Open a private security issue in the repo; do not post secrets/tokens.

---

## 9) Migration & Upgrades

- **Breaking changes** (token encoding, MAC chain domain strings, caveat semantics): **MAJOR** SemVer bump + explicit migration notes + new golden vectors.  
- **Additive caveats**: allowed with safe defaults (**deny** unless explicitly handled).  
- **KID policy**: document rotation window; removing a KID immediately invalidates existing tokens signed with it.

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  A[Caller / Host Service] -->|token_b64 + RequestCtx| B(ron-auth::verify)
  B -->|lookup (tenant,KID) via trait| K[MacKeyProvider]
  K -- opaque handle --> B
  B -->|MAC chain (const-time)<br/>decode+evaluate caveats| D{Decision}
  D -->|Allow (attenuated scope)| OK[Proceed]
  D -->|Deny (typed reason)| NO[Return error + host metrics]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

---

## 11) Residual Risks & Notes

* **Side-channels in parsing:** While MAC equality is constant-time, CBOR decoding work is data-dependent. We mitigate via strict bounds and non-leaky errors; hosts should not expose decode timing to untrusted networks without normalizing responses.
* **Harvest-now-decrypt-later:** Symmetric MACs are not broken by Shor; **signature envelopes** (if used) are hybrid (Ed25519+Dilithium2) behind a feature gate.
* **Custom caveats:** Namespaces widen the attack surface; keep **allowlist minimal** and treat unknowns as **deny** unless a strong reason exists.
* **Amnesia binding:** Security depends on truthful host signaling (`amnesia=true` only when operational guarantees hold).
* **Governance policy digest:** Equality binding only; policy content is out-of-scope for this crate.

---

```
```
