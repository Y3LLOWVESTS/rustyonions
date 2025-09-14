---
crate: naming
path: crates/naming
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
---

## 1) One-liner
What does this crate do in one sentence?

## 2) Primary Responsibilities
- (1–3 bullets, essential responsibilities only)

## 3) Non-Goals
- (Boundaries that prevent scope creep)

## 4) Public API Surface
- Re-exports: …
- Key types / functions / traits: …
- Events / HTTP / CLI (if any): …

## 5) Dependencies & Coupling
- Internal crates → why, stability (tight/loose), replaceable? [yes/no]
- External crates (top 5; pins/features) → why, risk (license/maintenance)
- Runtime services: Network / Storage / OS / Crypto

## 6) Config & Feature Flags
- Env vars, config structs, cargo features → effect

## 7) Observability
- Metrics, readiness/health, logs

## 8) Concurrency Model
- Tasks/channels/backpressure; locks/timeouts/retries

## 9) Persistence & Data Model
- DB/schema or key prefixes; artifacts/retention

## 10) Errors & Security
- Error taxonomy (retryable vs terminal); authn/z, TLS, secrets, PQ-readiness

## 11) Performance Notes
- Hot paths; latency/throughput targets

## 12) Tests
- Unit / Integration / E2E / fuzz / loom

## 13) Improvement Opportunities
- Known gaps / tech debt
- Overlap & redundancy signals (duplicates, API overlap)
- Streamlining (merge/extract/replace/simplify)

## 14) Change Log (recent)
- YYYY-MM-DD — short note

## 15) Readiness Score (0–5 each)
- API clarity: _
- Test coverage: _
- Observability: _
- Config hygiene: _
- Security posture: _
- Performance confidence: _
- Coupling (lower is better): _