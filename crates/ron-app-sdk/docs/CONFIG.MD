
````markdown
---
title: Configuration — ron-app-sdk
crate: ron-app-sdk
owner: Stevan White
last-reviewed: 2025-10-16
status: draft
template_version: 1.0
---

# Configuration — ron-app-sdk

This document defines **all configuration** for `ron-app-sdk`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> - **Library crate:** this document focuses on client-side knobs: transport, timeouts, retries, idempotency, caches, tracing, PQ.  
> - **Service-specific fields** (e.g., bind/metrics) are **N/A** for this crate.

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (only if using the optional `cli` feature in an example/bin)
2. **Environment variables**
3. **Config file** (e.g., `Config.toml` in app repo or next to the binary)
4. **Built-in defaults** (hard-coded)

> On live reload, the **effective** config is recomputed using the same precedence.

**Supported file formats:** TOML (preferred), JSON (optional).  
**Path resolution for `--config` (if relative):** `./`, then `$CWD`.

---

## 2) Quickstart Examples

### 2.1 Minimal (env + builder)
```bash
RON_SDK_TRANSPORT=tls
RON_SDK_GATEWAY_ADDR=https://localhost:8443
RON_SDK_OVERALL_TIMEOUT_MS=5000
RON_SDK_RETRY_BASE_MS=100
RON_SDK_RETRY_MAX_ATTEMPTS=5
RON_SDK_IDEM_ENABLED=true
````

```rust
use ron_app_sdk::{RonAppSdk, SdkConfig};
use std::time::Duration;

let cfg = SdkConfig::from_env()?.with_overrides(|c| {
    c.overall_timeout = Duration::from_millis(5000);
});
let sdk = RonAppSdk::new(cfg).await?;
```

### 2.2 Config file (TOML)

```toml
# Config.toml — client-side SDK config
transport        = "tls"          # tls | tor
gateway_addr     = "https://localhost:8443"   # base URL or .onion (when transport=tor)
overall_timeout  = "5s"

[timeouts]
connect = "2s"
read    = "5s"
write   = "5s"

[retry]
base_ms      = 100
factor       = 2.0
cap_ms       = 10000
max_attempts = 5
jitter       = "full"            # full | none

[idempotency]
enabled    = true
key_prefix = "app1"

[cache]
enabled      = false
max_entries  = 256
ttl          = "30s"
verify_puts  = false

[tracing]
propagate = true
redaction = "safe"               # safe | none

[pq]
mode = "off"                     # off | hybrid
```

### 2.3 CLI flags (if the app opts into `cli`)

```bash
myapp --config ./Config.toml \
  --gateway https://localhost:8443 \
  --transport tls \
  --overall-timeout 5s \
  --retry-base 100ms --retry-max 5
