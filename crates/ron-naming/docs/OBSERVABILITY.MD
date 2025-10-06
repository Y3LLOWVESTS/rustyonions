

---

```markdown
# 📈 OBSERVABILITY.md — ron-naming

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

> Role recap: `ron-naming` is a **library** for naming schemas, normalization, and canonical encodings (no network, no DB).  
> It also exposes an **optional offline CLI** (`tldctl`, feature `cli`) for authoring/linting/packing/signing/verifying governance artifacts.  
> Runtime surfaces like `/metrics`, `/healthz`, `/readyz` are provided by **host services** (e.g., `svc-index`) that embed this crate.

---

## 0) Purpose

Define **what is observable** for ron-naming and **how hosts should expose it**:

* Library-level signals for **validation/normalization** work
* CLI (`tldctl`) signals for **artifact authoring** workflows
* Conventions for logs, tracing, and correlation
* Recommended **host-facing** metrics, health semantics, and alerts

---

## 1) Metrics (Prometheus-style)

> The library itself does not run a server. Metrics are **emitted by hosts** that call into ron-naming (e.g., `svc-index`, a build tool, or a test harness).  
> `tldctl` is offline; it **does not** scrape/serve Prometheus but can print JSON events that hosts can ingest.

### 1.1 Library “golden” signals (to be exported by hosts)

- `naming_manifest_validation_seconds{operation,ok}` (Histogram)  
  - `operation ∈ {normalize,validate,encode,decode}`  
  - Bucket policy should capture typical artifact sizes (small/medium/large).
- `naming_manifest_parse_errors_total{reason}` (Counter)  
  - `reason` examples: `unknown_field`, `oversize`, `unicode_confusable`, `mixed_script`, `bad_address_format`.
- `naming_address_hygiene_total{result}` (Counter)  
  - `result ∈ {accepted,rejected}` for `b3:<hex>` guards and near-miss rejects.
- `naming_canonical_bytes_total{format}` (Counter)  
  - Count of canonical encodings emitted by hosts (e.g., `json`, `cbor`).

> Registration discipline: **register once** at host init; pass handles into code paths that call the library; avoid duplicate registration.  

### 1.2 CLI (`tldctl`) signals (offline)

`tldctl` emits **structured JSON events** to stderr (one line per event) that a wrapper can ship to logs/metrics:

- `event="tldctl_op"` with fields: `op ∈ {lint,pack,sign,verify,show}`, `ok`, `duration_ms`, `input_bytes`, `output_bytes`.  
- Aggregators may transform these into counters/histograms:
  - `tldctl_ops_total{op,ok}` (Counter)  
  - `tldctl_op_duration_seconds{op}` (Histogram)

### 1.3 What **not** to do

- No direct network exporters inside ron-naming.  
- No implicit global registries; hosts **own** registration and lifetimes.

---

## 2) Health & Readiness

> ron-naming has **no server**; health is **host-defined**. Use the following guidance when embedding:

### 2.1 Endpoints (host responsibility)

- `/healthz` — simple process liveness (host).  
- `/readyz` — returns `200` only after the host has loaded config, attached bus/listeners, and validated any naming policies it relies on.

### 2.2 Readiness keys (when naming policy is material to serving)

- `naming_policy_loaded` — policy tables (Unicode/IDNA/confusables) pinned and validated.  
- `naming_vectors_loaded` — golden vectors parsed and ready for spot checks.  
- `naming_selftest_ok` — host performed a quick round-trip (normalize→encode→decode) at boot.

### 2.3 Failure semantics (host convention)

- Reads can be **fail-open** only if policy isn’t critical for the route; otherwise **fail-closed** with `503` and JSON body:  
  `{ "degraded": true, "missing": ["naming_policy_loaded"], "retry_after": 1 }`.

---

## 3) Logs

### 3.1 Structure & fields

- JSON Lines; one event per line; UTC timestamps.
- Recommended fields produced by hosts and/or `tldctl`:
  - `ts` (ISO-8601), `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  - `service` (host service name or `tldctl`)
  - `event` (e.g., `naming.validate`, `naming.normalize`, `tldctl_op`)
  - `reason` (align with metrics labels when a reject/err happens)
  - `corr_id` (ULID/UUID propagated by host)
  - `duration_ms`, `bytes_in`, `bytes_out`
  - `content_id` (if an artifact has a `b3:<hex>` address)
  - `op` (for `tldctl`: `lint|pack|sign|verify|show`)

