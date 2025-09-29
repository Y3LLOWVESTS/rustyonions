

---

# 📈 OBSERVABILITY.md — `ron-proto`

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*
*crate-type: **lib** (pure DTOs; no I/O, no HTTP)*

---

## 0) Purpose

`ron-proto` defines **wire/domain types** for OAP/1 envelopes and service DTOs. Being a **library**, it does not expose endpoints or register metrics. Its observability role is to:

* Standardize **metrics labels**, **log fields**, and **span names** for services that use these DTOs.
* Provide **stable reason strings**, **schema fingerprints**, and **helpers** that eliminate drift.
* Enforce **schema compatibility** and **redaction** via CI and tests.

### 0.1 Six Concerns Mapping

| Concern  | `ron-proto` Contribution                                              | CI/Proof Gate                               |
| -------- | --------------------------------------------------------------------- | ------------------------------------------- |
| **SEC**  | Redaction rules; `deny_unknown_fields`; reason canon prevents leakage | schema-guard tests; redaction tests         |
| **RES**  | Uniform rejected/inflight/latency breakdown via `proto_*` taxonomy    | service greps for canonical labels          |
| **PERF** | Clear encode/decode latency buckets; oversize detection guidance      | perf smoke with `proto.decode/encode` spans |
| **GOV**  | ABI fingerprint & version exposure; immutable reason strings          | public-api + fingerprint snapshot           |
| **DX**   | Helper fns (`as_metric_reason`, `trace_fields`), DTO short-names      | doc examples + lints                        |
| **ECON** | (Indirect) Clean signals for quota/SLAs built atop reasons            | SLO/alert wiring in services                |

#### Facet tie-ins (Developer Suite)

* **Graph/Search/Index:** `type=ManifestV1` metrics + `schema_ok` in `/readyz` keep ingest healthy.
* **Media:** `proto_oap_oversize_total` informs byte-range/ingress caps (1 MiB OAP frame).
* **Trust & Safety:** reason canon aids auditability across services.

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden (emitted by services using `ron-proto`)

* `http_requests_total{route,method,status}`
* `request_latency_seconds{route,method}`
* `inflight_requests{route}`
* `rejected_total{reason}` — **use `ron-proto` reason canon (§7)**
* `service_restarts_total`
* `bus_lagged_total` (if bus involved)

### 1.2 Proto-Specific Taxonomy (service-side)

* `proto_decode_total{type,component,result}`
* `proto_decode_latency_seconds{type,component}`
* `proto_schema_incompat_total{type,component}`
* `proto_unknown_fields_total{type}`
* `proto_addr_invalid_total{why}` (`bad_prefix|bad_len|non_hex`)
* `proto_oap_oversize_total` (attempted OAP frame > **1 MiB**)
* `proto_manifest_invalid_total`

`type` = stable DTO short name (e.g., `OapHello`, `ManifestV1`, `CapTokenHdr`).
`component` = local stage (e.g., `ingress`, `indexer`, `storage`).

### 1.3 Registration Discipline

* Services register metrics **once** (normal Prometheus practice).
* `ron-proto` exports **string helpers/constants**—no `prometheus` dep—to avoid label drift:

  * `as_metric_reason(&ProtoErrorKind) -> &'static str`
  * `PROTO_REASON_*` constants

---

## 2) Health & Readiness (service-side)

* `/healthz` — liveness
* `/readyz` — readiness after config/DB/bus/listeners **and** proto checks

### 2.1 Readiness Keys contributed by `ron-proto`

* `proto_loaded` — crate linked and initialized
* `schema_ok` — compatibility guard passed

Expose version/fingerprint via `/version`:

* `PROTO_VERSION: &str` (crate semver)
* `PROTO_ABI_FINGERPRINT: &str` (stable hash over schema descriptors)

### 2.2 Failure Semantics

* Follow canon: **fail-open reads / fail-closed writes**.
* If schema checks fail, `/readyz` degrades with:

  ```json
  { "degraded": true, "missing": ["schema_ok"], "retry_after": 1 }
  ```

---

## 3) Logs (service-side JSON lines)

### 3.1 Required Fields

`ts`, `level`, `service`, `event` (`proto.decode|proto.encode|proto.schema.check`),
`type`, `result` (`ok|error`), `reason` (**canon**), `corr_id`,
`proto_version`, `proto_fingerprint`, optional `latency_ms`, optional `content_id` (IDs like `"b3:<hex>"` only).

### 3.2 Redaction & Secrets

* **No payload bytes**, **no capability tokens**, **no PII**.
* Only log **identifiers** (e.g., content IDs), never raw content.
* Config snapshot logs must **redact** secret fields.

---

## 4) Tracing & Correlation

* Use `tracing` in Rust services; recommended spans:

  * `proto.decode.<Type>`, `proto.encode.<Type>`, `proto.schema.check`
* Span fields: `type`, `result`, `reason`, `proto_version`, `proto_fingerprint`.
* Correlation: inject `X-Corr-ID` at ingress; propagate via bus/SDK.

**Polyglot SDKs (TS/Python/etc.)** MUST propagate `corr_id` and emit equivalent OTEL spans with these attributes.

---

## 5) SLOs & Alerts (wired in services)

### 5.1 SLOs (read path)

* p95 latency: **< 80 ms** intra-region, **< 200 ms** inter-region
* 5xx rate **< 0.1%**; 429/503 **< 1%**
* RF(observed) ≥ RF(target)

### 5.2 Example Alerts (reason canon applied)

* `increase_over_time(proto_schema_incompat_total[10m]) > 0` → warn/page
* `rate(proto_unknown_fields_total[5m]) > 0` post-deploy → warn
* `increase_over_time(proto_oap_oversize_total[10m]) > 0` → check ingress caps
* `rate(rejected_total{reason="proto.addr.invalid"}[5m]) > 0` → input audit