```

---

## 3) Schema (Typed, With Defaults)

> **Prefix convention:** All env vars begin with `RON_SDK_`.
> **Durations** accept `ms`, `s`, `m`, `h`.
> **Sizes** accept `B`, `KiB`, `MiB` (only used for cache sizing if extended).

| Key / Env Var                                       | Type                 | Default          | Description                                         | Security Notes                                     |
| --------------------------------------------------- | -------------------- | ---------------- | --------------------------------------------------- | -------------------------------------------------- |
| `transport` / `RON_SDK_TRANSPORT`                   | enum(`tls`,`tor`)    | `tls`            | Client transport backend                            | `tor` routes via local SOCKS (arti/tor); see below |
| `gateway_addr` / `RON_SDK_GATEWAY_ADDR`             | URL/string           | `""`             | Base endpoint (https://… or .onion)                 | Validate scheme; prefer HTTPS                      |
| `overall_timeout` / `RON_SDK_OVERALL_TIMEOUT_MS`    | duration             | `5s`             | Outer per-call deadline                             | Prevents hangs                                     |
| `timeouts.connect` / `RON_SDK_CONNECT_TIMEOUT_MS`   | duration             | `2s`             | Dial/connect timeout                                | DoS/backoff synergy                                |
| `timeouts.read` / `RON_SDK_READ_TIMEOUT_MS`         | duration             | `5s`             | Read timeout                                        | DoS mitigation                                     |
| `timeouts.write` / `RON_SDK_WRITE_TIMEOUT_MS`       | duration             | `5s`             | Write timeout                                       | DoS mitigation                                     |
| `retry.base_ms` / `RON_SDK_RETRY_BASE_MS`           | u64 (ms)             | `100`            | Full-jitter backoff base (I-5)                      | N/A                                                |
| `retry.factor` / `RON_SDK_RETRY_FACTOR`             | f32                  | `2.0`            | Backoff multiplier                                  | N/A                                                |
| `retry.cap_ms` / `RON_SDK_RETRY_CAP_MS`             | u64 (ms)             | `10000`          | Max backoff                                         | N/A                                                |
| `retry.max_attempts` / `RON_SDK_RETRY_MAX_ATTEMPTS` | u32                  | `5`              | Max tries (including first)                         | N/A                                                |
| `retry.jitter` / `RON_SDK_RETRY_JITTER`             | enum(`full`,`none`)  | `full`           | Jitter strategy                                     | Use `full` in prod                                 |
| `idempotency.enabled` / `RON_SDK_IDEM_ENABLED`      | bool                 | `true`           | Attach idempotency key to mutating calls            | Prevent dup effects                                |
| `idempotency.key_prefix` / `RON_SDK_IDEM_PREFIX`    | string/optional      | `""`             | Optional prefix to help dedupe by app/tenant        | Avoid PII                                          |
| `cache.enabled` / `RON_SDK_CACHE_ENABLED`           | bool                 | `false`          | Enable ephemeral LRU cache                          | Never bypass auth                                  |
| `cache.max_entries` / `RON_SDK_CACHE_MAX_ENTRIES`   | usize                | `256`            | Cache bound                                         | N/A                                                |
| `cache.ttl` / `RON_SDK_CACHE_TTL_MS`                | duration             | `30s`            | Entry TTL                                           | N/A                                                |
| `cache.verify_puts` / `RON_SDK_CACHE_VERIFY_PUTS`   | bool                 | `false`          | Verify BLAKE3 on terminal PUT success               | CPU trade-off                                      |
| `tracing.propagate` / `RON_SDK_TRACING_PROPAGATE`   | bool                 | `true`           | Forward correlation headers (B3/W3C when available) | Don’t leak secrets                                 |
| `tracing.redaction` / `RON_SDK_TRACING_REDACTION`   | enum(`safe`,`none`)  | `safe`           | Redact payload/meta in spans                        | Prefer `safe`                                      |
| `pq.mode` / `RON_SDK_PQ_MODE`                       | enum(`off`,`hybrid`) | `off`            | PQ hybrid toggle (e.g., X25519+Kyber)               | Interop risk; gate rollout                         |
| `tor.socks_addr` / `RON_SDK_TOR_SOCKS_ADDR`         | socket/string        | `127.0.0.1:9150` | Local Tor/arti SOCKS address (when `transport=tor`) | Local-only                                         |
| `msrv` (implicit)                                   | rustc version        | `1.80.0`         | Minimum supported Rust (build-time)                 | Bumps are semver-minor                             |

---

## 4) Validation Rules (Fail-Closed)

On init or reload:

* `gateway_addr` must be non-empty and parseable. When `transport="tls"`, scheme must be `https`. When `transport="tor"`, address must be a valid `.onion` host.
* `overall_timeout` ≥ `1s`. `max_attempts` ≥ `1`. `factor` ≥ `1.0`. `cap_ms` ≥ `base_ms`.
* When `transport="tor"`: `tor.socks_addr` must parse and be reachable (best-effort probe allowed).
* If `cache.enabled=true`: `max_entries` ≥ `1`, `ttl` ≥ `1s`.
* `tracing.redaction="safe"` in production builds (warn if `none`).
* If `pq.mode="hybrid"`: require caller opt-in (feature flag) and show an interop warning (documented).

**On violation:** return an error (library); **do not** fall back silently.

---

## 5) Dynamic Reload (If Supported by the Host App)

* **Trigger:** host app calls `sdk.update_config(new_cfg)` or uses a watcher (e.g., SIGHUP in host that rebuilds `SdkConfig`).
* **Reload semantics:** non-disruptive for timeouts/retries/tracing/cache. **Disruptive** for transport layer changes (`tls↔tor`, gateway base) which reinitialize connectors.
* **Atomicity:** compute a new `SdkConfig`, validate, then swap an `Arc<SdkConfig>` without holding `.await` across the swap.
* **Audit:** emit a tracing event `sdk.config.updated` with a **redacted** diff (no secrets).

---

## 6) CLI Flags (Canonical, optional via `cli` feature)

```
--config <path>                    # Load Config.(toml|json)
--gateway <url>                    # Override gateway_addr
--transport <tls|tor>
--overall-timeout <dur>            # e.g., 5s
--connect-timeout <dur>
--read-timeout <dur>
--write-timeout <dur>
--retry-base <dur>
--retry-factor <float>
--retry-cap <dur>
--retry-max <u32>
--idem-enabled <bool>
--idem-prefix <str>
--cache-enabled <bool>
--cache-max-entries <usize>
--cache-ttl <dur>
--tracing-propagate <bool>
--tracing-redaction <safe|none>
--pq-mode <off|hybrid>
--tor-socks <addr:port>
```

---

## 7) Feature Flags (Cargo)

|   Feature | Default | Effect                                              |
| --------: | :-----: | --------------------------------------------------- |
|     `tls` |    on   | Use TLS transport (native roots pre-configured)     |
|     `tor` |   off   | Route via local SOCKS (arti/tor)                    |
| `metrics` |   off   | Emit Prometheus counters/histograms (host collects) |
|     `cli` |   off   | Enable CLI parsing for host/example binaries        |
|      `pq` |   off   | Enable PQ hybrid mode toggles                       |

> Changes in features may gate which keys are accepted; unknown keys **warn** but do not crash unless `strict` mode is enabled by the host.

---

## 8) Security Implications

* **Capabilities** must never be logged; redact headers/bodies when `tracing.redaction="safe"`.
* **Tor**: ensure SOCKS is local-only; never auto-enable Tor without explicit choice.
* **Deadlines** are mandatory; library refuses unbounded I/O.
* **Caches** are ephemeral; they **never** bypass capability checks nor alter authorization.
* **PQ hybrid** can fragment interop; roll out behind explicit flags and compat tests.

---

## 9) Compatibility & Migration

* **Additive-first:** introduce new keys with safe defaults.
* **Renames:** keep env-var aliases for ≥1 minor; emit a deprecation warning.
* **Breaking changes:** require a **major** version bump and a migration note in `CHANGELOG.md`.

*Deprecation table (to be maintained):*

| Old Key              | New Key                      | Removal Target | Notes              |
| -------------------- | ---------------------------- | -------------- | ------------------ |
| `RON_SDK_TIMEOUT_MS` | `RON_SDK_OVERALL_TIMEOUT_MS` | v2.0.0         | Keep alias 1 minor |

---

## 10) Reference Implementation (Rust)

> Paste into `src/config/mod.rs`. Uses `serde` + `humantime_serde` for durations and an `Env` loader. Keep comments.

```rust
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport { Tls, Tor }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Jitter { Full, None }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Redaction { Safe, None }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PqMode { Off, Hybrid }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeouts {
    #[serde(with = "humantime_serde", default = "default_connect")]
    pub connect: Duration,
    #[serde(with = "humantime_serde", default = "default_io")]
    pub read: Duration,
    #[serde(with = "humantime_serde", default = "default_io")]
    pub write: Duration,
}
fn default_connect() -> Duration { Duration::from_secs(2) }
fn default_io() -> Duration { Duration::from_secs(5) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryCfg {
    #[serde(with = "humantime_serde", default = "default_retry_base")]
    pub base: Duration,
    #[serde(default = "default_retry_factor")]
    pub factor: f32,
    #[serde(with = "humantime_serde", default = "default_retry_cap")]
    pub cap: Duration,
    #[serde(default = "default_retry_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_jitter")]
    pub jitter: Jitter,
}
fn default_retry_base() -> Duration { Duration::from_millis(100) }
fn default_retry_factor() -> f32 { 2.0 }
fn default_retry_cap() -> Duration { Duration::from_secs(10) }
fn default_retry_max_attempts() -> u32 { 5 }
fn default_jitter() -> Jitter { Jitter::Full }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdemCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub key_prefix: Option<String>,
}
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cache_entries")]
    pub max_entries: usize,
    #[serde(with = "humantime_serde", default = "default_cache_ttl")]
    pub ttl: Duration,
    #[serde(default)]
    pub verify_puts: bool,
}
fn default_cache_entries() -> usize { 256 }
fn default_cache_ttl() -> Duration { Duration::from_secs(30) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingCfg {
    #[serde(default = "default_true")]
    pub propagate: bool,
    #[serde(default = "default_redaction")]
    pub redaction: Redaction,
}
fn default_redaction() -> Redaction { Redaction::Safe }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorCfg {
    #[serde(default = "default_tor_socks")]
    pub socks_addr: String, // "127.0.0.1:9150"
}
fn default_tor_socks() -> String { "127.0.0.1:9150".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkConfig {
    #[serde(default = "default_transport")]
    pub transport: Transport,
    #[serde(default)]
    pub gateway_addr: String,
    #[serde(with = "humantime_serde", default = "default_overall")]
    pub overall_timeout: Duration,
    #[serde(default)]
    pub timeouts: Timeouts,
    #[serde(default)]
    pub retry: RetryCfg,
    #[serde(default)]
    pub idempotency: IdemCfg,
    #[serde(default)]
    pub cache: CacheCfg,
    #[serde(default)]
    pub tracing: TracingCfg,
    #[serde(default)]
    pub pq: PqMode,
    #[serde(default)]
    pub tor: TorCfg,
    // allow extension fields as needed
}
fn default_transport() -> Transport { Transport::Tls }
fn default_overall() -> Duration { Duration::from_secs(5) }

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            transport: default_transport(),
            gateway_addr: String::new(),
            overall_timeout: default_overall(),
            timeouts: Timeouts { connect: default_connect(), read: default_io(), write: default_io() },
            retry: RetryCfg { base: default_retry_base(), factor: default_retry_factor(), cap: default_retry_cap(), max_attempts: default_retry_max_attempts(), jitter: default_jitter() },
            idempotency: IdemCfg { enabled: true, key_prefix: None },
            cache: CacheCfg { enabled: false, max_entries: default_cache_entries(), ttl: default_cache_ttl(), verify_puts: false },
            tracing: TracingCfg { propagate: true, redaction: default_redaction() },
            pq: PqMode::Off,
            tor: TorCfg { socks_addr: default_tor_socks() },
        }
    }
}

