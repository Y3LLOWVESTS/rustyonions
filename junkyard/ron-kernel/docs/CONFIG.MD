

---

````markdown
---
title: Configuration — ron-kernel
crate: ron-kernel
owner: Stevan White
last-reviewed: 2025-09-24
status: draft
template_version: 1.0
---

# Configuration — ron-kernel

This document defines **all configuration** for `ron-kernel`: sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> - **Library crate (default):** kernel is lifecycle/supervision/config/bus/health; it **does not** run overlay/DHT/storage/transport loops. Observability endpoints live in `ron-metrics` or service crates; kernel **emits** canonical counters/events.  
> - **Demo bins (optional):** may bind localhost for `/metrics|/healthz|/readyz` via `ron-metrics`. Production services typically own network binds.

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (`Config.toml` adjacent to the binary or as passed via `--config`)  
4. **Built-in defaults** (hard-coded)

Reload recomputes the **effective config** under the same precedence ordering.  
Supported file formats: **TOML** (preferred), **JSON** (optional).  
Relative `--config` is resolved against `./` then `$CWD`.

> Kernel exposes a **hot-reload watcher** and emits `KernelEvent::ConfigUpdated{ version }` on success (services consume it); the **public API re-exports** include `Config` and `wait_for_ctrl_c()`.

---

## 2) Quickstart Examples

### 2.1 Minimal (library usage; no ports)
```bash
RUST_LOG=info \
RONK_AMNESIA=false \
cargo test -p ron-kernel
````

### 2.2 Demo bin with localhost observability (optional)

```bash
RUST_LOG=info \
RONK_OBS_ADDR=127.0.0.1:0 \
RONK_SUP_RESTART_CAP=5 \
cargo run -p ron-kernel --bin kernel_demo
```

> Kernel emits **golden metrics** (`bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`) and readiness events; production services expose HTTP endpoints.

### 2.3 Config file (TOML; safe defaults)

```toml
# Config.toml
[supervisor]
min_backoff   = "100ms"
max_backoff   = "30s"
restart_cap   = 5         # max restarts per window
restart_window= "60s"

[bus]
capacity = 4096           # bounded broadcast buffer per IDB

[amnesia]
enabled = false           # kernel-level flag; services must honor

[pq]
mode = "off"              # or "hybrid" (pass-through posture only)

[observability]
expose_http = false       # library default; demo bins may set true
addr        = "127.0.0.1:0"

[log]
format = "json"
level  = "info"
```

### 2.4 CLI flags (override file/env)

```bash
cargo run -p ron-kernel -- \
  --sup-restart-cap 5 \
  --bus-capacity 4096 \
  --amnesia on \
  --obs-http true --obs-addr 127.0.0.1:0
