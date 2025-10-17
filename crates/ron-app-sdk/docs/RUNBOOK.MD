

---

```markdown
title: RUNBOOK — ron-app-sdk
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-16
audience: operators, SRE, auditors
```

# 🛠️ RUNBOOK — ron-app-sdk

## 0) Purpose

Operational manual for `ron-app-sdk`: initialization in the host, readiness, diagnostics, failure modes, recovery, scaling, and security ops.
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `ron-app-sdk`
* **Role:** Application-side SDK (no server sockets). Enforces OAP/1 bounds, DTO hygiene, deadlines/retries, and capabilities on every call to RustyOnions nodes (Gateway/Overlay/Index/Storage/Mailbox) over TLS or Tor.
* **Criticality Tier:** 2 (supporting library) gating Tier-1 services.
* **Dependencies (runtime):** `ron-transport` (client), `ron-proto` (DTOs), `tracing`, `serde`, `blake3`. Supply-chain controls: cargo-deny green; SBOM shipped on release.
* **Ports Exposed:** none (library).
* **Data Flows:** App → SDK → OAP/1 over TLS or `.onion` → Node; responses → DTO decode. **Max frame = 1 MiB**; storage streaming **≈64 KiB** chunks; idempotency keys for mutations.
* **Version/Compat:** SemVer discipline; DTOs are additive-first; `deny_unknown_fields` stays enabled. Keep env var aliases ≥1 minor before removal.

---

## 2) Startup / Shutdown (Host Integration)

### Initialize in the host

```rust
use ron_app_sdk::{RonAppSdk, SdkConfig, Transport};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Prefer env parsing + validation for ops ergonomics
    let mut cfg = SdkConfig::from_env()?  // RON_GATEWAY, RON_TRANSPORT, SDK_DEADLINE_MS, etc.
        .validate()?                      // checks OAP bounds, timeouts, cache limits
        .with_deadlines(Duration::from_secs(5)); // optional override

    // Example explicit transport selection
    cfg.transport = Transport::Tls; // or Transport::Tor if feature enabled

    // Optional PQ posture (classical default; Hybrid behind feature/config)
    // cfg.pq_mode = PqMode::Hybrid;

    let sdk = RonAppSdk::new(cfg).await?;

    // Canonical readiness mapping for your host's /readyz
    if !sdk.ready().ready {
        eprintln!("sdk not ready; failing fast");
        std::process::exit(1);
    }
    Ok(())
}
```

* **Env knobs (common):** `RON_GATEWAY`, `RON_TRANSPORT=tls|tor`, `RON_TOR_SOCKS_ADDR`, `SDK_RETRIES`, `SDK_DEADLINE_MS`, `SDK_BACKOFF_CAP_MS`, `SDK_IDEMPOTENCY=true|false`, `SDK_CACHE_ENABLED=true|false`, `SDK_CACHE_MAX_ENTRIES`, `SDK_CACHE_TTL_MS`.
* **Feature flags:** `tor` enables `.onion` via `ron-transport`; `pq` (or `hybrid`) enables PQ-Hybrid adapters if present.
* **Verification (host side):**

  * `sdk.ready()` returns `ready=true` with checks `{config, transport, tor?}`.
  * If your current SDK version lacks `ready()`, map a zero-side-effect `sdk.ping()` call to your host `/readyz`.

### Shutdown

* SDK owns no servers. Ensure the host cancels or awaits in-flight futures before exit.

---

## 3) Health & Readiness

* **Programmatic only:** `RonAppSdk::ready() -> ReadyReport { ready: bool, checks: { "config": "ok"|"bad", "transport": "ok"|"bad", "tor": "n/a"|"ok"|"bad" } }`.
* **Expectation:** Ready during init in **2–5s** (TLS). Tor may extend readiness; SDK reports `tor` check explicitly.
* **If not ready after 10s:** Treat as a fault (likely env/transport). Log typed error; do **not** mask with unbounded retries.

---

## 4) Common Failure Modes

| Symptom / Signal                     | Likely Cause                          | Metric / Log Clue                            | Resolution                                                                 | Alert Threshold          |
| ------------------------------------ | ------------------------------------- | -------------------------------------------- | -------------------------------------------------------------------------- | ------------------------ |
| `SdkError::DeadlineExceeded`         | Deadlines too tight; network variance | `sdk_request_latency_seconds` p95 drifts up  | Raise deadline modestly; cap backoff; verify gateway health and keep-alive | p95 > target +10% for 5m |
| `CapabilityExpired`/`Denied`         | Stale/invalid macaroon / wrong scope  | `sdk_auth_failures_total`                    | Refresh capability; **do not retry**                                       | any spike                |
| `OapViolation { oversize }`          | Attempted frame > 1 MiB               | `sdk_oap_violations_total`                   | Split payload or refuse; 1 MiB hard cap                                    | >0/min sustained         |
| `SchemaViolation`                    | DTO drift or server bug               | log `error_kind="SchemaViolation"`           | Pin versions; roll back/forward; keep `deny_unknown_fields`                | any burst                |
| `TorUnavailable`                     | SOCKS down; bootstrap stalled         | `sdk_tor_events_total{reason}`               | Fix Tor path first; don’t increase retries to mask                         | >3/min                   |
| **`RateLimited`**                    | Quota policy triggered                | `sdk_rate_limited_total`; `retry_after` seen | Respect `retry_after`; exponential backoff; enable idempotency for writes  | >5/min                   |
| Slow responses under load            | No TLS reuse; excessive alloc/serde   | flamegraph hotspots; alloc/op up             | Enable keep-alive; profile & trim allocations; reduce label cardinality    | p95 +10% vs baseline     |
| Cache misses > expected (if enabled) | Cache too small/TTL too short         | `sdk_cache_hits_total`, `*_misses_total`     | Tune `max_entries`/`ttl_ms`; consider `verify_puts=true` for safety        | hit rate < desired SLO   |

