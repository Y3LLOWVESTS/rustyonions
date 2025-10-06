
````markdown
---
title: Security Notes — ron-naming
crate: ron-naming
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
---

# Security Documentation — ron-naming

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-naming`.  
It complements the repo-wide Hardening Blueprint and Interop/IDB docs. The crate exposes **two artifacts**:

1) **Library** (default): schemas, normalization, validation, canonical encodings, address hygiene.  
2) **CLI `tldctl`** (feature `cli`): thin, **offline** wrapper for authoring/linting/packing/signing/verifying governance artifacts using the library. **No network or DB/DHT**. :contentReference[oaicite:0]{index=0} :contentReference[oaicite:1]{index=1}

---

## 1) Threat Model (STRIDE)

> Scope is tailored to **pure parsing/validation** (library) and **offline authoring** (CLI). Resolution/runtime auth lives in `svc-index` and friends.

| Category | Example threats | Relevant? | Mitigations / Notes |
|---|---|---:|---|
| **S**poofing | Names that look alike (confusables), forged governance artifacts | Yes | Unicode normalization + IDNA/UTS-46; strict confusable/mixed-script policy; BLAKE3 address format guard; detached, multi-sig envelopes via pluggable verifiers. :contentReference[oaicite:2]{index=2} :contentReference[oaicite:3]{index=3} |
| **T**ampering | Modified `TldMap` bytes, reordered entries | Yes | Canonical encodings (CBOR/JSON) with `deny_unknown_fields`; order-invariant digest for maps; canonical digest over **body**. :contentReference[oaicite:4]{index=4} :contentReference[oaicite:5]{index=5} |
| **R**epudiation | “We never published that map” | Partial | Audit/attestation lives in `ron-audit`; library provides typed structures & canonical bytes to feed audit systems. (Design choice: keep GOV in dedicated crates per Pillar model.) :contentReference[oaicite:6]{index=6} |
| **I**nfo Disclosure | PII leakage embedded in artifacts | Yes | Schema review + CI pinning of Unicode/IDNA tables; deny unknown fields; amnesia mode when embedded (no spill). :contentReference[oaicite:7]{index=7} |
| **D**oS | Oversized/cyclic inputs; path explosions in CLI | Yes | Hard caps on serialized sizes (<< OAP 1 MiB frame); explicit path validation; no network; offline, one-shot CLI. :contentReference[oaicite:8]{index=8} :contentReference[oaicite:9]{index=9} |
| **E**oP | Using `tldctl` to bypass capability checks | Yes | `tldctl` generates/verifies artifacts **only**; runtime capability enforcement happens in services (e.g., svc-index). No ambient authority here. :contentReference[oaicite:10]{index=10} |

---

## 2) Security Boundaries

- **Inbound (Library):** function inputs only; **no** config or I/O side-effects. **Inbound (CLI):** explicit files/stdin and flags/env (feature-gated). :contentReference[oaicite:11]{index=11}  
- **Outbound (Library):** canonical bytes; `b3:<hex>` addressing string checks; **no** network. **Outbound (CLI):** stdout/file outputs; **no** network attempts (tested). :contentReference[oaicite:12]{index=12}  
- **Downstream dependents:** `svc-index` for resolution, `svc-storage` for content, `svc-dht` for discovery; runtime quotas/readiness/capabilities enforced *there*, not here. :contentReference[oaicite:13]{index=13}  
- **Trust Zone:** same-tenant developer/ops usage; non-privileged by default (no keys stored).  
- **Assumptions (canon):** 33-crate canon, overlay/DHT split, `tldctl` folded into `ron-naming`. :contentReference[oaicite:14]{index=14}

---

## 3) Key & Credential Handling

- **Custody:** The library **stores no keys**. Signing/verification routes through small traits; backing implementations can delegate to `ron-kms`/HSM. :contentReference[oaicite:15]{index=15}  
- **CLI usage:** `tldctl` may **reference** key IDs/profiles and request detached signatures from an external provider; inputs/outputs are explicit paths or stdio. (No network in default profile.) :contentReference[oaicite:16]{index=16} :contentReference[oaicite:17]{index=17}  
- **PQ stance:** Optional `pq` feature selects PQ-capable verifiers (e.g., Dilithium) via the verifier abstraction; hybrid mode must be explicitly enabled and supported by the backend. :contentReference[oaicite:18]{index=18}

---

## 4) Hardening Requirements (Applicability-Tagged)

> These are **obligations**, not a completion checklist. Apply “Lib” or “CLI” as indicated.

- **Lib:** `#![forbid(unsafe_code)]`, deny `unwrap/expect` in clippy; deterministic normalization; strict DTOs with `#[serde(deny_unknown_fields)]`; address format guard (`b3:<64-hex>`). :contentReference[oaicite:19]{index=19} :contentReference[oaicite:20]{index=20} :contentReference[oaicite:21]{index=21}  
- **Lib:** Size limits on encoded artifacts documented so they remain **well below** OAP/1 `max_frame = 1 MiB`. :contentReference[oaicite:22]{index=22}  
- **CLI:** One-shot execution; **no network**; I/O restricted to stdin/stdout/explicit paths; exit codes for usage/validation/signature failures; unknown config keys rejected. :contentReference[oaicite:23]{index=23} :contentReference[oaicite:24]{index=24}  
- **CLI:** Configuration precedence (flags > env > file > defaults); dedicated `TLDCTL_*` env names. :contentReference[oaicite:25]{index=25}  
- **Both:** Unicode/IDNA table versions **pinned**; updates require vectors and CI gates. :contentReference[oaicite:26]{index=26}