```

---

## 3) Schema (Typed, With Defaults)

> **Prefix convention:** All env vars start with `RONK_` (e.g., `RONK_AMNESIA`).
> **Durations** accept `ms`, `s`, `m`, `h`. **Sizes** are not typical here (kernel is not an HTTP ingress).
> **Note:** Ports/binds are **N/A for library**; observability bind is **optional** and typically handled by `ron-metrics`/services.

| Key / Env Var                                     | Type                   | Default       | Description                                                                   | Security Notes                        |                          |                         |
| ------------------------------------------------- | ---------------------- | ------------- | ----------------------------------------------------------------------------- | ------------------------------------- | ------------------------ | ----------------------- |
| `supervisor.min_backoff` / `RONK_SUP_MIN`         | duration               | `100ms`       | Crash-only restart backoff lower bound                                        | Prevents thundering herds             |                          |                         |
| `supervisor.max_backoff` / `RONK_SUP_MAX`         | duration               | `30s`         | Backoff cap                                                                   | See chaos tests                       |                          |                         |
| `supervisor.restart_cap` / `RONK_SUP_RESTART_CAP` | u32                    | `5`           | Max restarts per window before quarantine                                     | Emits `ServiceCrashed{reason}`        |                          |                         |
| `supervisor.restart_window` / `RONK_SUP_WINDOW`   | duration               | `60s`         | Window for `restart_cap`                                                      | Ops alerting hook                     |                          |                         |
| `bus.capacity` / `RONK_BUS_CAPACITY`              | u32                    | `4096`        | Broadcast buffer (bounded)                                                    | Overflow counted, never blocks kernel |                          |                         |
| `amnesia.enabled` / `RONK_AMNESIA`                | bool                   | `false`       | Global amnesia flag surfaced by kernel; services must honor                   | RAM-only caches, ephemeral logs       |                          |                         |
| `pq.mode` / `RONK_PQ_MODE`                        | enum(`off`,`hybrid`)   | `off`         | **Pass-through posture** (e.g., transport/proto PQ); kernel is crypto-neutral | Interop posture only                  |                          |                         |
| `observability.expose_http` / `RONK_OBS_HTTP`     | bool                   | `false`       | If `true`, demo exposes `/metrics                                             | /healthz                              | /readyz`via`ron-metrics` | Prefer localhost in dev |
| `observability.addr` / `RONK_OBS_ADDR`            | socket                 | `127.0.0.1:0` | Bind for demo observability if enabled                                        | Library default is off                |                          |                         |
| `log.format` / `RONK_LOG_FORMAT`                  | enum(`json`,`text`)    | `json`        | Log format                                                                    | JSON in prod                          |                          |                         |
| `log.level` / `RONK_LOG_LEVEL`                    | enum(`trace`..`error`) | `info`        | Log level                                                                     | Avoid `trace` in prod                 |                          |                         |

> **Canonical kernel metrics** are always emitted (counters/histograms) regardless of who serves them: `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`.

---

## 4) Validation Rules (Fail-Closed)

On startup or reload:

* `bus.capacity` **> 0**; assert boundedness (no “unbounded” sentinel).
* `supervisor.restart_cap` **≥ 1**; `min_backoff ≤ max_backoff`.
* `observability.addr` must parse to socket **iff** `expose_http=true`.
* `pq.mode ∈ {off, hybrid}`; kernel treats it as **posture passthrough**, not behavior (no crypto in kernel).
* **On violation:** structured error, **exit non-zero** (for bins) or **Err** (lib), per Hardening defaults.

---

## 5) Dynamic Reload (If Supported)

**Triggers**

* POSIX: `SIGHUP` (bins)
* Programmatic: `KernelEvent::ConfigUpdated { version }` published on successful swap

**Semantics**

* **Non-disruptive:** `bus.capacity` (applies to new subscribers), log level, `amnesia.enabled`, `pq.mode` (posture).
* **Quasi-disruptive:** `supervisor.*` takes effect for future restarts; does not preempt running children.
* **Atomicity:** compute a new `Arc<Config>` snapshot; swap without holding a lock across `.await` (IDB invariant).

**Audit**

* Log a redacted **diff** (no secrets), include new `version`, and emit metrics labels when `amnesia` changes.

---

## 6) CLI Flags (Canonical)

```
--config <path>                 # Load Config.toml
--bus-capacity <num>            # Bounded broadcast buffer
--sup-min-backoff <dur>         # 100ms, 250ms, etc.
--sup-max-backoff <dur>         # 30s, etc.
--sup-restart-cap <num>         # e.g., 5
--sup-restart-window <dur>      # e.g., 60s
--amnesia <on|off>              # kernel flag (services must honor)
--pq <off|hybrid>               # posture passthrough only
--obs-http <true|false>         # demo only
--obs-addr <ip:port>            # demo only
--log-format <json|text>
--log-level  <trace|debug|info|warn|error>
```

---

## 7) Feature Flags (Cargo)

| Feature   | Default | Effect                                              |          |          |
| --------- | ------: | --------------------------------------------------- | -------- | -------- |
| `cli`     |     on* | Enables CLI parsing for the flags above (bins only) |          |          |
| `metrics` |     off | Integrates `ron-metrics` helper for demo `/metrics  | /healthz | /readyz` |
| `kameo`   |     off | Optional actor integration (kept behind a flag)     |          |          |
| `pq`      |     off | Enables PQ posture surfacing (`pq.mode`)            |          |          |

> Keep features **orthogonal** and ensure public API re-exports remain unchanged.

---

## 8) Security Implications

* **Kernel minimalism:** No overlay/DHT/transport/storage/ledger loops in kernel; keep all IO/policy in services.
* **Amnesia:** When `amnesia.enabled=true`, expect RAM-only caches and ephemeral logs across the stack; kernel must label metrics `amnesia="on"` for auditability.
* **PQ posture:** Kernel stays crypto-neutral but must not block PQ adoption; flags are pass-through and visible in health snapshots.
* **Hardening inheritance:** Most global limits (5s timeouts/1MiB bodies/10× decompress) apply to **services**; kernel tracks rejects via counters but does not police payloads.
* **UDS/PEERCRED:** If a demo uses UDS for observability, honor `0700/0600` and **SO_PEERCRED** allow-list as per Hardening defaults.

---

## 9) Compatibility & Migration

* **Backwards-compatible keys**: add with safe defaults; never repurpose semantics silently (GOV).
* **Renames:** deprecate with env var alias for ≥1 minor; warn on use.
* **Breaking changes:** require **major version** and `CHANGELOG.md` with migration steps; CI public-API guard enforces freeze.

*Deprecation table (maintained):*

| Old Key | New Key | Removal Target | Notes                    |
| ------: | :------ | -------------: | :----------------------- |
| `<old>` | `<new>` |     v(A+1).0.0 | Provide conversion logic |

---

## 10) Reference Implementation (Rust)

> Minimal `Config` with file/env/CLI merge and strict validation.
> Copy into `src/config.rs`. Keep comments—reviewers use them for drift checks.

```rust
use std::{time::Duration, net::SocketAddr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorCfg {
    #[serde(with = "humantime_serde", default = "d_100ms")]
    pub min_backoff: Duration,
    #[serde(with = "humantime_serde", default = "d_30s")]
    pub max_backoff: Duration,
    #[serde(default = "d_restart_cap")]
    pub restart_cap: u32,
    #[serde(with = "humantime_serde", default = "d_60s")]
    pub restart_window: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusCfg {
    #[serde(default = "d_bus_capacity")]
    pub capacity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmnesiaCfg {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PqMode { Off, Hybrid }
impl Default for PqMode { fn default() -> Self { PqMode::Off } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqCfg {
    #[serde(default)]
    pub mode: PqMode, // posture only; crypto-neutral kernel
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityCfg {
    #[serde(default)]
    pub expose_http: bool, // demo bins only
    #[serde(default = "d_localhost0")]
    pub addr: SocketAddr,  // 127.0.0.1:0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCfg {
    #[serde(default = "d_log_format")]
    pub format: String,  // "json"|"text"
    #[serde(default = "d_log_level")]
    pub level: String,   // trace..error
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub supervisor: SupervisorCfg,
    #[serde(default)]
    pub bus: BusCfg,
    #[serde(default)]
    pub amnesia: AmnesiaCfg,
    #[serde(default)]
    pub pq: PqCfg,
    #[serde(default)]
    pub observability: ObservabilityCfg,
    #[serde(default)]
    pub log: LogCfg,
}

fn d_100ms() -> Duration { Duration::from_millis(100) }
fn d_30s()   -> Duration { Duration::from_secs(30) }
fn d_60s()   -> Duration { Duration::from_secs(60) }
fn d_restart_cap() -> u32 { 5 }
fn d_bus_capacity() -> u32 { 4096 }
fn d_localhost0() -> SocketAddr { "127.0.0.1:0".parse().unwrap() }
fn d_log_format() -> String { "json".into() }
fn d_log_level() -> String { "info".into() }

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::bail;
        if self.bus.capacity == 0 { bail!("bus.capacity must be > 0"); }
        if self.supervisor.restart_cap == 0 { bail!("supervisor.restart_cap must be >= 1"); }
        if self.supervisor.min_backoff > self.supervisor.max_backoff {
            bail!("min_backoff must be <= max_backoff");
        }
        if self.observability.expose_http && (self.observability.addr.ip().is_unspecified()) {
            bail!("observability.addr must be a concrete address when expose_http=true");
        }
        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                               | Expected Outcome                                         |
| -------------------------------------- | -------------------------------------------------------- |
| Missing/empty `Config.toml`            | Start with safe defaults; log that defaults were applied |
| `bus.capacity = 0`                     | Fail fast with explicit error; no unbounded queues       |
| `min_backoff > max_backoff`            | Fail fast on validation                                  |
| Restart storm (> `restart_cap`/window) | Quarantine child; increment `service_restarts_total`     |
| Toggle `amnesia.enabled` on reload     | Snapshot flips; metrics include `amnesia="on"` label     |
| `pq.mode = hybrid`                     | Exposed in health; no kernel crypto behavior change      |
| `expose_http=true` with bad addr       | Fail validation; demo refuses to bind                    |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config File] --> D
  C[Env Vars (RONK_*)] --> D
  E[CLI Flags] --> D
  D --> V{Validate}
  V -- ok --> S[Arc<Config> Snapshot]
  V -- fail --> X[Exit/Error]
  S --> K[Emit KernelEvent::ConfigUpdated{version}]
  style S fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Kernel is **tiny by design**—it orchestrates, it **does not** police protocol payloads; services enforce OAP limits and surface rejects which the kernel **observes** via counters.
* Watch **bus lag** and **restart storms**; `/readyz` must degrade early by design under sustained pressure.
* Hardening defaults (timeouts/body caps/decompress guard) live in **services**; keep the kernel focused on **boundedness + observability**.

```