---

## 5) Diagnostics

* **Logs (host):** structured JSON lines. Required keys:
  `event`, `endpoint`, `attempt`, `deadline_ms`, `latency_ms`, `error_kind`, `transport`, `corr_id`.
  **Redaction = Safe** (no secrets, macaroon bodies, or idempotency keys).
* **Metrics (host-exported):**

  * Core: `sdk_requests_total{endpoint}`, `sdk_request_latency_seconds{endpoint}`, `sdk_io_timeouts_total`, `sdk_backoff_retries_total`, `sdk_auth_failures_total`, `sdk_oap_violations_total`, `sdk_rate_limited_total`, `sdk_tor_events_total{reason}`.
  * **Cache (if enabled):** `sdk_cache_hits_total`, `sdk_cache_misses_total`, `sdk_cache_evictions_total`, `sdk_cache_items`.
  * Keep label cardinality low (endpoint groups, not full paths).
* **Tracing:** spans `sdk.call|retry|decode`; propagate `corr_id` into outbound headers.
* **Perf debug:** `cargo flamegraph -p ron-app-sdk --bench sdk_benches`; `tokio-console` in integration runs.

---

## 6) Recovery Procedures

1. **Capability failures**
   Refresh/rotate macaroons; stop retrying until a fresh token is installed. Verify redaction in logs.
2. **Transport not ready**
   Validate gateway URL scheme (`https://` or `.onion`), TLS reachability, Tor SOCKS addr. Only then extend deadline/backoff.
3. **Oversize request (>1 MiB)**
   Split or refuse; for storage, stream in ≈64 KiB chunks.
4. **Schema drift**
   Pin/align `ron-proto` & server; roll back or forward; `deny_unknown_fields` stays on.
5. **Performance regressions**
   Confirm env parity; check `sdk_backoff_retries_total`; analyze flamegraph; enable connection reuse; trim allocations.
6. **Cache inconsistencies (if enabled)**
   Set `verify_puts=true` temporarily; clear ephemeral cache on restart; consider disabling cache in amnesia-strict contexts.

---

## 7) Backup / Restore

* **Stateless library.** No persistent state.
* If a host enables a local cache, treat it as **ephemeral**: may be purged anytime; must **never** bypass capability checks.
* **Amnesia mode (IDB)**: when `amnesia=true`, no backups; caches live strictly in memory.

---

## 8) Upgrades

* **SemVer + DTO discipline:** additive-first; removals/renames require a **major** bump. Keep env var aliases ≥1 minor before removal.
* **Compat gates:** CI runs `cargo public-api` snapshot and N/N-1 proto tests.
* **Perf gates:** compare benches pre/post; alert if p95 latency or throughput drifts by >10% or alloc/op by >15%.

---

## 9) Chaos & Fault Injection (Host Harness)

* Inject faults against a loopback/mock gateway:

  * 20% transient 5xx; 2% timeouts; random jitter.
  * Verify **p95 attempts ≤ 3** and no duplicate side-effects (idempotent mutations).
* Treat any masking by unbounded retries as a failure.

---

## 10) Scaling Notes (Library)

* **Concurrency:** scale by caller task count; guard with a host semaphore if needed.
* **Connection reuse:** reuse TLS sessions; avoid reconnect-per-call.
* **Chunking:** storage I/O ≈64 KiB; stay under OAP 1 MiB/frame.
* **Idempotency:** set idempotency keys on **all mutations** so retries are safe.
* **Tor path:** expect higher/variable latency; use smaller inflight, longer deadlines.
* **Cache tuning (if enabled):** bound memory via `max_entries`; choose `ttl_ms` to target your hit-rate SLO; export cache metrics.

---

## 11) Security Ops

* **Redaction:** no secrets/macaroons/idempotency keys in logs.
* **Capabilities:** attach on every call; on `Expired/Denied`, **do not retry** until refreshed.
* **OAP/1 limits:** hard 1 MiB frame cap; refuse oversize early.
* **DTO strictness:** `deny_unknown_fields` remains enabled in all builds.
* **PQ posture:** Classical by default; enable **Hybrid** via config/feature where supported; adapters must be audited before GA.

---

## 12) References

* `API.md`, `CONFIG.md`, `CONCURRENCY.md`, `INTEROP.md`, `OBSERVABILITY.md`, `PERFORMANCE.md`, `SECURITY.md`, `IDB.md` (amnesia/cache).
* Canon: Microkernel, Six Concerns, Scaling, Hardening, Omnigate/OAP bounds.

---

## ✅ Perfection Gates Checklist (SDK)

* [ ] **Gate A** — Metrics green (`sdk_request_latency_seconds`, error counters stable)
* [ ] **Gate F** — Perf guardrails: p95 latency +≤10%, throughput −≤10%, alloc/op +≤15%
* [ ] **Gate J** — Chaos drill passes; p95 attempts ≤ 3; no dup effects
* [ ] **Gate K** — Continuous vigilance: required log fields present; redaction safe
* [ ] **Gate N** — Tor perf gate: p95(Tor) ≤ **2×** p95(TLS) at equal load, with retries capped
* [ ] **Gate O** — Security audit clean (caps enforced, OAP cap observed, DTO strict, PQ posture declared)

```

