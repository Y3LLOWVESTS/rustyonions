```markdown
# 📈 OBSERVABILITY.md — ron-auth

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible — though this crate is sync/pure)*

---

## 0) Purpose

Define **what is observable**, **how it is exposed**, and **how it’s used** for:

- Metrics (Prometheus/OTEL via host integration)
- Health/readiness semantics (N/A for this **library**; host services own them)
- Logs (JSON schema & redaction guidance)
- Tracing spans & correlation
- Alerts & SLOs (for the **verify path**)

> `ron-auth` is a **pure library** (no I/O, no HTTP endpoints). It **does not** bind `/metrics` or emit logs by itself.  
> Instead, it exposes **typed outcomes** and **lightweight hooks** so **host services** export metrics/logs/traces consistently.

---

## 1) Metrics (Prometheus-style)

### 1.1 Library “golden” metrics (to be emitted by hosts)

| Metric name | Type | Labels | Meaning |
|---|---|---|---|
| `ron_auth_verify_total` | Counter | `result="allow|deny"` | Total verifications and outcome |
| `ron_auth_deny_total` | Counter | `reason` | Denies by **normalized reason** (see 1.4) |
| `ron_auth_parse_error_total` | Counter | `kind="b64|cbor|bounds|schema"` | Decode/shape failures (pre-MAC) |
| `ron_auth_unknown_kid_total` | Counter | `tenant`,`kid` | KID not found during verification |
| `ron_auth_duration_us` | Histogram | `caveats_bucket` (optional, e.g., `0-4`,`5-16`,`17+`) | End-to-end verification time (μs) |
| `ron_auth_token_size_bytes` | Histogram | — | Size distribution of tokens (decoded) |

> **Why these:** they directly support IDB invariants and Acceptance Gates (G-3/G-4/G-5/G-6/G-12), and are minimal enough to avoid PII/secret leakage.

#### 1.2 Minimal label discipline
- **Do not** attach raw `tenant` except where operationally necessary. If used, ensure low cardinality and **no secrets**.
- Never label with raw token material. If correlation is required, use **redacted digest** (see §3.2).

#### 1.3 Registration discipline (host side)
- Metrics are registered **once** in the host’s `Metrics::new()`.  
- Keep handles (`Counter`, `Histogram`) and pass to verifier wrapper or use a global `metrics` facade (feature-gated).

#### 1.4 Canonical deny reasons (stable strings)
These strings are **part of the contract** for dashboards/alerts:

```

"parse.b64" | "parse.cbor" | "parse.bounds" | "schema.unknown_field"
"mac.mismatch" | "kid.unknown" | "tenant.mismatch"
"caveat.exp" | "caveat.nbf" | "caveat.aud" | "caveat.method"
"caveat.path" | "caveat.ip" | "caveat.bytes" | "caveat.rate"
"caveat.tenant" | "caveat.amnesia" | "caveat.policy_digest"
"caveat.custom.unknown" | "caveat.custom.failed"

````

> The crate exposes a `DenyReason` enum → **these exact strings** via `as_str()`; hosts should log/metric using these, unchanged.

---

## 2) Health & Readiness

**N/A for this library.**  
- Liveness/readiness endpoints (`/healthz`, `/readyz`) belong to **host services**.  
- Readiness should **not** depend on `ron-auth`; however, a host may include a “policy/KID cache warmed” indicator if it wraps lookups or policy digests.

---

## 3) Logs

### 3.1 Format (host guidance)
- JSON lines. Required fields:
  - `ts` (RFC3339), `level`, `service`, `event` (e.g., `auth.verify`)
  - `result` (`allow|deny`)
  - `reason` (from `DenyReason::as_str()`; omit on allow)
  - `tenant` (optional, low-cardinality)
  - `corr_id` (propagated)
  - `latency_us` (int)
  - `token_digest8` (optional; **redacted**, see below)

### 3.2 Redaction & secrets
- **Never** log raw tokens/caveats/keys.
- If correlation is needed, compute `token_digest8 = hex(b3(token_bytes))[0..8]`.  
- Config diffs/logs must redact secrets/keys and omit token bodies.

---

## 4) Tracing & Correlation

- **Feature `trace` (optional)**: `ron-auth` can annotate `verify()` with `tracing` spans via `#[instrument(skip(token_b64, key_provider))]`.
  - Span name: `lib.ron_auth.verify`
  - Span fields: `tenant`, `kid`, `caveats_count`, `token_size`
  - Events: `deny` (with normalized `reason`), `allow`
- **OpenTelemetry**: exporting is a **host** concern. If hosts enable OTEL, spans propagate naturally.

---

## 5) Alerts & SLOs (verify path)

### 5.1 SLOs (library-level, enforced by host benches/alerts)

- **Latency SLO (dev HW)**: p95 `ron_auth_duration_us` ≤ **60 + 8×caveats** μs  
  (e.g., 10 caveats → p95 ≤ 140 μs)
- **Correctness SLO**: `ron_auth_parse_error_total` very low under normal traffic; spikes indicate bad clients or regressions.
- **Reliability SLO**: `ron_auth_unknown_kid_total` near-zero in steady state; spikes indicate rotation issues.

### 5.2 Alerts (examples)

- `rate(ron_auth_unknown_kid_total[5m]) > 0` for 10m → **KID rotation misconfig**
- `histogram_quantile(0.95, sum(rate(ron_auth_duration_us_bucket[5m])) by (le)) > 2x SLO` → **perf regression**
- `increase(ron_auth_deny_total{reason=~"mac.*|parse.*"}[10m]) > N` → **attack or bug**
- `increase(ron_auth_deny_total{reason="caveat.custom.unknown"}[10m]) > 0` → **namespace drift**