impl SdkConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.gateway_addr.trim().is_empty() {
            anyhow::bail!("gateway_addr must be set");
        }
        if matches!(self.transport, Transport::Tls) && !self.gateway_addr.starts_with("https://") {
            anyhow::bail!("transport=tls requires an https:// gateway_addr");
        }
        if matches!(self.transport, Transport::Tor) && !self.gateway_addr.contains(".onion") {
            anyhow::bail!("transport=tor requires a .onion gateway_addr");
        }
        if self.retry.max_attempts == 0 { anyhow::bail!("retry.max_attempts must be >= 1"); }
        if self.retry.factor < 1.0 { anyhow::bail!("retry.factor must be >= 1.0"); }
        if self.retry.cap < self.retry.base { anyhow::bail!("retry.cap must be >= retry.base"); }
        if self.cache.enabled {
            if self.cache.max_entries == 0 { anyhow::bail!("cache.max_entries must be >= 1"); }
            if self.cache.ttl < Duration::from_secs(1) { anyhow::bail!("cache.ttl must be >= 1s"); }
        }
        Ok(())
    }

    pub fn from_env() -> anyhow::Result<Self> {
        // Lightweight env loader (explicit to avoid extra deps).
        let mut cfg = SdkConfig::default();
        if let Ok(s) = std::env::var("RON_SDK_TRANSPORT") {
            cfg.transport = match s.to_ascii_lowercase().as_str() {
                "tls" => Transport::Tls,
                "tor" => Transport::Tor,
                other => anyhow::bail!("invalid RON_SDK_TRANSPORT: {other}"),
            };
        }
        if let Ok(s) = std::env::var("RON_SDK_GATEWAY_ADDR") { cfg.gateway_addr = s; }
        if let Ok(ms) = std::env::var("RON_SDK_OVERALL_TIMEOUT_MS").ok().and_then(|x| x.parse().ok()) {
            cfg.overall_timeout = Duration::from_millis(ms);
        }
        // (Omitted: parse remaining envs similarly or switch to figment/envy if preferred)
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn with_overrides<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }
}
```

---

## 11) Test Matrix

| Scenario                            | Expected Outcome                                    |
| ----------------------------------- | --------------------------------------------------- |
| Empty `gateway_addr`                | Error (fail-fast)                                   |
| `transport=tls` with http:// URL    | Error (require `https://`)                          |
| `transport=tor` with non-onion host | Error                                               |
| `retry.max_attempts=0`              | Error                                               |
| `cache.enabled=true` with ttl < 1s  | Error                                               |
| Switch TLS→Tor at runtime           | Reinitialize transport (disruptive), no panic       |
| Large retries under faults          | Backoff with full jitter; respect `overall_timeout` |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config File] --> D
  C[Env Vars] --> D
  E[CLI Flags (optional)] --> D
  D --> V{Validate}
  V -- ok --> R[Runtime Snapshot (Arc<SdkConfig>)]
  V -- fail --> X[Return Error]
  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Keep application-level config in version control; **never** commit secrets (capabilities).
* Prefer environment variables in containerized deployments; mount config files read-only.
* For Tor, run a **local** SOCKS (arti/tor) and restrict binds to localhost.
* Treat PQ hybrid as a **coordinated rollout** across client and node; guard with compatibility tests.

```
