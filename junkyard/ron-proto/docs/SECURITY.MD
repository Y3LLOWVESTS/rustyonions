

---

# Security Notes — `ron-proto`

title: Security Notes — ron-proto
crate: ron-proto
owner: Stevan White
last-reviewed: 2025-09-28
status: draft

---

# Security Documentation — ron-proto

`ron-proto` is a **pure library** that defines typed DTOs and envelopes for OAP/1.
It does not open sockets, read files, or perform cryptography. Its security role is to **prevent abuse via parsing** and to **standardize safe error/observability semantics** used by host services.

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-proto`. It complements the repo-wide Hardening Blueprint and Interop/OAP specs.

**Core stance**

* All public types are `serde` DTOs with **strict decoding** (`#[serde(deny_unknown_fields)]`) and **closed enums** by default.
* OAP/1 bounds are **protocol invariants** (frame cap = **1 MiB**; storage chunk ≈ **64 KiB**) referenced from Interop; hosts enforce them at ingress.
* **No secrets** handled; DTOs are **amnesia-ready** (transient, zeroizable buffers when host runs in Micronode amnesia=ON).

---

## 1) Threat Model — STRIDE (+PQ)

| Category                           | Threat (examples)                                                                        | Relevant in `ron-proto`?  | Mitigation (in this crate)                                                                                              | Host Responsibilities                                                                             |
| ---------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| **S**poofing                       | Fake identities, unauthenticated peers                                                   | **N (lib)**               | N/A                                                                                                                     | Terminate TLS 1.3 (rustls); authenticate via macaroons/caps; mutual auth where required.          |
| **T**ampering                      | Mutated payloads; field injection; ambiguous encodings                                   | **Y**                     | `deny_unknown_fields`; strongly typed DTOs; closed enums; canonical `ContentId` (`b3:<hex>`) validators; versioned DTOs | Verify content addressing end-to-end; integrity (BLAKE3) and signatures live in kms/auth/overlay. |
| **R**epudiation                    | Missing/auditable logs                                                                   | **N (lib)**               | N/A                                                                                                                     | Emit JSON logs with `corr_id`; attach proto `version`/`fingerprint`; retain audit per policy.     |
| **I**nfo Disclosure                | PII/token leakage in logs; data echo                                                     | **Y (logging surface)**   | Redaction rules (never log payloads or tokens); expose **IDs/fingerprints only**                                        | Ensure log pipelines scrub secrets; rotate / protect sinks; disable debug in prod.                |
| **D**oS                            | Parser bombs; deeply nested JSON; oversize bodies; decompression bombs                   | **Y**                     | Strict parsers; structure-aware fuzz; bounded containers; reject oversize at type layer when encoded length known       | Enforce ingress caps (body/time/conn); decompression limits; DRR/quotas; circuit breakers.        |
| **E**lev. of Privilege             | Bypass auth via permissive/ambiguous parsing                                             | **Y (schema laxity)**     | Exact schema; closed enums; versioned evolution gates                                                                   | Gate privileged ops post-auth; defense-in-depth checks after decode.                              |
| **PQ (Harvest-Now/Decrypt-Later)** | Logging PQ-hybrid metadata (e.g., key/suite params) aiding future decryption correlation | **Y (metadata exposure)** | **Never** log PQ-hybrid fields; expose **fingerprints only**; reason-string canon                                       | Keep PQ material in kms/auth; rotate; prefer hybrid KEM (host’s concern), not surfaced here.      |

**Facet callouts**

* **Search/Graph**: reject unknown fields to prevent query-shape injection; maintain deterministic ordering for index inputs.
* **Media**: rely on OAP frame cap (1 MiB) + host byte-range enforcement; reject DTOs implying out-of-bound frames.
* **Trust & Safety**: canonical reason strings enable consistent audit & rate-limit policies.

---

## 2) Security Boundaries & Assumptions

**In-scope (this crate)**

* DTO definitions, strict decoding, canonical address validators, immutable error reason canon, schema/ABI fingerprint exposure.
* Fuzz/property tests for decode paths and structural invariants.

**Out-of-scope (host/service)**

* TLS termination, authentication/authorization, rate limiting/quotas, persistent storage, key management, PQ choices, onion/overlay transport, metrics endpoints, logs emission.

**Assumptions**

* Host enforces ingress caps (size/time/conn), decompression limits, and quotas.
* Host uses `ron-proto` helpers for reason strings and logs only IDs/fingerprints (never payloads).

---

## 3) Hardening Requirements (MUST)

* **Strict Serde**: All externally consumed DTOs use `#[serde(deny_unknown_fields)]`.
* **Closed Enums**: Default-deny unknown discriminants; version new variants under semver discipline.
* **Canonical IDs**: `ContentId` parser only accepts `b3:<hex>` (lowercase, 64 nybbles).
* **Immutable Reasons**: Reason strings are **append-only**; stability tested in CI.
* **Zeroization-Ready**: Provide opt-in zeroize for transient buffers; docs instruct hosts to enable amnesia.
* **No I/O, No Globals, No Unsafe**: `#![forbid(unsafe_code)]`; no `lazy_static!/static mut`.
* **Schema/ABI Fingerprints**: Export stable fingerprint string; host surfaces in `/version` and logs.