### 3.2 Redaction discipline

- No PII; no secrets; never log private keys or capability tokens.
- If logging config snapshots, redact secret keys; log **key IDs** only when signatures are referenced.

### 3.3 Error taxonomy (examples)

- `unknown_field`, `oversize`, `unicode_confusable`, `mixed_script`, `bad_address_format`,
  `canonical_mismatch`, `signature_invalid`, `vectors_mismatch`.

---

## 4) Tracing & Correlation

- Use `tracing` + `tracing-subscriber` (JSON) in hosts; `tldctl` can emit spans in JSON when `--trace` is set.
- Span naming:
  - `lib.ron_naming.normalize`, `lib.ron_naming.validate`
  - `cli.tldctl.lint|pack|sign|verify|show`
  - Hosts wrap these with higher-level spans like `svc.index.resolve`
- Correlation:
  - In hosts: inject/propagate `X-Corr-ID`; include `corr_id` in log events and span fields.
  - In CLI mode: generate a fresh `corr_id` per invocation (printed in the first event) so shells/wrappers can stitch multi-step workflows.
- OTEL:
  - Exporters live in hosts; gate with `otel` feature flags at the host level.

---

## 5) Alerts & SLOs

> Alerts target **hosts** running naming-critical routes (e.g., `svc-index`) and CI/workflows that use `tldctl`. Suggestion set:

### 5.1 CI/Build-time SLOs (applicable to both hosts and CLI pipelines)

- **Golden vectors** pass rate = 100% (hard gate).  
- **Canonical stability**: no diff in canonical bytes for unchanged fixtures.  
- **Validation performance**: p95 `naming_manifest_validation_seconds{operation=validate}` below a budget for representative sizes (document per repo).

### 5.2 Runtime SLOs (hosts using ron-naming on hot paths)

- Policy availability: `naming_policy_loaded` true ≥ 99.99% during serving windows.  
- Validation latency: host-defined p95/p99 budgets per route (e.g., `resolve`).

### 5.3 Alert examples

- `increase(rate(naming_manifest_parse_errors_total{reason=~"unknown_field|confusable"}[5m]))` above baseline for 15m → **WARN** (data quality drift or abuse).  
- Boot readiness missing key: `absent(naming_selftest_ok == 1)` for > 60s after process start → **CRIT** (bad deploy).  
- Canonical bytes drift in CI: diff detected on protected fixtures → **CRIT** (compat regression).

### 5.4 Runbooks

- Link to `RUNBOOK.md` with steps: confirm policy versions, re-run vectors, inspect last schema change, bisect confusables/mixed-script rejects.

---

## 6) CI / Enforcement

- **Vectors & pins:** Unicode/IDNA/confusable tables are **pinned**; CI regenerates and compares canonical outputs; any delta requires explicit approval and updated vectors.  
- **No network invariant:** `tldctl` must not attempt network calls; CI runs with a network sandbox and fails if egress is observed.  
- **Metrics discipline:** host crates register metrics once; a CI grep/lint prevents duplicate registration names.  
- **Tracing discipline:** spans exist for every public library entrypoint used on hot paths; smoke tests assert presence of span fields (`corr_id`, `operation`, `ok`).  
- **Docs freshness:** review this file on a cadence (e.g., quarterly) and stamp the header date.

---

## 7) Reference Implementations (how to wire it)

- **Host (Rust, Axum):** wrap calls to `normalize/validate` with latency timers; increment counters on typed error categories; expose `/metrics`.  
- **CLI pipeline:** run `tldctl ... --json` and pipe stderr to your log shipper; convert `event="tldctl_op"` lines into counters/histograms in your collector.

---

## 8) Known Non-Goals

- ron-naming will not open sockets, publish `/metrics`, or own readiness endpoints.  
- No global singletons or background tasks; all observability is **call-scoped** and **host-owned**.

---
```
