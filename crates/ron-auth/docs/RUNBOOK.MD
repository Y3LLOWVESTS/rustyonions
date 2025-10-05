
---

````markdown
---
title: RUNBOOK — ron-auth
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-04
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — ron-auth

## 0) Purpose
Operational manual for **ron-auth** (pure verification library): startup (as embedded in hosts), health semantics, diagnostics, failure modes, recovery, scaling notes, and security ops.  
Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview
- **Name:** `ron-auth`
- **Role:** Capability **verification & attenuation** library (macaroon-style) — **no I/O, no endpoints**; hosts instrument and export metrics/logs/traces. See [API.md](./API.md) and [OBSERVABILITY.md](./OBSERVABILITY.md).
- **Criticality Tier:** 1 (critical function embedded in many services; treat as blocking if verification is required at ingress).
- **Dependencies (runtime contracts):**
  - [`MacKeyProvider`](./API.md#mackeyprovider) → `(tenant, kid) -> MacHandle` (opaque keyed-BLAKE3 handle). Keys never reside in this crate.
  - Optional [`SigAdapter`](./API.md#sigadapter) (feature `pq-hybrid`) for cross-org signature **envelopes**.
  - Issuance/rotation via `svc-passport` (caps) and custody via `ron-kms`. See [SECURITY.md](./SECURITY.md#key-custody).
- **Ports Exposed:** **N/A (library)** — `/metrics`, `/readyz` live on hosts that wrap `verify()`. See [OBSERVABILITY.md](./OBSERVABILITY.md#golden-metrics).
- **Data Flows:** `token_b64` + `RequestCtx` + KID→key lookup → **Decision (allow/deny + reason)**; host records metrics/logs. See [API.md](./API.md#decision).
- **Version Constraints:** SemVer; reason strings and bounds are frozen API. See [IDB.md](./IDB.md#invariants) and [CONFIG.md](./CONFIG.md#verifierconfig).

---

## 2) Startup / Shutdown
**This crate is not a process.** It’s configured and used by a host (e.g., `svc-gateway`).

### Startup (host pattern)
```rust
use anyhow::Result;
use std::sync::Arc;
use ron_auth::{Verifier, VerifierConfig, Decision};
use ron_auth::ctx::RequestCtx;
use ron_auth::keys::MacKeyProvider;

fn init_verifier() -> Arc<Verifier> {
    let cfg = VerifierConfig::from_env_with_defaults(None)  // or load from your config system
        .expect("valid VerifierConfig");
    Arc::new(Verifier::new(cfg))
}

fn verify_once(
    token_b64: &str,
    ctx: &RequestCtx,
    keys: &dyn MacKeyProvider,
    v: &Verifier,
) -> Result<bool> {
    match v.verify(token_b64, ctx, keys, None)? {
        Decision::Allow => Ok(true),
        Decision::Deny { reason } => {
            // increment metrics: ron_auth_deny_total{reason}
            // log JSON with redacted digests; never log tokens/keys
            Ok(false)
        }
    }
}
````

See [CONFIG.md](./CONFIG.md#environment-mapping) for env mapping and [OBSERVABILITY.md](./OBSERVABILITY.md#metrics-taxonomy) for the canonical metrics.

### Shutdown

**N/A** at library level. Host shutdown semantics apply (drain requests, stop, unload).

---

## 3) Health & Readiness

* **Library has no endpoints.** Hosts own:

  * **`/healthz`** = process alive.
  * **`/readyz`** = deps connected (e.g., key provider ready, caches warmed if the host adds any).
* Target: host ready within **2–5s** after start; alert if not **ready by 10s**.

---

## 4) Common Failure Modes

| Symptom / Alert              | Likely Cause                                             | Metric / Log Clue                                     | Resolution                                                                                      | Alert Threshold    |           |                                                                       |        |
| ---------------------------- | -------------------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------ | --------- | --------------------------------------------------------------------- | ------ |
| Spike in `unknown_kid`       | Rotation window misconfigured; KID removed too early     | `ron_auth_unknown_kid_total{tenant,kid}` rising       | Provider map must accept **current + N previous**; republish rotation policy; hot-swap provider | any sustained rise |           |                                                                       |        |
| Many `parse.*` denies        | Malformed/oversized tokens; client bug or abuse          | `ron_auth_parse_error_total{kind="b64                 | cbor                                                                                            | bounds             | schema"}` | Enforce bounds (`max_token_bytes`, `max_caveats`) earlier; fix client | > N/5m |
| Latency p95 regressions      | Larger tokens, more caveats, or wrapper copies           | `ron_auth_duration_us` histogram                      | Check token size/caveats; ensure ≤2 alloc/op; remove extra copies in wrapper                    | p95 > SLO for 10m  |           |                                                                       |        |
| `caveat.custom.unknown`      | Namespace drift; host didn’t register custom caveat      | `ron_auth_deny_total{reason="caveat.custom.unknown"}` | Freeze registrar; align names; unknown custom caveats **deny** by default                       | any                |           |                                                                       |        |
| Denies with `caveat.amnesia` | Host set `Amnesia(true)` but process not in amnesia mode | reason string mentions amnesia                        | Either run host in RAM-only/no persistent logs or mint tokens without that caveat               | any                |           |                                                                       |        |
| Frequent `mac.mismatch`      | Wrong key handle or tenant mix-up                        | `ron_auth_deny_total{reason="mac.mismatch"}`          | Audit KID map by tenant; check mint vs verify binding                                           | sustained rise     |           |                                                                       |        |

> Use **stable deny reason strings** exactly (see [API.md](./API.md#deny-reasons) and [OBSERVABILITY.md](./OBSERVABILITY.md#dashboards)).

---

## 5) Diagnostics

**Metrics (host):**

```bash
curl -s http://HOST:PORT/metrics | \
  grep -E 'ron_auth_(verify|deny|parse_error|unknown_kid|duration_us|token_size_bytes)'
```

**Logs (host guidance):**

* JSON lines: `event=auth.verify`, `result=allow|deny`, `reason` (normalized), `corr_id`, and **redacted** `token_digest8`.
* **Never** log raw tokens or keys. See [SECURITY.md](./SECURITY.md#log-redaction).

**Tracing (optional):**

* Span name `lib.ron_auth.verify` with fields: `tenant`, `kid`, `caveats_count`, `token_size`; events `allow|deny`. See [OBSERVABILITY.md](./OBSERVABILITY.md#tracing).

---

## 6) Recovery Procedures

1. **Key Rotation Drift → `unknown_kid`**

   * **Action:** Update provider map to include **current + N previous**; hot-swap atomically (e.g., `ArcSwap<Provider>`); verify drop in `unknown_kid_total`.

2. **Bounds / Parse Errors**

   * **Action:** Confirm `VerifierConfig` within validated ranges (bytes ∈ [512, 16384]; caveats ∈ [1, 1024]) in [CONFIG.md](./CONFIG.md#validated-ranges). Enforce caps earlier at ingress.

3. **Config Drift (host)**

   * **Action:** Build+validate new `VerifierConfig` off-thread; hot-swap via `Arc`/`ArcSwap`. Rollback if deny storms persist.

4. **Amnesia Mismatch**

   * **Action:** Either operate host in amnesia mode (RAM-only, no persistent logs, aggressive zeroization) or remove the `Amnesia(true)` caveat from minted tokens.

5. **Deny Storm from Custom Caveats**

   * **Action:** Freeze registrar; audit namespace; unknown custom caveats **deny by default**. Update [API.md](./API.md#custom-caveats) table.

---

## 7) Backup / Restore

* **Stateless library.** No state to back up. Backups concern host services only (e.g., their sled DBs).

---

## 8) Upgrades

* Respect SemVer: behavior changes to token encoding, reason strings, or caveat semantics → **MAJOR** + new golden vectors ([IDB.md](./IDB.md#test-vectors)).
* Cross-crate surfaces: `svc-passport` (mint/rotation), `ron-kms` (keys). After upgrade, replay **canonical test vectors** before enabling traffic.

---

## 9) Chaos & Validation Drills

**Quarterly (Gate J):**

* **Hot-swap KID window drill:** Remove previous KID, observe clean `kid.unknown` denies, no panics; restore window.
* **Hostile input drill:** Replay malformed Base64URL/CBOR vectors (oversize, truncated, schema-invalid); confirm `parse.*` reasons and no OOM/panic.
* **Amnesia drill:** Toggle host amnesia; mint/verify tokens with `Amnesia(true)`; ensure policy is enforced and logs remain non-persistent.

**Optional CI example (wire to repo when harness is present):**

```yaml
name: runbook-chaos
on:
  workflow_dispatch: {}
  schedule: [{cron: "0 8 1 */3 *"}]  # quarterly
jobs:
  chaos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test -p ron-auth -- --ignored runbook_chaos_hot_swap_kid
      - run: cargo test -p ron-auth -- --ignored runbook_chaos_hostile_inputs
      - run: cargo test -p ron-auth -- --ignored runbook_chaos_amnesia_mode
```

(Implement the `--ignored` drills as integration tests or a small harness under `testing/`.)

---

## 10) Scaling Notes (when embedded)

* **Bounds are your throttle:** `max_token_bytes` (default 4096) and `max_caveats` (default 64) cap per-op cost. See [CONFIG.md](./CONFIG.md#verifierconfig).
* **Key lookups:** keep `(tenant,kid)→handle` in a fast map; rotate atomically (current + N previous).
* **Parallelism:** scale via host request-level concurrency; `verify()` is re-entrant (`Send + Sync`).
* **Observability:** use the golden metrics set and stable deny reason strings for dashboards/alerts. See [OBSERVABILITY.md](./OBSERVABILITY.md#dashboards).

---

## 11) Security Ops

* **No secret logs**; use redacted token digests for correlation. See [SECURITY.md](./SECURITY.md#log-redaction).
* **Amnesia binding:** only set `amnesia=true` in `RequestCtx` if host runs with RAM-only/no persistent logs (and zeroization policies).
* **Key custody:** keys remain in `ron-kms`; `ron-auth` sees opaque handles and zeroizes any transient material.

---

## 12) References

* [CONFIG.md](./CONFIG.md) — verifier config & bounds; env/validation rules.
* [SECURITY.md](./SECURITY.md) — threat model; boundaries; zeroization.
* [OBSERVABILITY.md](./OBSERVABILITY.md) — metrics taxonomy; alerts; tracing.
* [API.md](./API.md) — traits, DTOs, decision & reasons.
* [IDB.md](./IDB.md) — invariants & canonical vectors.

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Golden metrics wired (verify/deny/parse_error/unknown_kid/duration/token_size).
* [ ] Gate J: Chaos drill: hot-swap KID removal passes (clean denies, no panics); hostile inputs pass.
* [ ] Gate K: Continuous vigilance (alerts link to this runbook; logs redacted).
* [ ] Gate N: Edge/ARM spot check recorded (perf note only; no endpoints).
* [ ] Gate O: Security audit clean (zeroize, no secret logs).

---

## Appendix — Visuals

### A. Hot-Path Sketch

```mermaid
flowchart LR
  A[Base64URL decode] --> B[CBOR parse]
  B --> C[MAC chain (keyed BLAKE3) per caveat]
  C --> D[Evaluate caveats (exp/nbf/path/custom)]
  D --> E{Decision}
  E -->|Allow| G[Return 200/continue]
  E -->|Deny + reason| H[Return 401/403; record metrics]
```

### B. Automation Hooks (optional)

* **Rundeck / GH Actions:** expose “Rotate KID Window” and “Amnesia Drill” as `workflow_dispatch` actions that:

  1. Toggle provider map & run the chaos tests above,
  2. Post a short summary (p95 latency, deny counts) back to the PR or ops channel.

```
```
