

---

# 🏛 GOVERNANCE.md — ron-naming

---

title: Governance & Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, ops, auditors, SDK authors
crate-type: policy (naming schemas & normalization)
---------------------------------------------------

## 0. Purpose

This document defines the **rules of engagement** for `ron-naming`:

* Transparent, auditable decision-making for **naming schemas**, **normalization**, and **canonical encodings**.
* Enforcement of **naming invariants** (determinism, idempotence, bounded label/length, stable DTOs).
* Clear **authority boundaries** with adjacent crates/services (e.g., `svc-index` for runtime resolution).
* **Compatibility SLAs** for downstream consumers (services, SDKs, `tldctl` users).

It ties into:

* **Hardening Blueprint** (bounded authority; signed releases; supply-chain hygiene).
* **Interop Blueprint** (DTO stability & test vectors).
* **Perfection Gates A–O** with emphasis on **Gate I** (invariants can’t be weakened silently) and **Gate M** (appeal paths & dispute handling).

---

## 1. Invariants (MUST)

Non-negotiable **naming** rules enforced by code and CI:

* **[I-N1] Determinism:** Same input ⇒ same canonical output (platform/arch independent).
* **[I-N2] Idempotence:** `normalize(normalize(x)) == normalize(x)`.
* **[I-N3] Bounds:** Post-canonicalization: label length ≤ **63 octets**; FQDN length ≤ **253 octets**; reject otherwise with typed errors.
* **[I-N4] Encoding Canon:** Canonical CBOR/JSON encodings are **stable** and **round-trip lossless**.
* **[I-N5] No side effects:** Library and `tldctl` perform **no network/DB I/O**; purely computational.
* **[I-N6] Auditability:** All schema/normalization changes are RFC’d, reviewed, signed, and shipped with **golden test vectors** and perf baselines.
* **[I-N7] Backward compatibility:** Public DTOs and normalization semantics are **SemVer-governed**; breaking changes require a **major** and migration notes.
* **[I-N8] Anti-scope:** Runtime **resolution** is not implemented here (belongs to `svc-index`).

---

## 2. Roles & Authority

### Roles

* **Maintainers (N-Core):** Own `ron-naming` design/implementation; gate RFCs and releases.
* **Policy Stewards (P-Stewards):** Curate higher-level naming **policy** drafts (live in `ron-policy`); propose inputs to N-Core.
* **Interop Liaisons:** Ensure DTO/test-vector alignment across SDKs/services.
* **Auditors:** External reviewers verifying invariants and release provenance.

### Authority Boundaries

* **N-Core** can **define/alter normalization** and **DTOs**, but **cannot** add runtime I/O or resolution.
* **P-Stewards** can **propose** rules (e.g., confusables strategy, future variants) but cannot merge code without N-Core sign-off.
* **Services (e.g., `svc-index`)** may **reject** inputs that violate current invariants; they **cannot** extend/override normalization rules upstream.
* All impactful actions require **capability-based Git permissions** (review approvals) and **signed tags** on release.

---

## 3. Rules & SLAs

### Compatibility & Stability

* **Public API/DTO stability:**

  * Minor/patch releases: **no breaking changes**; only additive or bug-fix semantics.
  * Major release: may change normalization semantics or DTO shape **only with** migration notes + upgraded golden vectors.

* **Test Vectors SLA:**

  * A canonical vector corpus (`/testing/corpora` + `/testing/vectors/*.json`) is shipped each release.
  * Any change to vectors must include rationale, RFC link, and SemVer label (breaking/additive/fix).

* **`tldctl` Coupling:**

  * `tldctl` is **version-locked** to the library. New flags must not change defaults silently.
  * Deprecations: mark in **N+1**, remove no earlier than **N+3** minor releases.

* **Release Cadence & Notification:**

  * Patch ≤ 2 weeks for critical correctness bugs; Minor on demand; Major with **30-day** heads-up (RFC accepted, migration notes published).

### Observability & Provenance

* **Signed releases:** All tags are **cryptographically signed**; SBOM and checksums published.
* **Provenance trail:** RFC ID → PRs → CI artifacts → signed tag → CHANGELOG with vectors & perf links.

---

## 4. Governance Process

### Proposal Lifecycle (RFC)

