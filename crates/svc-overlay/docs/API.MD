````markdown
---
title: API Surface & SemVer Reference — svc-overlay
status: draft
msrv: 1.80.0
last-updated: 2025-10-03
audience: contributors, auditors, API consumers
---

# API.md — svc-overlay

## 0) Purpose

This document captures the **public API surface** of `svc-overlay`, including the **binary/CLI**, **admin HTTP endpoints**, **metrics & labels**, and (optional) **Rust library surface** gated by features. It defines **SemVer discipline** for each surface, provides **CI-enforceable** checks, and acts as the contract for external consumers.

> Role recap: `svc-overlay` is a **service crate** for the session/gossip plane (OAP/1). It deliberately **does not** expose broad Rust APIs by default.

---

## 1) Public API Surface

### 1.1 Binary / CLI surface (stable)

**Executable name:** `svc-overlay`

**Synopsis**
```bash
svc-overlay [FLAGS] [--config <path>]
````

**Flags & options (contract)**

```text
--config <path>            Load Config.toml/Config.json (low precedence)
--bind <ip:port>           Bind address for overlay listener (overlay.listen)
--advertise <host:port>    Advertised address (overlay.advertise_addr)
--metrics <ip:port>        Admin/metrics bind (observability.metrics_addr)
--max-conns <num>          Transport connection ceiling (transport.max_conns)
--read-timeout <dur>       Read timeout, e.g., 5s, 250ms (timeouts.read)
--write-timeout <dur>      Write timeout (timeouts.write)
--idle-timeout <dur>       Idle timeout (timeouts.idle)
--fanout <n>               Gossip relay fanout (overlay.gossip.relay_fanout)
--burst <n>                Gossip burst budget (overlay.gossip.burst_budget_frames)
--chunk-bytes <size>       Chunk size (overlay.gossip.chunk_bytes)
--pq <off|hybrid>          PQ mode (overlay.pq_mode)
--amnesia <true|false>     Amnesia mode (overlay.amnesia)
--tls                      Shorthand for tls.enabled=true
--tls-cert <path>          TLS certificate path (tls.cert_path)
--tls-key <path>           TLS key path (tls.key_path)
--uds <path>               UDS control socket (uds.path)
--version                  Print version (semver + git hash) and exit
--help                     Print help and exit
```

> **SemVer note:** Renaming, removing, or changing the meaning of any **existing** flag is **breaking** (major). Adding new flags is **additive** (minor). Default changes that alter behavior are **breaking** unless opt-in.

---

### 1.2 Admin HTTP surface (stable)

**Base path:** local admin interface (bind via `--metrics`), serving: `/healthz`, `/readyz`, `/metrics`, `/version`.

**Endpoints**

1. `GET /healthz` — **liveness**

```http
200 OK
Content-Type: application/json
{"alive": true}
```

2. `GET /readyz` — **readiness truth**

* **200 OK**

  ```json
  {"ready": true}
  ```
* **503 Service Unavailable**

  ```json
  {"ready": false, "degraded": true, "missing": ["<keys>"], "retry_after": 5}
  ```

3. `GET /metrics` — **Prometheus exposition**

* Content type: `text/plain; version=0.0.4`
* Stable **metric names** & **label sets** defined in §1.3.

4. `GET /version` — semantic version and build info

```json
{"version":"vX.Y.Z","git":"<shortsha>","build":"<timestamp>","features":["tls","pq?","arti?","quic?"]}
```

> **SemVer note:** Adding **new** endpoints is **minor**. Changing response **schema** or semantics of existing endpoints is **breaking**. Adding **optional** fields is **minor** if old clients continue to parse.

---

### 1.3 Metrics API (stable names & labels)

> **Metrics are part of the public contract.** Names and **label keys** are stable. Value ranges & units are documented.

**Golden set (every service)**

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram; buckets: `0.005..5.0,+Inf`)
* `inflight_requests{route}` (Gauge)
* `service_restarts_total` (Counter)
* `bus_lagged_total` (Counter)
* `rejected_total{reason}` (Counter) — `reason ∈ {unauth, oversize, ratio_cap, timeout, busy, handshake_ratelimit}`

**Overlay-specific**

* `overlay_sessions_active` (Gauge)
* `overlay_sessions_open_total` (Counter)
* `overlay_handshake_attempts_total` (Counter)
* `overlay_handshake_failures_total{reason}` (Counter) — `reason ∈ {timeout, tls, mtls, pq_neg_fail, ratelimit, proto}`
* `overlay_frames_total{dir,kind}` (Counter) — `dir ∈ {rx, tx}`, `kind ∈ {gossip, ctrl, other}`
* `overlay_bytes_total{dir}` (Counter) — `dir ∈ {rx, tx}`
* `overlay_frame_size_bytes{dir}` (Histogram; up to 1 MiB) — `dir ∈ {rx, tx}`
* `overlay_queue_depth{queue}` (Gauge) — `queue ∈ {ingress, gossip_ingress, egress}`
* `overlay_queue_dropped_total{queue,reason}` (Counter) — `reason ∈ {full, timeout, shed}`
* `overlay_backpressure_events_total{peer_bucket}` (Counter) — `peer_bucket ∈ {tiny, small, med, large}`
* `auth_failures_total{reason}` (Counter) — `reason ∈ {unauth, expired, invalid_caveat}`
* `tls_handshake_failures_total{reason}` (Counter) — `reason ∈ {cert, version, cipher, alpn}`
* `overlay_pq_negotiations_total{mode,outcome}` (Counter) — `mode ∈ {off, hybrid}`, `outcome ∈ {ok, fail, incompatible}`
* `readyz_state` (Gauge) — `0=down,1=ready,2=degraded`

> **SemVer note:** Renaming a metric, removing a label key, or changing units is **breaking**. Adding a **new** metric or a **new label value** is **minor** if existing dashboards/alerts remain valid.

---

### 1.4 Environment & Config keys (operational API)

* **Env prefix:** `SVC_OVERLAY_` (stable)
* **Nested env mapping:** `__` separator (stable)
* Selected **stable keys** (full list in `CONFIG.md`):

  * `overlay.listen`, `overlay.advertise_addr`, `overlay.amnesia`, `overlay.pq_mode`
  * `overlay.peer.inflight_frames`, `overlay.peer.max_rps`
  * `overlay.gossip.max_frame_bytes` (fixed to 1 MiB), `overlay.gossip.chunk_bytes`, `overlay.gossip.relay_fanout`
  * `timeouts.read|write|idle`, `observability.metrics_addr`
  * `tls.enabled|cert_path|key_path`, `uds.path|allow_uids`

> **SemVer note:** Renaming or removing a **key** is **breaking**. Adding keys with safe defaults is **minor**. Tightening validation that causes previously-valid configs to fail is **breaking** unless opt-in.

---

### 1.5 Rust library surface (feature-gated, optional)

By default, `svc-overlay` is a **binary-only** crate. For **embedders**, we provide an **opt-in** `libapi` feature exposing a minimal, stable Rust API.

**Feature:** `libapi` (off by default)

**Intended surface (stable while `libapi` exists)**

```text
pub mod api {
    pub struct OverlayConfig;          // re-export of validated Config snapshot (subset)
    pub struct OverlayHandle;          // shutdown & join handle
    pub fn spawn(cfg: OverlayConfig) -> anyhow::Result<OverlayHandle>;
}
```

> This is intentionally tiny. The full `Config` type and internals remain crate-private to allow evolution.

**Current `cargo public-api --simplified` (with `--features libapi`)**

```text
pub mod api
pub struct api::OverlayConfig
pub struct api::OverlayHandle
pub fn api::spawn(cfg: api::OverlayConfig) -> Result<api::OverlayHandle, anyhow::Error>
```

> **SemVer note:** Any change to the above **is breaking** unless purely additive (e.g., new `api::OverlayHandle` methods with default semantics).

---

## 2) SemVer Discipline

### 2.1 Surfaces & what counts as breaking

| Surface          | Examples                                      | Breaking (major)                                                                    | Additive (minor)                              |
| ---------------- | --------------------------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------- |
| **CLI**          | flags/options                                 | rename/remove/repurpose existing flag; change default in a way that alters behavior | add new flags; add aliases                    |
| **HTTP**         | `/healthz`, `/readyz`, `/metrics`, `/version` | change schema/meaning; remove endpoint                                              | add optional fields or new endpoints          |
| **Metrics**      | names, labels, units                          | rename metric; remove label key; change units/buckets materially                    | add new metric; add new label **value**       |
| **Config**       | TOML/env keys                                 | rename/remove keys; tighten validation breaking old configs                         | add new keys with safe defaults               |
| **Lib (libapi)** | `api::*`                                      | change signatures/types; remove items                                               | add new items; mark enums `#[non_exhaustive]` |

