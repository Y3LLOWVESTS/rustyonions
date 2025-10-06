

---

````markdown
---
title: Configuration — ron-metrics
crate: ron-metrics
owner: Stevan White
last-reviewed: 2025-10-05
status: draft
template_version: 1.1
---

# Configuration — ron-metrics

This document defines **all configuration** for `ron-metrics`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> - **Library:** primary role. Provides a tiny HTTP exposer used by services to publish `/metrics`, `/healthz`, `/readyz`.  
> - **Service:** when embedded in services (gateway, overlay, storage, etc.), those services should surface these settings in their own `CONFIG.md` and forward to `ron-metrics`.

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (e.g., `Config.toml` near the binary)  
4. **Built-in defaults** (hard-coded)

> On dynamic reload, the effective config is recomputed under the same precedence.

**Supported file formats:** TOML (preferred), JSON (optional).  
**Path resolution for `--config` (if relative):** `./`, `$CWD`, crate dir.  
**Env prefix:** `RON_METRICS_` (e.g., `RON_METRICS_METRICS_ADDR`).

---

## 2) Quickstart Examples

### 2.1 Minimal service start (local scrape)
```bash
RUST_LOG=info
RON_METRICS_METRICS_ADDR=127.0.0.1:0
RON_METRICS_LABELS_SERVICE=my-service
RON_METRICS_LABELS_INSTANCE=$(hostname)-1
RON_METRICS_LABELS_AMNESIA=off
cargo run -p my-service-bin
````

### 2.2 Config file (TOML)

```toml
# Config.toml (excerpt used by ron-metrics)
[metrics]
metrics_addr = "127.0.0.1:0"   # prefer loopback; port 0 auto-selects
uds_path     = ""              # if set, UDS is used instead of TCP
uds_dir_mode  = "0700"
uds_sock_mode = "0600"
read_timeout  = "2s"
write_timeout = "2s"
idle_timeout  = "15s"

[tls]
enabled   = false
cert_path = ""
key_path  = ""

[labels]
service       = "my-service"
instance      = "node-a-1"
build_version = "1.2.3+sha.abc"
amnesia       = "off"
extra         = { region = "us-east-1", az = "use1-az1" }

[histograms]
request_latency_seconds = [0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0]

[pq]
mode = "off"   # off | hybrid

[zk]
enabled = false

