

---

# 🏛 GOVERNANCE.md — `oap`

---

title: Governance & Protocol Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, ops, auditors, SDK/host implementers
crate-type: policy|protocol
---------------------------

## 0. Purpose

This document defines the **rules of engagement** for the `oap` crate’s **protocol surface** and reference library:

* Transparent and auditable decision-making for protocol changes.
* Enforcement of **protocol invariants** (frame caps, envelope grammar, reject taxonomy, metrics names).
* Clear **authority boundaries** (who proposes/approves/releases), and **appeal paths** for interop disputes.
* SLA-backed commitments to **test vectors, conformance suites, and SemVer discipline**.

Ties into:

* **Omnigate / Interop Blueprints** (wire invariants; DTO semantics).
* **Hardening Blueprint** (bounded authority; signed releases; amnesia-safe).
* **Perfection Gates A–O** (esp. Gate F: perf regressions barred; Gate I: invariants bounded; Gate M: appeal paths).

---

## 1. Invariants (MUST)

Non-negotiable protocol rules (OAP/1):

* **[I-P1] Max frame size = 1 MiB (1,048,576 bytes).** Refuse oversize at **header stage**, no speculative allocation.
* **[I-P2] Envelope set is closed for OAP/1:** `HELLO | START | DATA | ACK | END | ERROR`.
* **[I-P3] Deterministic rejects:** malformed/hostile inputs map only to canonical reasons:
  `frame_oversize | header_malformed | unknown_envelope | credit_violation | missing_obj | truncated`.
* **[I-P4] Zero-copy hot path:** payloads are treated as `&[u8]`/`Bytes`—no hashing/compression/crypto in the framer.
* **[I-P5] Metrics & labels are **canonical & low-cardinality**:**
  `oap_frames_total{kind}`, `oap_rejects_total{reason}`, `oap_read_frame_seconds`, `oap_ack_window`, `oap_inflight_frames`.
* **[I-P6] PQ/HELLO feature bits are **opaque** at OAP/1:** forwarding allowed by host policy, no state-machine change in `oap`.
* **[I-P7] Strict headers in production:** undefined/duplicate headers are rejects unless explicitly enabled for interop labs.
* **[I-P8] Library scope:** no sockets, no TLS, no storage—**pure protocol**. Any drift is out-of-scope and rejected in review.

> Changing any MUST in `I-P1..I-P7` requires a **Protocol Major** (see §4, §8).

---

## 2. Roles & Authority

### Roles

* **Protocol Owner (PO):** maintains invariants; final arbiter of wire semantics.
* **Interop Steward (IS):** runs conformance, test-vector registry, and SDK alignment.
* **Release Manager (RM):** signs/ships versions; enforces SemVer & gates.
* **Security Response (SR):** handles vuln reports; coordinates hotfix vectors.
* **Auditor (external):** read-only access; verifies invariants & process adherence.
* **SDK Leads (polyglot):** Rust/Go/TS/etc. bindings stewards; ensure parity with test vectors.

### Authority Boundaries

* PO/IS **define & test** protocol; RM **releases**; SR **patches** vulns under emergency policy.
* SDK Leads **cannot** change wire rules; they consume vectors & raise PIPs (Protocol Improvement Proposals).
* Any change that affects the wire or invariants must pass **PIP quorum** and **Conformance Green** (§4).

---

## 3. Rules & SLAs

**Conformance & Vectors**

* **Vector SLA:** publish/reference **canonical vectors** and reasoned rejects with each release. Available within **24h** of tag.
* **Conformance SLA:** SDKs and hosts must pass **100%** of OAP/1 conformance for the released tag (CI badge required).
* **Metrics SLA:** names & label sets are **stable**; additions are allowed (non-breaking), renames/removals are **major**.

**Security & Disclosures**

* **Vuln triage:** acknowledge within **24h**, initial analysis **72h**, patched library + vector **<=7 days** unless embargoed.
* **Opaque PQ bits:** policy misconfig is a **host** concern; `oap` must not leak raw DATA or bitmasks in logs.

**Perf & Stability**

* **Perf regression gate:** decode p95 ↑>10% vs last green baseline blocks release (Gate F).
* **Reject ratio guard:** cannot loosen strictness defaults without major + migration guide.

---

## 4. Governance Process

**Change Types**

* **PIP-Minor:** doc clarifications, new metrics (additive), new test vectors (no wire change).
* **PIP-Compat:** additive non-wire behavior (e.g., new canonical reject subtype with stable reason value).
* **PIP-Major:** wire semantics change (envelope set, frame cap, header grammar), metrics rename/removal, reject taxonomy change.

**Lifecycle**