> Each alert should link to a **RUNBOOK**: check rotation window, policy digest propagation, clock skew, and host amnesia mode.

---

## 6) CI / Enforcement

- **Unit tests** ensure `DenyReason::as_str()` is **stable** (golden map).
- **Criterion benches** publish `ron_auth_duration_us` locally and compare to SLOs (fail on large regressions).
- **No-log guarantee**: tests assert that the crate does not emit logs on success/failure (library purity).
- **Metrics contract test** (host example): verify counters/histograms increment as expected when wrapping `verify()`.

---

## 7) Integration patterns (copy-paste)

### 7.1 Thin wrapper that instruments verification (host side)

```rust
// host/src/auth_obs.rs
use std::time::Instant;
use prometheus::{CounterVec, Histogram, IntCounterVec};
use ron_auth::{verify_token, DenyReason, RequestCtx};
use ron_auth::config::VerifierConfig;

pub struct AuthMetrics {
    pub verify_total: CounterVec,      // labels: result
    pub deny_total: IntCounterVec,     // labels: reason
    pub parse_error_total: IntCounterVec, // labels: kind
    pub unknown_kid_total: IntCounterVec, // labels: tenant,kid (optional)
    pub duration_us: Histogram,
    pub token_size_bytes: Histogram,
}

pub fn verify_with_metrics(
    m: &AuthMetrics,
    cfg: &VerifierConfig,
    token_b64: &str,
    ctx: &RequestCtx<'_>,
    keyp: &impl ron_auth::MacKeyProvider,
) -> Result<ron_auth::Decision, ron_auth::AuthError> {
    let start = Instant::now();

    // Optional size metric (decoded)
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(token_b64) {
        m.token_size_bytes.observe(bytes.len() as f64);
    }

    match verify_token(cfg, token_b64, ctx, keyp) {
        Ok(decision) => {
            m.verify_total.with_label_values(&["allow"]).inc();
            m.duration_us.observe(start.elapsed().as_micros() as f64);
            Ok(decision)
        }
        Err(err) => {
            use ron_auth::AuthError::*;
            match &err {
                ParseBase64 => { m.parse_error_total.with_label_values(&["b64"]).inc(); }
                ParseCbor   => { m.parse_error_total.with_label_values(&["cbor"]).inc(); }
                Bounds      => { m.parse_error_total.with_label_values(&["bounds"]).inc(); }
                SchemaUnknownField => { m.parse_error_total.with_label_values(&["schema"]).inc(); }
                UnknownKid { tenant, kid } => {
                    m.unknown_kid_total.with_label_values(&[tenant, kid]).inc();
                }
                _ => { /* fall through to deny reason below */ }
            }
            m.verify_total.with_label_values(&["deny"]).inc();
            if let Some(reason) = err.deny_reason() {
                m.deny_total
                    .with_label_values(&[reason.as_str()])
                    .inc();
            }
            m.duration_us.observe(start.elapsed().as_micros() as f64);
            Err(err)
        }
    }
}
````

> This wrapper demonstrates label normalization, separation of parse vs. decision failures, and latency observation.

### 7.2 Tracing instrumentation (optional)

```rust
#[cfg(feature = "trace")]
#[tracing::instrument(
    name = "lib.ron_auth.verify",
    skip_all,
    fields(tenant = ctx.tenant, caveats = ?token.caveats_len())
)]
fn verify_instrumented(...) -> Result<Decision, AuthError> { /* call into core */ }
```

---

## 8) Log event cheatsheet (host)

| Event                  | Level  | Fields                                                         | When                                                              |
| ---------------------- | ------ | -------------------------------------------------------------- | ----------------------------------------------------------------- |
| `auth.verify`          | `INFO` | `result`, `latency_us`, `tenant?`, `token_digest8?`, `corr_id` | On every verification (sample in prod if high QPS)                |
| `auth.deny`            | `WARN` | `reason`, `tenant?`, `corr_id`, `latency_us`                   | On deny; consider rate-limiting noisy reasons (e.g., `parse.b64`) |
| `auth.rotation.window` | `INFO` | `active_kid`, `prev_kids`, `window_days`                       | On rotation rollouts (host-side)                                  |
| `auth.policy.digest`   | `INFO` | `digest`, `source`                                             | When governance digest changes (host-side)                        |

---

## 9) Dashboards (suggested panels)

* **Outcomes:** stacked `ron_auth_verify_total{result}` over time
* **Denies by reason:** top-N table of `ron_auth_deny_total` deltas
* **Latency:** p50/p95/p99 from `ron_auth_duration_us`
* **Unknown KID:** time series, broken down by tenant
* **Token size distribution:** histogram heatmap (sanity check against bounds)

---

## 10) Stability & versioning

* Metric names and **deny reason strings** are **SemVer-stable API** for this crate.
* Any rename/add/remove is a **breaking change** (major version) unless purely additive with clear defaults (new reason strings start as **off-path** until documented).

---

## 11) Security notes (observability)

* Never include token plaintext or caveat bodies in metrics, logs, or traces.
* Redaction rule: **digest-then-truncate**, never substring of raw token.
* Correlate with `corr_id` and `tenant` (if safe), not with secrets.

---

## 12) N/A items (library context)

* `/metrics`, `/healthz`, `/readyz` endpoints — **host-only**.
* Bus lag & supervised restarts — **host-only**.
* I/O timeouts & framing metrics — **host-only**.

---

## 13) Review cadence

* Re-validate SLOs and alert thresholds **every 90 days** or after perf-affecting changes.
* Keep this file in scope for any change to error taxonomy or `DenyReason` strings.

---

```
```
