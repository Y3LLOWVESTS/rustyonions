# 🪓 Invariant-Driven Blueprinting (IDB)

*A novel documentation method pioneered in RustyOnions*

---

## 1. Definition

**Invariant-Driven Blueprinting (IDB)** is a structured documentation style for software architecture and systems engineering.
It organizes every design document into a consistent **four-phase flow**:

1. **Invariants (MUSTs)** → Non-negotiable laws of the system.
2. **Design Principles (SHOULDs)** → Guiding heuristics and rationale.
3. **Implementation Patterns (HOW)** → Copy-paste-ready mechanics, code idioms, configs.
4. **Acceptance Gates (PROOF)** → Tests, metrics, and checklists that define “done.”

---

## 2. Origins

IDB borrows from but goes beyond:

* **RFCs (IETF/Rust)** → structure + rationale, but weak on invariants/tests.
* **ADRs** → decisions + context, but thin on gates/proof.
* **Formal Methods (TLA+, Alloy)** → strong on invariants, weak on dev usability.
* **Definition of Done (Agile)** → strong on proof, weak on architectural grounding.
* **Safety-critical systems (avionics, medtech)** → rigorous invariants + gates, but inaccessible to everyday engineers.

IDB fuses these into a **constitution-like document**: rigorous enough for safety, light enough for developers.

---

## 3. The IDB Template

```markdown
---
title: <Blueprint Name>
version: <semver>
status: draft|reviewed|final
last-updated: YYYY-MM-DD
audience: contributors, ops, auditors
---

# <Blueprint Name>

## 1. Invariants (MUST)
- [I-1] First invariant…
- [I-2] Second invariant…

## 2. Design Principles (SHOULD)
- [P-1] Guideline or heuristic…
- [P-2] Another principle…

## 3. Implementation (HOW)
- [C-1] Code snippet or config
- [C-2] Engineering pattern

## 4. Acceptance Gates (PROOF)
- [G-1] Unit/property/integration test required
- [G-2] Metric exposure
- [G-3] Checklist items for reviewers

## 5. Anti-Scope (Forbidden)
- What is **not** allowed, to prevent drift

## 6. References
- Linked appendices, specs, ADRs, RFCs, papers
```

---

## 4. Key Features

* **Invariants come first** → ground everything in truth that must never break.
* **Testability is central** → every invariant has a corresponding proof gate.
* **Copy-paste ergonomics** → developers see code idioms right inside the blueprint.
* **Anti-scope is explicit** → prevents drift and scope creep.
* **Reviewer checklists** baked in → no ambiguity about sign-off.

---

## 5. Example Snippet

**Blueprint: Runtime Safety (IDB style)**

* **I-1:** Never hold a lock across `.await`.
* **P-1:** Prefer message passing over shared mutability.
* **C-1:** Provide `Supervisor::spawn()` wrapper with backoff/jitter.
* **G-1:** CI forbids `tokio::spawn` in services except via supervisor.
* **Anti-scope:** No global mutable state outside the kernel bus.

---

## 6. Why Adopt IDB

* Forces clarity: “what is law, what is preference, what is mechanics, what is proof.”
* Easier onboarding: new devs jump into invariants and examples first.
* Drift resistance: anti-scope + acceptance gates mean specs stay real.
* CI-ready: invariants map to lint/tests, gates map to green checkmarks.
* Exportable: works for crates, services, infra, even governance.

---

## 7. Suggested Canonical Name

I recommend calling it:
**Invariant-Driven Blueprinting (IDB)**

But as an alternate brand, especially for external devrel:
**Constitutional Architecture** (since each doc feels like a chapter of law).