1. **Draft PIP** (template in `docs/pip/XXX.md`) with: summary, motivation, risk, interop impact, test plan, vectors.
2. **Review**: PO+IS+SDK Leads; public comments **≥7 days** (except security embargo).
3. **Conformance Green**: CI proves **100%** pass on updated suite across Rust/Go/TS SDKs.
4. **Quorum**: **3-of-5** (PO, IS, RM, +2 SDK Leads) for Minor/Compat; **4-of-5** for Major.
5. **Release**: RM signs tag; vectors published; CHANGELOG + migration guide attached.
6. **Post-release audit**: Auditor signs off within **14 days**.

**Emergency powers (security)**

* SR may ship a **hotfix** with temporary vector coverage; must be ratified by PO+RM within **72h** and fully PIPed within **14 days**.

**Deprecation**

* Marked in docs & conformance; provide shim period **≥1 minor**; hard removal only on next **major**.

---

## 5. Audit & Observability

* **Audit trail:** all PIPs, votes, and release signatures stored append-only; release artifacts are **detached-sig** verified.
* **Conformance dashboards:** pass/fail by crate/SDK/host; reject-reason distribution heatmap for interop labs.
* **Metrics of governance:**

  * `oap_governance_pips_total{status}`
  * `oap_vectors_published_total`
  * `oap_conformance_runs_total{result}`
* **Red-team drills:** simulate rogue proposal (e.g., raising frame cap); ensure quorum prevents merge; verify signed artifacts.

---

## 6. Config & Custody

* **Config Registry:** defaults for strict headers, reason taxonomy, and metrics names are **documented & versioned**; hosts can loosen strictness **only** in interop labs.
* **Key Custody:** release signing keys in **ron-kms/HSM**; no raw keys in repo or env; rotation **90 days** or on compromise.
* **Vector Custody:** canonical vectors & corpora stored in a signed bucket; CI fetch is integrity-checked.

---

## 7. Appeal Path

When a consumer disputes behavior (e.g., believes a valid frame was rejected):

1. **Open an Interop Dispute** (issue type `interop` with pcap/bytes + version + vector).
2. **Triage by IS**: reproduce using conformance harness; classify **protocol bug** vs **host misconfig**.
3. **If protocol bug:** create PIP; ship vector; fast-track release if security-relevant.
4. **If host misconfig:** provide remediation; record outcome.
5. **Escalation:** if unresolved in **7 days**, PO calls a quorum vote; Auditor reviews & publishes summary.

No silent rollbacks—**all** outcomes are logged and vectors updated.

---

## 8. Versioning & Compatibility

* **SemVer for `oap`:**

  * **Major**: any wire-affecting change (I-P1..I-P7), metric rename/removal, or reject taxonomy change.
  * **Minor**: additive metrics, new canonical vectors, additive non-exhaustive enums (reasons), docs.
  * **Patch**: bugfixes, perf improvements, non-wire-visible strictness clarifications.

* **Compatibility Window:**

  * Conformance + vectors retained for **last 2 minors** and **current major**.
  * Deprecations announced **≥1 minor** before removal.

* **Release Artifacts:**

  * Tag + signed checksums, vectors (`tests/vectors/*.bin|json`), conformance report, migration notes.

---

## 9. Acceptance Checklist (DoD)

* [ ] Invariants (I-P1..I-P8) codified in tests & docs.
* [ ] Roles & authority boundaries documented; contacts listed.
* [ ] PIP process active; quorum rules automated (bot labels).
* [ ] Conformance suite green across Rust/Go/TS SDKs for this tag.
* [ ] Metrics/observability contracts stable; dashboards live.
* [ ] Release signed; vectors published within 24h.
* [ ] Appeal path tested in a quarterly drill (Gate M).
* [ ] Security: keys in KMS/HSM; rotation schedule logged.

---

## 10. Appendix

**Blueprints**

* Interop / Omnigate (wire invariants & DTO semantics)
* Hardening (bounded authority; release hygiene)
* Perfection Gates F, I, M (perf, invariants, appeals)

**References**

* Conformance harness spec (`/testing/conformance/README.md`)
* Canonical vectors (`/tests/vectors/`)
* PIP template (`/docs/pip/PIP-TEMPLATE.md`)
* Release process (`/docs/releasing.md`)
* SDK parity matrix (`/docs/interop/sdk-parity.md`)

**History**

* Maintain a table of protocol disputes, decisions, and rationale (with links to vectors and PIPs).

---

### Why this works for `oap`

* It **replaces economic “no doubles”** with protocol **“no drift”**: fixed frame cap, closed envelope set, deterministic rejects, stable metrics.
* It gives you **process teeth**: PIPs, quorum, signed releases, and a 24h vector SLA.
* It’s **polyglot-friendly**: SDK leads participate, but can’t change the wire; they must pass conformance before release.
* It is **audit-ready** and **ops-ready**: every change is testable, signed, and observable.