---

## 4) Proof Checklist (what we verify + where)

| Check                              | Proof Artifact / Test                                     | Gate    |
| ---------------------------------- | --------------------------------------------------------- | ------- |
| Strict decoding on all public DTOs | `tests/schema/deny_unknown_fields.rs` + schemars snapshot | **G-1** |
| Public API stability (semver)      | `cargo public-api --deny-breaking`                        | **G-2** |
| Decode fuzz (structure-aware)      | `cargo fuzz run proto_decode` (CI smoke / nightly long)   | **G-3** |
| Cross-SDK round-trip parity        | `tests/interop_vectors_{rs,ts,py}.json`                   | **G-4** |
| Reason strings stable/unique       | `tests/reason_canon.rs`                                   | **G-5** |
| Unsafe forbidden / no globals      | `#![forbid(unsafe_code)]` + clippy lint set               | **G-6** |
| Zeroization property (amnesia)     | `tests/zeroize_props.rs`                                  | **G-7** |
| Facet DTO SLOs (Graph/Media)       | `tests/facet_matrix.rs` (p95 constraints)                 | **G-8** |
| ECON conservation (if used)        | `tests/econ_props.rs` (no double-spend/negative)          | **G-9** |

---

## 5) Logging & Redaction Rules

* **Never** log payload bodies or capability tokens.
* Log **identifiers only**: e.g., `content_id: "b3:<hex>"`, plus `proto_version` and `proto_fingerprint`.
* Use canonical **reason strings** from `ron-proto` (no ad-hoc text).
* Include `corr_id` (ULID/UUID) for traceability; omit PII.
* PQ-hybrid trap: **do not** log KEM/suite params; use fingerprints.

---

## 6) Supply Chain & Dependencies

* `#![forbid(unsafe_code)]` everywhere; minimal dependencies.
* `cargo-deny` policy: licenses OK, advisories zero, bans enforced, sources pinned.
* No crypto crates in `ron-proto`—crypto belongs to kms/auth/transport.
* MSRV pinned at workspace root; keep serde/schemars versions harmonized.

---

## 7) Migration & Versioning

* Additive fields must be **optional**; breaking shape changes bump `proto_version` and crate semver.
* Deprecations are **documented**, not silently changed; reason strings are never mutated, only appended.
* Hosts must expose both `proto_version` and `proto_fingerprint` during rollouts to debug interop.

---

## 8) CI Enforcement (copy-paste)

```yaml
name: ron-proto security gates
on: [push, pull_request]
jobs:
  sec-ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build & Tests
        run: |
          cargo build -p ron-proto
          cargo test  -p ron-proto --features schema-guard
      - name: Public API (semver)
        run: |
          cargo install cargo-public-api || true
          cargo public-api -p ron-proto --deny-breaking
      - name: Decode Fuzz (smoke)
        run: |
          cargo install cargo-fuzz || true
          cargo fuzz run proto_decode -- -runs=10000
      - name: Reason Canon
        run: cargo test -p ron-proto --test reason_canon
```

---

## 9) Facet-Specific Threat Notes

* **Search/Graph**: Unknown-field rejection blocks query-shape smuggling; ensure DTO order is deterministic (e.g., `BTreeMap`) for reproducible indexing.
* **Media**: Enforce OAP 1 MiB frame cap at ingress; reject DTOs implying larger frames; avoid decompression surprises via content-encoding limits.
* **Trust & Safety**: Use reason canon to drive rate limits / anomaly detection; don’t echo user-supplied text into logs.

---

## 10) Diagram — Secure Decode Boundary

```mermaid
flowchart LR
  C[Client] -->|TLS 1.3 + macaroon| S[Host Service]
  S -->|body caps + timeouts| S
  S -->|deserialize (ron-proto)| P[Parser]
  P -->|OK → typed DTO| S
  P -.->|ERR → reason canon| L[Logs/Metrics]
  L --> A[Alerts/Runbooks]
  style P fill:#c2410c,stroke:#7c2d12,color:#fff
```

**Narrative:** The **host** authenticates and caps inputs. `ron-proto` performs strict, closed-schema parsing. On error, it returns typed errors with **canonical reasons** for consistent logs/metrics and alerting. No payloads or secrets are logged—only IDs and fingerprints.

---

## 11) Traps & Anti-Patterns

* Logging payload bytes, capability tokens, or PQ suite details.
* Accepting unknown fields “for compatibility.”
* Ad-hoc error strings (breaks observability uniformity).
* Conflating OAP frame cap (1 MiB) with storage chunk size (~64 KiB).
* Introducing I/O, TLS, or crypto into this crate.

---

## 12) References

* Interop/OAP spec (frame cap, semantics)
* Hardening Blueprint (DoS defenses, size limits)
* SIX_CONCERNS (SEC/RES/PERF/GOV/ECON/DX mapping)
* OBSERVABILITY.md (reason canon, logging schema)
* IDB.md (invariants, acceptance gates, anti-scope)

---

**Definition of Done**

* STRIDE + PQ threats reflected; strict serde enforced; fuzz/property tests in CI; reason canon test green; zeroization property verified; host guidance documented; diagram present.

---