### 5.3 Runbooks

Each alert links to the owning service’s `RUNBOOK.md` (triage: verify `proto_version`/`proto_fingerprint`, assess schema diffs, adjust caps/roll back if needed).

---

## 6) CI / Enforcement (this crate’s teeth)

* **Schema Compatibility Guard:** snapshot descriptors in `tests/schema/`; `#[serde(deny_unknown_fields)]` on external DTOs.
* **Public API Guard:** `cargo public-api -p ron-proto` + semver checks.
* **Fuzz/Property Tests:** OAP envelopes (HELLO/START/DATA/END/ACK/ERROR), Manifest, CapToken headers.
* **Reason Canon:** test `all_reasons_unique_and_stable()`; adding a reason requires changelog entry.
* **No I/O / No Globals:** `#![forbid(unsafe_code)]`, no `lazy_static!/static mut`.

### 6.1 Copy-Paste GitHub Actions Snippet

```yaml
name: ron-proto CI
on: [push, pull_request]
jobs:
  proto-ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build & Test
        run: |
          cargo build -p ron-proto
          cargo test  -p ron-proto --features schema-guard
      - name: Public API (semver)
        run: |
          cargo install cargo-public-api || true
          cargo public-api -p ron-proto --deny-breaking
      - name: Reason Canon Check
        run: cargo test -p ron-proto --test reason_canon
```

### 6.2 Workspace Greps (service repos)

* Deny ad-hoc `rejected_total{reason=...}` not in `ron-proto` canon.
* Require `/readyz` to surface `schema_ok`; `/version` to expose `proto` block.

---

## 7) Canonical Reason Strings (table)

| Reason String               | Description                         | Alert (shorthand)             |
| --------------------------- | ----------------------------------- | ----------------------------- |
| `proto.decode.error`        | Generic decode failure              | `rate[5m] > 0` → warn         |
| `proto.encode.error`        | Generic encode failure              | CI/unit tests                 |
| `proto.schema.mismatch`     | Incompatible shape/semver           | `Δ[10m] > 0` → page           |
| `proto.unknown.fields`      | Tripped `deny_unknown_fields`       | `rate[5m] > 0` → warn         |
| `proto.addr.invalid`        | Malformed `b3:<hex>`                | `rate[5m] > 0` → audit        |
| `proto.oap.oversize`        | OAP frame > **1 MiB**               | `Δ[10m] > 0` → check caps     |
| `proto.manifest.invalid`    | Structural invariants failed        | `Δ[10m] > 0` → page           |
| `proto.cap.invalid`         | Capability header malformed/expired | `Δ[10m] > 0` → sec review     |
| `proto.enum.out_of_range`   | Invalid discriminant encountered    | `rate[5m] > 0` → warn         |
| `proto.version.unsupported` | DTO version newer than runtime      | `rate[5m] > 0` → rollback/fwd |

> **Immutability rule:** Once released, reason strings are stable. Deprecate—do not mutate.

---

## 8) Version & Fingerprint Exposure

Services MUST include in logs and `/version`:

```json
{
  "service": "<name>",
  "version": "<svc-semver>",
  "proto": { "version": "x.y.z", "fingerprint": "<8-hex>" }
}
```

Used for auditability, rollback decisions, and cross-service correlation.

---

## 9) Traps & Anti-Patterns

* Logging payload bytes or capability tokens.
* Ad-hoc reason strings instead of `ron-proto` helpers.
* Accepting unknown fields “to be forgiving.”
* Confusing **OAP frame cap (1 MiB)** with **storage chunk size (~64 KiB)**.
* Emitting metrics directly from this crate (libs don’t register metrics).
* **Amnesia (Micronode default = ON):** services MUST **zeroize** transient DTO buffers after use.
* **PQ trap:** never log PQ-hybrid key/cap metadata—expose **fingerprints only**.

---

## 10) Definition of Done (crate)

* [ ] External DTOs use `#[serde(deny_unknown_fields)]`; tests enforce.
* [ ] Canonical reason strings + helpers exported; stability test present.
* [ ] Schema snapshots & compatibility tests are green.
* [ ] Fuzz/property tests cover envelopes and headers.
* [ ] No I/O, no globals; `unsafe` forbidden.
* [ ] Docs show how services should wire metrics/logs/spans using this crate.

---

## 11) Quick Reference (for service owners)

* **Metrics:** reuse golden set; add `proto_*` per §1.2; label `reason` with canon.
* **Logs:** `event=proto.decode|proto.encode|proto.schema.check`, include `type`, `result`, `reason`, `proto_version`, `proto_fingerprint`, `corr_id`; never payloads.
* **Spans:** `proto.decode.<Type>` / `proto.encode.<Type>` with standard fields.
* **Readiness:** include `schema_ok`; expose `/version` with `proto` block.
* **Alerts:** watch `proto_schema_incompat_total`, `proto_unknown_fields_total`, and `rejected_total{reason~"^proto\\."}`.

---

## 12) Signal Flow (Mermaid)

```mermaid
flowchart LR
  A[Ingress (HTTP/Bus)] -->|X-Corr-ID| B[proto.decode.<Type> span]
  B --> C{Decode OK?}
  C -- No --> D[Log WARN error + reason] --> E[rejected_total{reason}]
  C -- Yes --> F[Business Logic / Storage / Index]
  B --> G[proto_decode_total & latency]
  F --> H[proto.encode.<Type> span]
  H --> I[Logs + Metrics]
  I --> J[Alerts (SLO breaches)]
```

**End of file.**
