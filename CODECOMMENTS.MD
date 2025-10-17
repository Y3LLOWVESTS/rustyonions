

# Project policy (drop into CONTRIBUTING.md)

**Rule:** Every code file must start with a brief RustyOnions header comment answering: **what** this file does, **why** it exists, **who/what it touches**, and **the invariants it must keep**. Keep it ultra-brief (ideally ≤10 lines). Cite any special limits (e.g., **OAP/1 max_frame = 1 MiB; storage chunk ≈ 64 KiB**) and amnesia implications when relevant.  

**Minimum required fields (in order):**

1. **WHAT:** one sentence summary
2. **WHY:** brief rationale (Pillar / Concern)
3. **INTERACTS:** nearest neighbors (files/modules/services)
4. **INVARIANTS:** 2–4 bullets (e.g., no lock across `.await`; single writer; DTO hygiene; OAP/size limits; amnesia)  
5. **METRICS/LOGS:** key counters/histograms this file updates (if any) 
6. **CONFIG:** knobs read (if any)
7. **SECURITY:** capability/keys/PII touchpoints (if any) 
8. **TEST HOOKS:** where it’s validated (unit/prop/fuzz/chaos)

> Tip: If you need more than ~10 lines, your file probably wants a doc module or a smaller responsibility.

---

# Paste-ready headers

## Rust (modules, handlers, services)

Use module-level doc comments (`//!`), which render in rustdoc and stay close to the code.

```rust
//! RO:WHAT — <1-sentence what this module/file does>
//! RO:WHY  — Pillar <#>, Concerns <SEC|RES|PERF|ECON|DX|GOV>; brief reason it exists
//! RO:INTERACTS — <crate::mod>, <svc>, files: <path>, DTOs: <TypeA, TypeB>
//! RO:INVARIANTS — no lock across .await; single writer per conn; OAP max_frame=1MiB; chunk≈64KiB
//! RO:METRICS — increments <request_latency_seconds>, <rejected_total{reason}>
//! RO:CONFIG — reads <cfg.path>, <env var> (amnesia honored if applicable)
//! RO:SECURITY — capability required: <cap_name>; keys via ron-kms; DTOs deny_unknown_fields
//! RO:TEST — unit:<mod tests>, fuzz:<target>, chaos:<script>, perf:<criterion bench>
```

(Those invariant bullets reflect our global rules: OAP/1 1 MiB frame cap; storage chunk ≈ 64 KiB; never hold a lock across `.await`; single-writer; DTO hygiene. Keep them only if applicable to that file.)  

### Example (handler)

```rust
//! RO:WHAT — Axum handler for GET /o/{addr}; streams CAS bytes with strong ETag.
//! RO:WHY — P6 Ingress; Concerns: PERF/RES/SEC. Public read path SLO.
//! RO:INTERACTS — svc_storage::get_blob, ron_naming::Addr, ron_metrics::http metrics
//! RO:INVARIANTS — verify full BLAKE3 "b3:<hex>" before serve; range OK; no lock across .await
//! RO:METRICS — http_requests_total, request_latency_seconds{route="/o/:addr"}, integrity_fail_total
//! RO:CONFIG — read_timeout_ms, body_cap_bytes=1MiB; amnesia has no disk spill
//! RO:SECURITY — public GET (rate-limited/quotas via svc-gateway); no ambient auth
//! RO:TEST — unit: get_object_ok(); fuzz: oap_frame_parse; chaos: range under throttle
```

(BLAKE3 addresses and OAP limits are normative. Keep the language tight.) 

## TypeScript / TSX (SDK, web, Tauri UI)

```ts
/**
 * RO:WHAT — <what this file exports / component does>
 * RO:WHY — App Integration; Concerns: DX/SEC/RES (idempotency, retries, tracing)
 * RO:INTERACTS — SDK Client, endpoints: /send /recv /o/:addr; DTOs from ron-proto
 * RO:INVARIANTS — OAP max_frame=1MiB; chunk≈64KiB; idempotency on; structured errors only
 * RO:METRICS — emits tracing spans; logs error taxonomy
 * RO:CONFIG — baseUrl, timeouts, retry/jitter settings
 * RO:SECURITY — macaroon cap required on writes; no PII persisted locally
 * RO:TEST — unit:<file>.test.ts; e2e: sdk-roundtrip.spec.ts
 */
```

(OAP/DTO hygiene & SDK behaviors flow from Interop/App Integration blueprints.) 

## Bash/Zsh (scripts, test rigs)

