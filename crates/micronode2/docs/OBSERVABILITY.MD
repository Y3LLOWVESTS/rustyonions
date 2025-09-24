
---

# 📈 OBSERVABILITY.md — micronode

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus/OTEL)
* Health & readiness semantics
* Logs (JSON schema, fields)
* Tracing spans & correlation
* Alerts & SLOs

Micronode must surface **uniform, scrape-safe** signals: `/metrics`, `/healthz`, `/readyz`, `/version`, with deterministic traces and latency histograms for every ingress path.&#x20;

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (always present)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram; track p95)
* `inflight_requests{route}` (Gauge; reflects concurrency cap)
* `bus_lagged_total` (Counter; dropped broadcast backlog)
* `service_restarts_total` (Counter; crash-only supervision)
* `rejected_total{reason}` (Counter; quota/timeouts/errors)

These are mandated by the Hardening blueprint’s required taxonomy and golden-metrics guidance. &#x20;

**Labels (minimum set):**
`service="micronode"`, `route`, `method`, `status`, `instance`, `profile="micronode"`, `amnesia="on|off"` (reflecting runtime mode). Amnesia controls are first-class for micronode.&#x20;

### 1.2 Micronode-Specific (composed facets)

Micronode composes minimal facets (RAM storage tier, index cache, mailbox shard, mini-gateway). Expose the following when enabled:

* **Storage:** `storage_get_latency_seconds`, `chunks_written_total`, `integrity_fail_total` (BLAKE3 verify on read; corrupted chunk → 502 + counter). &#x20;
* **Mailbox:** `mailbox_visibility_timeout_total`, `mailbox_dlq_total`. &#x20;
* **Gateway/Ingress:** `quota_exhaustions_total`, `bytes_in_total`, `bytes_out_total`.&#x20;
* **(Optional) DHT bridge:** If micronode is built with discovery integration, mirror SLO gauges in aggregate dashboards only (not typical for micronode): `ron_dht_lookup_ms`, `ron_dht_missing_records`, `ron_dht_repair_bytes_total`.&#x20;

### 1.3 Registration Discipline

* Metrics are **registered once** (e.g., `ron-metrics::Metrics::new()`); pass clones to components.
* CI greps **deny duplicate registration**.&#x20;

---

## 2) Health & Readiness

### 2.1 Endpoints

* `/healthz` — liveness (always `200 OK` if the process is alive).
* `/readyz` — readiness (returns `200 OK` **only** after config is loaded, DB is open (if any), listeners bound, and bus attached).
* `/version` — build/version string for release hygiene. These admin surfaces are required across services. &#x20;

### 2.2 Readiness Keys (micronode profile)

* `db_open` (only when persistence is enabled; amnesia defaults to none)
* `bus_attached`
* `tor_bootstrap` (if Tor transport enabled)
* `hs_published` (if onion service profile enabled)&#x20;

### 2.3 Failure Semantics

* **Fail-open reads, fail-closed writes** under degradation.
* Degraded mode returns `503` with a machine-readable envelope:

  ```json
  { "degraded": true, "missing": ["<keys>"], "retry_after": 10 }
  ```

  This matches the macronode blueprint’s operator contract and is reused for micronode.&#x20;

**Ingress profile observability note:** under quota/overload, `/readyz` flips to “shed” and DRR queue depths should be visible.&#x20;

---

## 3) Logs

### 3.1 Format

* JSON Lines (`application/jsonl`), one event per line, with required fields:

  * `ts` (ISO8601 UTC)
  * `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `service` (e.g., `micronode`)
  * `event` (e.g., `oap_frame`, `quota_exhausted`, `service_crashed`)
  * `reason` (aligns with `rejected_total{reason}`)
  * `corr_id` (UUID/ULID propagated)
  * `latency_ms` (if applicable)
  * `content_id` (for storage/overlay ops)&#x20;

### 3.2 Redaction & Secrets

* **No PII, keys, or payloads** in logs.
* Config snapshots must **redact secrets** (KMS, macaroon sigs, Tor keys).
* **Amnesia mode:** logs are ephemeral; zeroized on shutdown (micronode default posture).&#x20;

---

## 4) Tracing & Correlation

* Use `tracing` with JSON subscriber output.
* Span naming: `svc.micronode.<operation>` (e.g., `svc.micronode.get_object`).
* Correlation IDs:

  * Injected on ingress (`X-Corr-ID`)
  * Propagated via the bus and SDK calls
* Optional OpenTelemetry exporters are gated behind a feature.&#x20;

---

## 5) Alerts & SLOs

### 5.1 Standard SLOs

* Public GET p95 latency < **80 ms** intra-region; < **200 ms** inter-region.
* 5xx error rate < **0.1%**.
* 429/503 < **1%**.
* `rf_observed ≥ rf_target` (if replication is in use).&#x20;

### 5.2 Alerts (examples)

* `bus_lagged_total > 0` sustained (backpressure visible).
* `quota_exhaustions_total > 100/min` (burst detection).
* `integrity_fail_total > 0` (critical).
* `service_restarts_total > 5 in 5m` (crash storm).
* **Optional DHT mirror (if bridged):** `ron_dht_missing_records > 0 for 10m` (warn), 30m (crit).&#x20;

### 5.3 Runbooks

Each alert links to `RUNBOOK.md` with **triage & recovery** steps; reuse the Scaling blueprint’s DHT/RF runbooks where applicable.&#x20;

---

## 6) CI / Enforcement

* CI verifies **admin surfaces** exist (`/metrics`, `/healthz`, `/readyz`) and that reject reasons align with crash reasons.
* Clippy/TSan/Loom walls for readiness DAG and concurrency safety.
* **Self-test container** (per service):

  1. wait on `/readyz`; 2) send **2 MiB** → expect **413**; 3) scrape `/metrics` → `request_latency_seconds` present.
* Docs & dashboards refreshed **every 90 days**. &#x20;

---

### Notes on Profile Posture

* Micronode’s **amnesia mode** is the default; ensure the `amnesia` label in metrics and **aggressive readiness degradation** (shed → fail) to avoid on-disk spill.&#x20;

---
