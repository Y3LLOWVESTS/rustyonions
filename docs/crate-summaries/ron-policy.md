
---

crate: ron-policy
path: crates/ron-policy
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Shared policy/quotas/limits library providing a simple decision engine (`PolicyEngine`) and typed decisions (`PolicyDecision`) for services to consult.

## 2) Primary Responsibilities

* Expose a small, stable API for allow/deny checks with reason strings.
* Serve as the single place to evolve quotas/limits logic used by gateway/omnigate/edge/micronode.
* Remain lightweight and dependency-minimal to avoid tight coupling.

## 3) Non-Goals

* Not a full IAM/OPA replacement; no DSL parser today.
* No persistence or distributed state.
* No direct metrics/telemetry emission (leave to callers for now).

## 4) Public API Surface

* Re-exports: none.
* Key types/functions:

  * `struct PolicyEngine` — constructor `new_default()`, method `check(principal, action) -> PolicyDecision`.
  * `struct PolicyDecision { allowed: bool, reason: &'static str }`
* HTTP/CLI: none (library only).

## 5) Dependencies & Coupling

* Internal: none (pure lib).
* External:

  * `serde`, `serde_json` (serialization) — low risk.
* Replaceable: **yes** (can be swapped for a more sophisticated engine later).
* Runtime services: none.

## 6) Config & Feature Flags

* None today; future: `PolicyConfig` (roles, limits, allowlists) via serde (file/env/remote).

## 7) Observability

* None baked in; callers should record metrics (counters for allow/deny) and traces.

## 8) Concurrency Model

* Pure functions, no async; immutable state (except future config loads).
* No locks/channels; thread-safe by construction.

## 9) Persistence & Data Model

* None; in future, load read-only config (YAML/JSON), hot-reload optional.

## 10) Errors & Security

* No error surface (current stub always allows).
* Security: reason text only; no PII; ensure callers sanitize principals/actions.

## 11) Performance Notes

* Nanosecond-class checks; no allocations on hot path (fixed reason strings).
* Scales linearly with call sites.

## 12) Tests

* Unit: default engine always allows; table-driven cases (principals/actions).
* Property tests: idempotence, determinism under concurrent calls.

## 13) Improvement Opportunities

* Add `PolicyConfig` with roles, action patterns, quotas (token-bucket or leaky-bucket).
* Add decision enums (`Allowed`, `Denied { code, reason }`), structured error type.
* Optional metrics hooks (trait) so services can record allow/deny uniformly.
* Consider OPA/Rego compatibility via compile-time translation (future).

## 14) Change Log (recent)

* 2025-09-14 — Initial allow-all stub with typed decision.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 2
* Observability: 1
* Config hygiene: 2
* Security posture: 3
* Performance confidence: 5
* Coupling (lower is better): 5