```bash
# RO:WHAT — <script purpose in one line>
# RO:WHY — Ops/CI; Concerns: GOV/RES (readiness gates, no magic sleeps)
# RO:INTERACTS — endpoints:/readyz /metrics; tools: curl, jq
# RO:INVARIANTS — timeout 5s; fails on non-200; no arbitrary sleep (poll with deadline)
# RO:METRICS — prints key gauges; exits non-zero on SLO breach
# RO:CONFIG — uses $BIND, $TIMEOUT_SEC
# RO:SECURITY — no secrets echoed; set -euo pipefail
# RO:TEST — invoked by CI workflow ci-invariants.yml
```

(We explicitly ban “magic sleeps” and enforce readiness polling. Keep it short.) 

## TOML / YAML (service configs, manifests)

```toml
# RO:WHAT — Config for <service>; binds, limits, quotas
# RO:WHY — Hardening defaults (timeout=5s, inflight=512, body=1MiB, decompress≤10x)
# RO:INTERACTS — consumed by <svc>; hot-reload via kernel ConfigUpdated
# RO:INVARIANTS — values must not exceed OAP/HTTP caps; amnesia=true forbids disk spill
# RO:TEST — validated by config schema & CI invariant checks
```

(These limits come straight from Hardening v2.0.) 

## YAML (GitHub Actions, K8s)

```yaml
# RO:WHAT — CI job enforcing Concern gates (lint, fuzz, perf, tsan)
# RO:WHY — GOV spine; blocks drift on Six Concerns
# RO:INTERACTS — scripts/testing/*; crates: critical set
# RO:INVARIANTS — must fail on clippy deny-wall or sanitizer failures
# RO:TEST — workflow self-runs on PR; labels route jobs
```

(Six Concerns are the constitutional gates we enforce in CI.) 

---

# Optional: pre-commit header checker (fast & friendly)

**File:** `scripts/enforce_ro_headers.sh`

> This scans staged changes and fails if a modified/added file is missing an `RO:` header line in its first 40 lines. You can tune the extensions as you like.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Recognized file extensions and required prefix regex
PATTERN='^(//!|/\*\*|/\*|#) +RO:(WHAT|WHY|INTERACTS|INVARIANTS)\b'

files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(rs|ts|tsx|js|sh|bash|zsh|toml|yaml|yml)$' || true)

[ -z "$files" ] && exit 0

fail=0
for f in $files; do
  head -n 40 "$f" | rg -n -e "$PATTERN" >/dev/null || {
    echo "Missing RustyOnions header in: $f (expected an RO:WHAT/WHY/INTERACTS/INVARIANTS line near top)" >&2
    fail=1
  }
done

exit $fail
```

**Install as a pre-commit hook:**

```bash
mkdir -p .git/hooks
cp scripts/enforce_ro_headers.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

---

# Why these exact fields?

* **WHAT/WHY** anchor intent so future edits don’t grow new responsibilities silently.
* **INTERACTS** makes coupling explicit (reviewers quickly see blast radius).
* **INVARIANTS** encode our non-negotiables (e.g., **no lock across `.await`**, **single writer per conn**, **OAP 1 MiB / 64 KiB**, **amnesia mode**), which are repeated across the **Concurrency & Aliasing**, **Hardening**, **Scaling**, and **Full Project** blueprints.    
* **METRICS/CONFIG/SECURITY/TEST** wire each file to observability, ops knobs, capability posture, and CI gates—matching our **Six Concerns** spine. 

---

# “Ultra-brief” examples you can paste right now

### Rust worker (mailbox consumer)

```rust
//! RO:WHAT — Consumes mailbox messages on topic X and writes index updates.
//! RO:WHY — P11 Messaging; Concerns: RES/PERF/SEC (at-least-once, idempotent).
//! RO:INTERACTS — svc_mailbox::recv/ack, svc_index::put_edge, ron_metrics::queue gauges
//! RO:INVARIANTS — at-least-once; idempotency key respected; no lock across .await
//! RO:METRICS — queue_age_seconds, dlq_total, request_latency_seconds
//! RO:CONFIG — mailbox.visibility_ms, ack_deadline_ms
//! RO:SECURITY — requires cap: "index.write"; no ambient authority
//! RO:TEST — prop: idempotent_upsert(); chaos: kill worker under load; soak: 1M msgs/day
```

### TS SDK client method

```ts
/**
 * RO:WHAT — sendMessage(): SEND(topic, bytes, idem_key) via gateway → mailbox.
 * RO:WHY — DX/Interop; ensures retries with full jitter and idempotency.
 * RO:INTERACTS — /send, ron-proto DTOs; traces via window.performance.
 * RO:INVARIANTS — OAP max_frame=1MiB; idem key used; structured errors only.
 * RO:SECURITY — macaroon attached on write; no PII persisted.
 * RO:TEST — sdk-roundtrip.spec.ts; fuzz: frame envelope fuzzer
 */
```

---