---

## 5) Observability for Security (when embedded)

- **Metrics (suggested names):**  
  - `naming_manifest_parse_errors_total{reason}`  
  - `naming_manifest_validation_seconds` (histogram)  
  These are emitted by hosts that embed the library or wrap the CLI; `ron-naming` itself does not run a server.  
- **Logs:** structured JSON lines with `service`, `reason`, `corr_id`, and when relevant `content_id`.  
- **Readiness:** Resolution/serving readiness lives in `svc-index`/`svc-storage`; apply hardening defaults there (timeouts 5s, 512 inflight, 1 MiB, decompress ≤10×). :contentReference[oaicite:27]{index=27}

---

## 6) Dependencies & Supply Chain

- **Security-sensitive deps:** `serde` (strict), `blake3` (address checking), `hex` (encoding), optional arg-parsing/logging only under `cli`. (Enabling `cli` must not raise MSRV or violate workspace pins.) :contentReference[oaicite:28]{index=28}  
- **Pinning & policy:** versions pinned at workspace root; `cargo-deny` and SBOM in CI. (See Full Project Blueprint hardening defaults & CI.) :contentReference[oaicite:29]{index=29}

---

## 7) Formal, Property & Destructive Validation

- **Property tests (Lib):** idempotence (`normalize(normalize(x)) == normalize(x)`), JSON/CBOR round-trip, address hygiene near-miss rejects, TldMap digest invariants. :contentReference[oaicite:30]{index=30}  
- **Fuzz (Lib):** arbitrary Unicode into normalization — no panics, typed errors, policy respected. :contentReference[oaicite:31]{index=31}  
- **CLI conformance:** golden tests for `lint/pack/sign/verify/show`; assert **no network** attempted; canonical bytes stable across runs. :contentReference[oaicite:32]{index=32}  
- **Loom:** N/A today (sync lib); minimal models retained only for future single-writer init patterns. :contentReference[oaicite:33]{index=33}

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Disclosure policy:** Use the repo-root security process (GitHub Security Advisories).  

---

## 9) Migration & Upgrades

- **Schema evolution:** any breaking change in wire structs or canonicalization rules requires a **major version bump** and a migration note with golden vectors.  
- **Table updates:** bumping Unicode/IDNA tables must land with vector updates and CI gates; `cli` feature must not raise MSRV. :contentReference[oaicite:34]{index=34} :contentReference[oaicite:35]{index=35}

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  subgraph Dev/Ops Workstation
    A[tldctl (offline)] -->|lint/pack/sign/verify| B(ron-naming lib)
  end
  B -->|canonical bytes & b3:<hex>| C[Artifact store (svc-storage)]
  B -.->|reject invalid schema/addr| E[Error + typed reason]
  D[svc-index (runtime)] -->|resolve name→manifest| C
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

---

### Appendix — Canon Hooks (where these constraints come from)

* `tldctl` is folded into `ron-naming`; runtime lookups remain in `svc-index`. 
* Pillar 9: library/runtime split (naming vs index); Six Concerns mapping: **SEC, GOV**. 
* Hardening defaults for services (timeouts, caps, UDS perms) are global; libraries adhere via DTO hygiene and size bounds.  

```