[otel]
enabled = false  # requires cargo feature `otel`
```

### 2.3 CLI flags (override file/env)

```bash
--metrics 127.0.0.1:0 \
--metrics-uds /var/run/ron/metrics.sock \
--read-timeout 2s --write-timeout 2s --idle-timeout 15s \
--label service=my-service --label instance=$(hostname) \
--label build_version=$(git describe --always) --amnesia on \
--histogram request_latency_seconds=[0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0] \
--pq-mode hybrid \
--zk \
--otel
```

> **Embedding (lib) one-liner**
> In your bin’s `main.rs`:
>
> ```rust
> let cfg = ron_metrics::config::Config::default(); // or load/merge
> cfg.validate()?;
> let metrics = ron_metrics::Metrics::global();
> let (_task, bound) = metrics.clone().serve(cfg.metrics.bind_addr()?, health.clone()).await?;
> tracing::info!(%bound, "metrics/health/ready exposer bound");
> ```

---

## 3) Schema (Typed, With Defaults)

> **Durations** accept `ms`, `s`, `m`, `h`.
> **Paths** must exist if provided. **Modes** are octal strings validated at runtime.
> **Reload:** columns note whether a live reload is disruptive (requires rebind) or not.

| Key / Env Var / CLI                                                                                                                 | Type                      | Default           | Reload | Description                                              | Security Notes                         |
| ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ----------------- | :----: | -------------------------------------------------------- | -------------------------------------- |
| `metrics.metrics_addr` / `RON_METRICS_METRICS_ADDR` / `--metrics`                                                                   | socket                    | `127.0.0.1:0`     |  **D** | TCP bind for `/metrics`, `/healthz`, `/readyz`           | Prefer loopback in prod                |
| `metrics.uds_path` / `RON_METRICS_UDS_PATH` / `--metrics-uds`                                                                       | path                      | `""`              |  **D** | If set, expose over UDS instead of TCP                   | Dir `0700`, sock `0600`, SO_PEERCRED   |
| `metrics.uds_dir_mode` / `RON_METRICS_UDS_DIR_MODE`                                                                                 | octal string              | `"0700"`          |    N   | UDS parent dir permissions                               | Enforced on create                     |
| `metrics.uds_sock_mode` / `RON_METRICS_UDS_SOCK_MODE`                                                                               | octal string              | `"0600"`          |    N   | UDS socket file mode                                     | Enforced on bind                       |
| `metrics.read_timeout` / `RON_METRICS_READ_TIMEOUT` / `--read-timeout`                                                              | duration                  | `2s`              |    N   | Per-connection read timeout                              | DoS mitigation                         |
| `metrics.write_timeout` / `RON_METRICS_WRITE_TIMEOUT` / `--write-timeout`                                                           | duration                  | `2s`              |    N   | Per-connection write timeout                             | DoS mitigation                         |
| `metrics.idle_timeout` / `RON_METRICS_IDLE_TIMEOUT` / `--idle-timeout`                                                              | duration                  | `15s`             |    N   | Keep-alive idle shutdown                                 | Resource hygiene                       |
| `tls.enabled` / `RON_METRICS_TLS_ENABLED` / `--tls`                                                                                 | bool                      | `false`           |  **D** | Enable TLS for exposer                                   | Only tokio-rustls                      |
| `tls.cert_path` / `RON_METRICS_TLS_CERT_PATH` / `--tls-cert`                                                                        | path                      | `""`              |  **D** | PEM cert path                                            | Secrets on disk; perms `0600`          |
| `tls.key_path` / `RON_METRICS_TLS_KEY_PATH` / `--tls-key`                                                                           | path                      | `""`              |  **D** | PEM key path                                             | Zeroize in memory                      |
| `labels.service` / `RON_METRICS_LABELS_SERVICE` / `--label-service`                                                                 | string                    | process name      |    N   | `service` label applied to all metrics                   | Must be stable per binary              |
| `labels.instance` / `RON_METRICS_LABELS_INSTANCE` / `--label-instance`                                                              | string                    | hostname          |    N   | `instance` label                                         | Unique per instance                    |
| `labels.build_version` / `RON_METRICS_LABELS_BUILD_VERSION` / `--label-build`                                                       | string                    | crate version     |    N   | `build_version` label                                    | Inject at build if possible            |
| `labels.amnesia` / `RON_METRICS_LABELS_AMNESIA` / `--amnesia`                                                                       | enum(`on`,`off`)          | kernel flag       |    N   | Mirrors kernel amnesia flag                              | **MUST** reflect truth                 |
| `labels.extra` / `RON_METRICS_LABELS_EXTRA`                                                                                         | map<string,string> (JSON) | `{}`              |    N   | Arbitrary extra static labels                            | Avoid high-cardinality                 |
| `histograms.request_latency_seconds` / `RON_METRICS_HISTOGRAMS_REQUEST_LATENCY_SECONDS` / `--histogram request_latency_seconds=[…]` | list<float>               | canonical buckets |    N   | Histogram buckets for request latency                    | Changing affects dashboards            |
| `pq.mode` / `RON_METRICS_PQ_MODE` / `--pq-mode`                                                                                     | enum(`off`,`hybrid`)      | `off`             |    N   | PQ readiness toggle (observability only)                 | Interop posture; families always exist |
| `zk.enabled` / `RON_METRICS_ZK_ENABLED` / `--zk`                                                                                    | bool                      | `false`           |    N   | Emit ZK metrics families (may remain zero)               | Observational only                     |
| `otel.enabled` / `RON_METRICS_OTEL_ENABLED` / `--otel`                                                                              | bool (feature-gated)      | `false`           |  **D** | Use OTEL exporter instead of Prometheus (feature `otel`) | Keep public API stable                 |

> **Reload legend:** **N** = non-disruptive; **D** = disruptive (requires rebind / server restart).

---

## 4) Validation Rules (Fail-Closed)

On startup or reload:

* If **both** `metrics_addr` and `uds_path` are set → **fail** (choose one).
* `metrics_addr` must parse; if binding to public interfaces, log **warning** and require ops sign-off.
* If `uds_path` set: ensure parent dir exists (or create) with `0700`; socket mode `0600`; enable **SO_PEERCRED** allow-listing at the caller/service layer if needed.
* If `tls.enabled=true`: `cert_path` & `key_path` must exist and be readable; key not world-readable.
* `labels.service`, `labels.instance`, `labels.build_version`, `labels.amnesia` must be non-empty after resolution; `amnesia` ∈ {`on`,`off`}.
* Histogram bucket list must be strictly increasing, > 0 values.
* **PQ/ZK cross-check (observability only):**

  * If `pq.mode="hybrid"` **and** crate built **without** `pq` feature → log **warning** (`pq.mode set but feature disabled`), still start with zeroed families.
  * If `zk.enabled=true` **and** crate built **without** `zk` feature → log **warning** (`zk.enabled true but feature disabled`), still start with zeroed families.

**On violation:** structured log + **exit non-zero** (service bins) or **return error** (lib init).
**On PQ/ZK mismatch:** warn (non-fatal), metrics families still present (zeros) to keep dashboards stable.

---

## 5) Dynamic Reload (If Supported)

* **Triggers:** SIGHUP (service bin) **or** kernel bus event `ConfigUpdated { version }`.
* **Atomicity:** Build new `Config`, validate, then swap an `Arc<Config>` (no `.await` while holding locks).
* **Non-disruptive keys:** timeouts, labels, histogram buckets, pq/zk toggles.
* **Disruptive keys:** `metrics_addr`, `uds_path`, `tls.*`, `otel.enabled` (rebind/exporter swap).
* **Audit:** Emit `KernelEvent::ConfigUpdated { version }` and a redacted diff (no secrets).

---

## 6) CLI Flags (Canonical, bins only)

```
--config <path>                  
--metrics <ip:port>              
--metrics-uds <path>             
--read-timeout <dur>             
--write-timeout <dur>
--idle-timeout <dur>
--tls                            
--tls-cert <path>
--tls-key <path>
--label <k=v>                    # merges into labels.extra
--label-service <name>
--label-instance <name>
--label-build <version>
--amnesia <on|off>
--histogram request_latency_seconds=[comma-separated-floats]
--pq-mode <off|hybrid>
--zk
--otel                           # feature-gated
```

---

## 7) Feature Flags (Cargo)

| Feature | Default | Effect                                                       |
| ------: | ------: | ------------------------------------------------------------ |
|   `tls` |     off | Enable TLS path (type: `tokio_rustls::rustls::ServerConfig`) |
|    `pq` |     off | Enables PQ observability toggles (families always exist)     |
|    `zk` |     off | Enables ZK observability toggles                             |
|  `otel` |     off | Switch exporter to OpenTelemetry (public API unchanged)      |
|   `cli` |    bins | Enable CLI parsing where applicable                          |

---

## 8) Security Implications

* **Never** expose `/metrics` publicly; prefer **loopback** or **UDS** + SO_PEERCRED.
* If TLS is required, only use `tokio_rustls::rustls::ServerConfig`. Never `rustls::ServerConfig` directly.
* `labels.amnesia` **must** reflect kernel truth (Micronode defaults to `on`).
* Avoid high-cardinality labels in `labels.extra` (e.g., per-request IDs).
* Histogram edits alter dashboards; coordinate with ops.

---

## 9) Compatibility & Migration

* Add new keys with safe defaults; mark in CHANGELOG.
* Renames must ship aliases (env + file) for ≥1 minor and emit warnings.
* Breaking changes require **major** bump and migration notes.

**Deprecations (template):**

| Old Key | New Key | Removal Target | Notes |
| ------: | :------ | -------------: | ----- |
|       — | —       |              — | —     |

---

## 10) Reference Implementation (Rust)

> Minimal `Config` (serde) used by the exposer. Services may wrap/extend this.

```rust
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsCfg {
    pub metrics_addr: Option<SocketAddr>,   // Some => TCP; None => rely on uds_path
    pub uds_path: Option<PathBuf>,          // Some => UDS; mutually exclusive with metrics_addr
    #[serde(with = "humantime_serde", default = "d2s")] pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "d2s")] pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "d15s")] pub idle_timeout: Duration,
    #[serde(default = "m0700")] pub uds_dir_mode: String,  // "0700"
    #[serde(default = "m0600")] pub uds_sock_mode: String, // "0600"
}
fn d2s() -> Duration { Duration::from_secs(2) }
fn d15s() -> Duration { Duration::from_secs(15) }
fn m0700() -> String { "0700".into() }
fn m0600() -> String { "0600".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsCfg {
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Labels {
    pub service: Option<String>,
    pub instance: Option<String>,
    pub build_version: Option<String>,
    pub amnesia: Option<String>, // "on" | "off"
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histograms {
    #[serde(default = "default_req_buckets")]
    pub request_latency_seconds: Vec<f64>,
}
impl Default for Histograms {
    fn default() -> Self { Self { request_latency_seconds: default_req_buckets() } }
}
fn default_req_buckets() -> Vec<f64> {
    vec![0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PqCfg { pub mode: Option<String> } // "off" | "hybrid"

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZkCfg { pub enabled: bool }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OtelCfg { pub enabled: bool }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub metrics: MetricsCfg,
    pub tls: TlsCfg,
    pub labels: Labels,
    pub histograms: Histograms,
    pub pq: PqCfg,
    pub zk: ZkCfg,
    pub otel: OtelCfg,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        // Exclusivity
        if self.metrics.metrics_addr.is_some() && self.metrics.uds_path.is_some() {
            anyhow::bail!("metrics_addr and uds_path are mutually exclusive");
        }
        // TLS pairing
        if self.tls.enabled {
            match (&self.tls.cert_path, &self.tls.key_path) {
                (Some(c), Some(k)) if c.exists() && k.exists() => {},
                _ => anyhow::bail!("TLS enabled but cert/key missing or unreadable"),
            }
        }
        // Buckets monotonic
        let b = &self.histograms.request_latency_seconds;
        if b.is_empty() || b.windows(2).any(|w| w[0] >= w[1]) || b[0] <= 0.0 {
            anyhow::bail!("invalid request_latency_seconds buckets");
        }
        // Labels amnesia
        if let Some(a) = &self.labels.amnesia {
            if a != "on" && a != "off" { anyhow::bail!("labels.amnesia must be 'on' or 'off'"); }
        }
        // PQ/ZK consistency (warn if toggled without feature)
        #[cfg(not(feature = "pq"))]
        if self.pq.mode.as_deref() == Some("hybrid") {
            tracing::warn!("pq.mode=hybrid but built without feature `pq`; families remain zeroed");
        }
        #[cfg(not(feature = "zk"))]
        if self.zk.enabled {
            tracing::warn!("zk.enabled=true but built without feature `zk`; families remain zeroed");
        }
        // OTEL exporter toggle is disruptive; caller should rebind/swap exporter
        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                               | Expected Outcome                                             |
| -------------------------------------- | ------------------------------------------------------------ |
| `metrics_addr` & `uds_path` both set   | Fail fast (mutually exclusive)                               |
| Invalid TCP bind                       | Fail with clear error                                        |
| UDS path with wrong modes              | Corrected or fail (enforce `0700/0600`)                      |
| TLS enabled w/o keys                   | Fail fast                                                    |
| Missing labels resolved from defaults  | service/process name, instance/hostname, build/crate version |
| Buckets not increasing                 | Fail validation                                              |
| SIGHUP / ConfigUpdated                 | Non-disruptive keys apply atomically; disruptive keys rebind |
| `pq.mode=hybrid` without `pq` feature  | Start + **warn**; PQ families present but zero               |
| `zk.enabled=true` without `zk` feature | Start + **warn**; ZK families present but zero               |
| `--otel` enabled                       | Exporter swaps (disruptive); metrics families preserved      |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config File] --> D
  C[Env Vars RON_METRICS_*] --> D
  E[CLI Flags] --> D
  D --> V{Validate}
  V -- ok --> R[Runtime Snapshot (Arc<Config>)]
  V -- fail --> X[Exit / Error]
  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Prefer **UDS** or **127.0.0.1** exposure; scrape via sidecar/agent.
* Keep labels **stable and low-cardinality**; use `labels.extra` sparingly.
* Coordinate histogram changes with dashboard owners.
* Ensure `labels.amnesia` matches kernel truth (Micronode defaults to `on`).
* Bus-driven reload: services should emit/consume `ConfigUpdated { version }` to coordinate disruptive toggles.

```
```