> **Patch**: doc fixes, performance improvements, and internal refactors with no surface change.

---

## 3) Stability Guarantees

* **MSRV**: `1.80.0`.
* **Unsafe**: forbidden unless explicitly justified and audited.
* **Privacy**: internal types and Tokio primitives are **not** exposed.
* **Feature flags**: `tls`, `pq`, `arti`, `quic`, `cli`, `libapi` — adding a new feature is **minor**; changing feature behavior that breaks default users is **breaking**.

---

## 4) Invariants (API-level)

* **Service-first**: Default build exposes only **CLI + Admin HTTP + Metrics**. Rust API is **opt-in** (`libapi`).
* **Metrics are API**: Names & label **keys** are stable. Avoid high-cardinality labels by policy.
* **Config is API**: Keys & env mappings are stable; validation is fail-closed and versioned.
* **Ready truthfulness**: `/readyz` reflects ability to accept **writes**; flipping semantics is **breaking**.
* **Public docs**: Every public item & endpoint is documented; `#![deny(missing_docs)]` for `libapi`.

---

## 5) Tooling

* `cargo public-api` — detect Rust surface diffs (with/without `--features libapi`).
* `cargo semver-checks` — optional semantic checking.
* **Golden files**:

  * Rust symbols → `docs/api-history/svc-overlay/<version>-libapi.txt`
  * Metrics list → `docs/api-history/svc-overlay/<version>-metrics.txt`
  * CLI flags help text → `docs/api-history/svc-overlay/<version>-cli.txt`
  * Admin schemas (examples) → `docs/api-history/svc-overlay/<version>-http.json`