1. **Draft** (problem statement, invariants impacted, test-vector diffs, perf impact).
2. **Public Review** (≥ 5 business days; SDKs/services may file compatibility concerns).
3. **Decision** (N-Core quorum ≥ 2 maintainers; at least 1 Interop Liaison sign-off if DTOs change).
4. **Implementation** (feature branch with vectors, tests, perf baselines).
5. **Staging** (release-candidate tag; downstream try-builds).
6. **Release** (signed tag + CHANGELOG + migration notes).

**Default posture:** If quorum not reached in **T = 10 business days**, **reject** (can resubmit).

### Emergency Fixes

* **Correction Patch (CP-N):** For correctness/security regressions only; skip full RFC but require dual-maintainer approval and postmortem within **72h**.
* **No semantic expansions** via CP-N.

### Parameterization

* **Confusables / Unicode tables / normalization knobs** are code-constants here; any change requires RFC (no runtime/config toggles).

---

## 5. Audit & Observability

* **Audit artifacts:**

  * RFC docs (in repo or governance site).
  * Signed release tags + checksums.
  * Golden vectors (before/after), Criterion baselines, hyperfine CSV.
  * `cargo-public-api` diff for public items.

* **Metrics (repo/process level, optional):**

  * `governance_rfc_total{status}` – Draft/Accepted/Rejected.
  * `golden_vector_changes_total{kind}` – additive/breaking/fix.
  * `semver_releases_total{level}` – major/minor/patch.

* **Red-team drills:**

  * Attempt to slip a breaking semantic under a “patch” label → CI gate must fail (public-API diff and vector delta checks).
  * Attempt to introduce I/O (network/fs) → anti-scope lint must fail.

---

## 6. Config & Custody

* **Runtime config:** N/A (no network/DB). `tldctl` supports flags only; no secrets.

* **Key custody (release signing):**

  * Maintainer keys in **KMS/HSM** (preferred) or hardware tokens.
  * Rotation every **90 days** or post-incident.
  * Public keys published; verify in CI and by downstream packaging.

* **Supply-chain hygiene:**

  * `cargo-deny` (advisories/licenses/sources) must be green.
  * Reproducible builds where feasible; SBOM (CycloneDX) attached to releases.

---

## 7. Appeal Path

* **Who may appeal:** SDK authors, service owners, external integrators.
* **How:**

  1. Open an **Appeal Issue** referencing the RFC/commit; attach failing cases (inputs, expected vs actual).
  2. N-Core triage within **3 business days**.
  3. If blocked in production, request **Compatibility Waiver** (temporary) with scoped mitigation.
  4. Resolution via: revert, targeted patch, or scheduled major with migration aids.
* **Transparency:** Appeals and outcomes are linked in the CHANGELOG for that release.

---

## 8. Acceptance Checklist (DoD)

* [ ] Invariants **[I-N1]…[I-N8]** enforced by tests and CI gates.
* [ ] RFC lifecycle documented for any semantic change; vectors updated.
* [ ] Public API diffs checked (`cargo-public-api`) and labeled with correct SemVer.
* [ ] Release is **signed**; SBOM + checksums published.
* [ ] Perf baselines & golden vectors attached; CI perf deltas within thresholds.
* [ ] Appeal path documented; at least one red-team drill run per quarter.

---

## 9. Appendix

* **Blueprints:** Interop (DTO stability), Hardening (bounded authority, signing), Scaling/Perf (bench baselines & gates).
* **References:** Unicode normalization (NFKC + casefold), IDN length rules, canonical CBOR spec notes for determinism.
* **History:** Maintain a table of semantic changes (version, RFC, vectors changed, migration notes).

---

### Notes specific to `tldctl`

* Treated as an **offline companion** to `ron-naming` with **no runtime authority**.
* New flags default to **non-breaking** behavior; breaking defaults allowed **only** in a major release with migration guides and dual-path compatibility (`--legacy` window).
* CLI help and `--version --vectors-hash` must reflect library version and the hash of golden vectors for easy provenance checks.

---

With this governance in place, `ron-naming` remains **stable, auditable, and predictable** for all downstream consumers while preserving the strict lib boundary (no I/O, no runtime resolution) and ensuring any semantic evolution is **measured, reviewable, and revertible**.