**Commands**

```bash
cargo public-api --simplified --deny-changes
cargo public-api --simplified --features libapi --deny-changes
```

---

## 6) CI & Gates

* **Rust surface gate**

  * Run `cargo public-api` (both default and `--features libapi`).
  * If diff detected → PR must include CHANGELOG entry + API.md update.

* **Metrics contract gate**

  * Export once under synthetic load; parse names/labels.
  * Compare to `docs/api-history/...-metrics.txt` (whitelist changes explicitly).

* **HTTP contract gate**

  * Contract tests for `/healthz`, `/readyz`, `/version` schemas.
  * Prometheus `promtool` scrape test for `/metrics`.

* **Config contract gate**

  * Validate that legacy configs from `docs/api-history/...-config/` still start (unless major).

**GitHub Actions sketch**

```yaml
- run: cargo install cargo-public-api
- run: cargo public-api --simplified --deny-changes -p svc-overlay
- run: cargo public-api --simplified --features libapi --deny-changes -p svc-overlay
- run: cargo test -p svc-overlay --test http_contract
- run: promtool check metrics http://127.0.0.1:9600/metrics
```

---

## 7) Acceptance Checklist (Definition of Done)

* [ ] Current API snapshots generated & stored under `docs/api-history/svc-overlay/`.
* [ ] `cargo public-api` passes (default + `libapi`).
* [ ] Metrics & HTTP contract tests pass; `promtool` scrape OK.
* [ ] CHANGELOG updated for any surface changes (and SemVer bump chosen).
* [ ] `API.md` updated (this file).
* [ ] For new/changed endpoints: runbooks & dashboards updated.

---

## 8) Appendix

### 8.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 8.2 Perfection Gates tie-in

* **Gate G:** No undocumented public surface (Rust, HTTP, metrics, config).
* **Gate H:** Breaking changes require **major** version bump.
* **Gate J:** CHANGELOG alignment enforced by CI.

### 8.3 History (example placeholders; update per release)

```
v0.7.0
- Added metric overlay_pq_negotiations_total{mode,outcome} (minor)
- Added CLI flag --advertise (minor)
- /readyz schema gained "retry_after" (minor, optional field)

v0.6.0 (breaking)
- Renamed --egress-window -> --burst (breaking CLI)
- Changed /version schema field "hash" -> "git" (breaking HTTP)
```

---

### 8.4 Current Rust Surface (as of this doc)

> Default build (binary only): no public Rust items beyond `main` in the bin target.
> With `--features libapi`:

```text
pub mod api
pub struct api::OverlayConfig
pub struct api::OverlayHandle
pub fn api::spawn(cfg: api::OverlayConfig) -> Result<api::OverlayHandle, anyhow::Error>
```

```
```
