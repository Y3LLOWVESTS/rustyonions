Awesome—here are the paste-ready files with all fixes applied (schema/feature/deprecation/test tables formatted, alias + WARN for jitter, added validations for deadline range, max_msg_bytes upper bound, and rapid_fail_window ≥ 1s, plus a tiny test block). Drop these in as-is.

---

**File:** `crates/ryker/docs/CONFIG.md`

````markdown
---
title: Configuration — ryker
crate: ryker
owner: Stevan White
last-reviewed: 2025-09-28
status: reviewed
template_version: 1.0
---

# Configuration — ryker

This document defines **all configuration** for `ryker` (actor & mailbox runtime), including
sources, precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> **Library crate** — Ryker does **not** bind sockets or expose `/healthz`. Config governs
> actor/mailbox behavior, supervision backoff, deadlines, fairness, and observability hooks.

---

## 1) Sources & Precedence (Authoritative)

Ryker is embedded into host services. **Highest precedence wins:**

1. **Builder/explicit API** (e.g., `RykerConfig::builder()...build()`)
2. **Environment variables** (prefix: `RYKER_`)
3. **Config file** (TOML/JSON) — *host loads and passes snapshot to ryker*
4. **Built-in defaults** (hard-coded)

> On reload, the effective config snapshot is recomputed under the **same precedence** and
> atomically swapped where supported (see §5).

**Supported file formats:** TOML (preferred), JSON (optional).  
**Optional dev convenience:** `RYKER_CONFIG_PATH` may point to a TOML/JSON file; it is intended for **developer workflows only**. In production, the **host** remains responsible for file loading and merging.

**Production guard:**  
- Canonical: `APP_ENV=production` means “prod.”  
- Alias: `RUST_ENV=production` is also treated as prod.  
- In prod, `RYKER_CONFIG_PATH` is **rejected** unless `RYKER_ALLOW_CONFIG_PATH=1`.

**Path behavior:** `RYKER_CONFIG_PATH` is interpreted **as given**. If a relative path is used, it’s resolved from the current working directory (`$CWD`).

---

## 2) Quickstart Examples

### 2.1 Minimal embedding (Rust)
```rust
use ryker::config::RykerConfig;

let cfg = RykerConfig::from_env_validated()?
    .with_overrides(|c| {
        c.defaults.mailbox_capacity = 512;
        c.defaults.max_msg_bytes = 128 * 1024; // 128 KiB
        c.supervisor.backoff_base_ms = 100;
        c.supervisor.backoff_cap_ms  = 5_000;
    });

let ryker = ryker::Runtime::new(cfg.clone());

// Per-actor overrides at construction time:
let inbox = ryker.mailbox::<MyMsg>("worker")
    .capacity(256)
    .max_msg_bytes(64 * 1024)
    .deadline_ms(1000)
    .build();
# Ok::<(), anyhow::Error>(())
````

### 2.2 Config file (TOML) loaded by host and passed in

```toml
# ryker.toml
[defaults]
mailbox_capacity = 256
max_msg_bytes    = "64KiB"
deadline         = "1s"

[supervisor]
backoff_base_ms       = 100
backoff_cap_ms        = 5000
rapid_fail_threshold  = 5
rapid_fail_window     = "10s"
decorrelated_jitter   = true

[fairness]
batch_messages        = 32
yield_every_n_msgs    = 64

[observability]
enable_metrics        = true
queue_depth_sampling  = true
span_level            = "info"

[amnesia]
enabled               = false
zeroize_on_drop       = true
```

### 2.3 Env vars (prefix `RYKER_`)

```bash
RYKER_DEFAULT_MAILBOX_CAPACITY=256 \
RYKER_DEFAULT_MAX_MSG_BYTES=65536 \
RYKER_DEFAULT_DEADLINE=1s \
RYKER_BACKOFF_BASE_MS=100 \
RYKER_BACKOFF_CAP_MS=5000 \
RYKER_RAPID_FAIL_THRESHOLD=5 \
RYKER_RAPID_FAIL_WINDOW=10s \
RYKER_DECORRELATED_JITTER=true \
RYKER_BATCH_MESSAGES=32 \
RYKER_YIELD_EVERY_N=64 \
RYKER_ENABLE_METRICS=true \
RYKER_QUEUE_DEPTH_SAMPLING=true \
RYKER_SPAN_LEVEL=info \
RYKER_AMNESIA=true \
RYKER_ZEROIZE=true \
cargo run -p your_host_service
```

---

## 3) Schema (Typed, With Defaults)

*Prefix:* all env vars start with `RYKER_`.
*Durations:* accept `ms`, `s`, `m`.
*Sizes:* accept `B`, `KiB`, `MiB` (case-insensitive: `kib/mib` also accepted).

| Key / Env                                                                | Type                                    | Default | Description                                                           | Security Notes        |
| ------------------------------------------------------------------------ | --------------------------------------- | ------- | --------------------------------------------------------------------- | --------------------- |
| `defaults.mailbox_capacity` / `DEFAULT_MAILBOX_CAPACITY`                 | `u32`                                   | `256`   | Global default capacity for new mailboxes unless overridden per actor | Bounds memory use     |
| `defaults.max_msg_bytes` / `DEFAULT_MAX_MSG_BYTES`                       | `size` (`B/KiB/MiB`)                    | `64KiB` | Per-message size cap                                                  | Prevents OOM          |
| `defaults.deadline` / `DEFAULT_DEADLINE`                                 | `dur` (`ms/s/m`)                        | `1s`    | Default req/resp deadline (overridable per call)                      | Latency guard         |
| `supervisor.backoff_base_ms` / `BACKOFF_BASE_MS`                         | `u32`                                   | `100`   | Restart backoff base (ms), decorrelated jitter                        | —                     |
| `supervisor.backoff_cap_ms` / `BACKOFF_CAP_MS`                           | `u32`                                   | `5000`  | Restart backoff cap (ms)                                              | —                     |
| `supervisor.decorrelated_jitter` / `DECORRELATED_JITTER` (old: `JITTER`) | `bool`                                  | `true`  | Enable decorrelated jitter                                            | —                     |
| `supervisor.rapid_fail_threshold` / `RAPID_FAIL_THRESHOLD`               | `u32`                                   | `5`     | Max restarts within window before giving up                           | —                     |
| `supervisor.rapid_fail_window` / `RAPID_FAIL_WINDOW`                     | `dur`                                   | `10s`   | Rolling window for rapid-fail accounting                              | —                     |
| `fairness.batch_messages` / `BATCH_MESSAGES`                             | `u32`                                   | `32`    | Max messages handled per batch/poll                                   | Starvation guard      |
| `fairness.yield_every_n_msgs` / `YIELD_EVERY_N`                          | `u32`                                   | `64`    | Force cooperative yield after N messages                              | Scheduler-friendly    |
| `observability.enable_metrics` / `ENABLE_METRICS`                        | `bool`                                  | `true`  | Emit metrics via caller’s registry                                    | No PII in labels      |
| `observability.queue_depth_sampling` / `QUEUE_DEPTH_SAMPLING`            | `bool`                                  | `true`  | Sample queue depth for scrapes                                        | —                     |
| `observability.span_level` / `SPAN_LEVEL`                                | `enum(off,error,warn,info,debug,trace)` | `info`  | Baseline span verbosity for actors/messages                           | Avoid `trace` in prod |
| `amnesia.enabled` / `AMNESIA`                                            | `bool`                                  | `false` | Honor amnesia mode (no hidden persistence; ephemeral state)           | Zeroize on drop       |
| `amnesia.zeroize_on_drop` / `ZEROIZE`                                    | `bool`                                  | `true`  | Zeroize transient buffers on drop where feasible                      | —                     |

> **Per-actor overrides** at construction time always beat global defaults.

---

## 4) Validation Rules (Fail-Closed)

On construction or reload, Ryker enforces:

* `defaults.mailbox_capacity ≥ 1`
* `defaults.max_msg_bytes ∈ [1KiB, 1MiB]` *(library safety guard)*
* `defaults.deadline ∈ [10ms, 60s]`
* `backoff_base_ms ≤ backoff_cap_ms`, both `> 0`
* `rapid_fail_threshold ≥ 1`; `rapid_fail_window ≥ 1s`
* `yield_every_n_msgs ≥ batch_messages`
* If `amnesia.enabled = true` → `zeroize_on_drop` **must** be `true`

On violation: construct-time error (library); log structured error at **ERROR** with offending key.

---

## 5) Dynamic Reload (Library Semantics)

**Trigger options (host decides):**

* Explicit call: `ryker_runtime.reload(new_cfg)`
* Bus event: host translates `KernelEvent::ConfigUpdated{version}` → `reload(...)`

**Reloadability by field:**

* **Hot-reloadable:** deadline default, fairness knobs, observability toggles.
* **Cold-only (new actors only):** mailbox_capacity, max_msg_bytes (existing mailboxes keep bounds).
* **Supervisor:** backoff/jitter/rapid-fail parameters affect future restarts (not already-scheduled delays).

**Atomicity:**
Compute a new immutable `RykerConfig` snapshot; swap under a mutex without holding `.await` (IDB [I-3]).

**Audit trail:**
Log diff (old→new) without exposing message contents; include version/source (env/file/builder).

---

## 6) CLI Flags (Canonical)

Ryker is a library. No canonical CLI.

Optional dev helper (feature `dev-cli`) may expose:

```
--ryker-config <path>   # Load ryker.toml, print resolved/validated snapshot for debugging
```

---

## 7) Feature Flags (Cargo)

| Feature   | Default | Effect                                                                                 |
| --------- | ------- | -------------------------------------------------------------------------------------- |
| `metrics` | on      | Enables metric counters/gauges (no HTTP server; uses caller’s registry)                |
| `tracing` | on      | Enables tracing spans for actors/messages                                              |
| `amnesia` | on      | Enables zeroize hooks on drop and amnesia wiring                                       |
| `pq`      | off     | No crypto in Ryker; placeholder for API compatibility toggles (keys live in `ron-kms`) |
| `dev-cli` | off     | Builds a tiny debug binary to dump a resolved config snapshot                          |
| `loom`    | off     | Enables loom smoke tests (dev-only)                                                    |

*Feature flags must not change public API semantics unexpectedly; document any schema deltas.*

---

## 8) Security Implications

* Ryker stores **no secrets**; do not log message contents or sensitive payloads.
* **Amnesia mode:** ephemeral by design; ensure `zeroize_on_drop=true` when `amnesia.enabled=true`.
* **Metrics:** labels may include actor name but never user data or PII.
* **Env is untrusted input:** strict validation + fail-closed.
* **Quantum note:** config parsing/validation performs no expensive crypto and avoids data-dependent timing branches; no timing/side-channel exposure via config paths.

See `docs/SECURITY.md` for full threat model.

---

## 9) Compatibility & Migration

**BC policy:** add keys with safe defaults; deprecate via aliases for ≥1 minor release.
**Renames:** keep env alias; log `WARN` when old key used.
**Breaking:** major version bump + `CHANGELOG.md` with explicit migration steps.

**Deprecation table (maintained):**

| Old Key        | New Key                     | Removal Target | Notes                                                 |
| -------------- | --------------------------- | -------------- | ----------------------------------------------------- |
| `RYKER_JITTER` | `RYKER_DECORRELATED_JITTER` | `v2.0.0`       | Alias accepted for one minor; `WARN` when old is used |

---

## 10) Reference Implementation (Rust)

Minimal, copy-paste-ready `src/config/mod.rs`. Uses `serde`, `humantime_serde`, `thiserror`.
Supports optional `RYKER_CONFIG_PATH` (dev) with prod guard, DRY env parsing, strict `from_env_validated()`.

*(See file in repo; kept in sync with this document.)*

---

## 11) Test Matrix

| Scenario                            | Expected Outcome                                                                                     |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Env overrides only                  | `from_env_validated()` yields snapshot with overrides; `validate()` passes                           |
| Invalid `mailbox_capacity = 0`      | `validate()` fails with explicit message                                                             |
| `max_msg_bytes < 1KiB`              | `validate()` fails                                                                                   |
| `max_msg_bytes > 1MiB`              | `validate()` fails                                                                                   |
| `deadline < 10ms` or `> 60s`        | `validate()` fails                                                                                   |
| `backoff_base_ms > backoff_cap_ms`  | `validate()` fails                                                                                   |
| `yield_every_n < batch_messages`    | `validate()` fails                                                                                   |
| `amnesia = true`, `zeroize = false` | `validate()` fails                                                                                   |
| Jitter env alias (old vs new)       | If only old set → honored with `WARN`; if both set → **new wins** and `WARN` about old being ignored |
| Reload fairness/deadline tweaks     | Existing actors keep bounds; new actors adopt; fairness applied immediately                          |
| Stress to overflow                  | Drops occur with reason `capacity` and counters increment                                            |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[File (TOML/JSON via host or RYKER_CONFIG_PATH dev)] --> D
  C[Env Vars (RYKER_*)] --> D
  E[Builder/API] --> D
  D --> V{Validate}
  V -- ok --> R[Snapshot -> Runtime]
  V -- fail --> X[Error]
  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* **Overflow incident:** inspect `ryker_mailbox_overflow_total{actor,reason}`.

  * If `reason="capacity"` spikes → reduce producer rate or boundedly raise `mailbox_capacity`; re-run load tests.
  * If `reason="deadline"` dominates → ensure callers set realistic deadlines; consider reducing `batch_messages` or `yield_every_n_msgs` for fairness.
* **Rapid restarts:** check `ryker_actor_restarts_total{actor}` vs. `rapid_fail_threshold/window`; widen window or investigate actor crash causes.
* Co-locate Ryker config with the host’s config; apply identical secret-handling rules (Ryker itself stores no secrets).
* Prefer builder/API for per-actor overrides; use env/file for sane global defaults.
* Turn on `queue_depth_sampling` in load tests; keep labels clean (no user data).
* When in doubt, prefer tighter deadlines + backpressure over inflating capacities.

````

---

**File:** `crates/ryker/src/config/mod.rs`
```rust
//! ryker config — see docs/CONFIG.md for the authoritative spec.

use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error("parse error for {key}: {msg}")]
    Parse { key: &'static str, msg: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub mailbox_capacity: u32, // per-actor default capacity
    pub max_msg_bytes: u64,    // bytes
    #[serde(with = "humantime_serde")]
    pub deadline: Duration,    // default req/resp deadline
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorCfg {
    pub backoff_base_ms: u32,
    pub backoff_cap_ms: u32,
    pub decorrelated_jitter: bool,
    pub rapid_fail_threshold: u32,
    #[serde(with = "humantime_serde")]
    pub rapid_fail_window: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fairness {
    pub batch_messages: u32,
    pub yield_every_n_msgs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observability {
    pub enable_metrics: bool,
    pub queue_depth_sampling: bool,
    pub span_level: SpanLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpanLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl FromStr for SpanLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(SpanLevel::Off),
            "error" => Ok(SpanLevel::Error),
            "warn" => Ok(SpanLevel::Warn),
            "info" => Ok(SpanLevel::Info),
            "debug" => Ok(SpanLevel::Debug),
            "trace" => Ok(SpanLevel::Trace),
            other => Err(format!("invalid span level: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amnesia {
    pub enabled: bool,
    pub zeroize_on_drop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RykerConfig {
    pub defaults: Defaults,
    pub supervisor: SupervisorCfg,
    pub fairness: Fairness,
    pub observability: Observability,
    pub amnesia: Amnesia,
}

impl Default for RykerConfig {
    fn default() -> Self {
        Self {
            defaults: Defaults {
                mailbox_capacity: 256,
                max_msg_bytes: 64 * 1024,
                deadline: Duration::from_secs(1),
            },
            supervisor: SupervisorCfg {
                backoff_base_ms: 100,
                backoff_cap_ms: 5_000,
                decorrelated_jitter: true,
                rapid_fail_threshold: 5,
                rapid_fail_window: Duration::from_secs(10),
            },
            fairness: Fairness {
                batch_messages: 32,
                yield_every_n_msgs: 64,
            },
            observability: Observability {
                enable_metrics: true,
                queue_depth_sampling: true,
                span_level: SpanLevel::Info,
            },
            amnesia: Amnesia {
                enabled: false,
                zeroize_on_drop: true,
            },
        }
    }
}

impl RykerConfig {
    /// Validate invariants from docs/CONFIG.md §4.
    pub fn validate(&self) -> Result<(), ConfigError> {
        use ConfigError::Invalid as Inv;

        if self.defaults.mailbox_capacity == 0 {
            return Err(Inv("mailbox_capacity must be >= 1".into()));
        }

        // Enforce [1KiB, 1MiB] inclusive
        let min_bytes = 1024_u64;
        let max_bytes = 1024_u64 * 1024;
        if self.defaults.max_msg_bytes < min_bytes || self.defaults.max_msg_bytes > max_bytes {
            return Err(Inv("max_msg_bytes must be within [1KiB, 1MiB]".into()));
        }

        // Enforce deadline ∈ [10ms, 60s]
        let min_deadline = Duration::from_millis(10);
        let max_deadline = Duration::from_secs(60);
        if self.defaults.deadline < min_deadline || self.defaults.deadline > max_deadline {
            return Err(Inv("deadline must be within [10ms, 60s]".into()));
        }

        if self.supervisor.backoff_base_ms == 0 {
            return Err(Inv("backoff_base_ms must be > 0".into()));
        }
        if self.supervisor.backoff_base_ms > self.supervisor.backoff_cap_ms {
            return Err(Inv("backoff_base_ms must be <= backoff_cap_ms".into()));
        }
        if self.supervisor.rapid_fail_threshold == 0 {
            return Err(Inv("rapid_fail_threshold must be >= 1".into()));
        }
        if self.supervisor.rapid_fail_window < Duration::from_secs(1) {
            return Err(Inv("rapid_fail_window must be >= 1s".into()));
        }

        if self.fairness.yield_every_n_msgs < self.fairness.batch_messages {
            return Err(Inv("yield_every_n_msgs must be >= batch_messages".into()));
        }

        if self.amnesia.enabled && !self.amnesia.zeroize_on_drop {
            return Err(Inv("amnesia.enabled requires zeroize_on_drop=true".into()));
        }

        Ok(())
    }

    /// Strict loader: reads optional file (dev), env overrides, then validates.
    /// Returns `Err` on any invalid config instead of swallowing it.
    pub fn from_env_validated() -> Result<Self, ConfigError> {
        let mut cfg = Self::default();

        // Optional dev convenience with prod guard: RYKER_CONFIG_PATH
        if let Some(path) = std::env::var("RYKER_CONFIG_PATH").ok() {
            if !path.trim().is_empty() {
                let env_is_prod = matches!(std::env::var("APP_ENV").as_deref(), Ok("production"))
                    || matches!(std::env::var("RUST_ENV").as_deref(), Ok("production"));
                let allow_dev_path =
                    matches!(std::env::var("RYKER_ALLOW_CONFIG_PATH").as_deref(), Ok("1"));

                if env_is_prod && !allow_dev_path {
                    return Err(ConfigError::Invalid(
                        "RYKER_CONFIG_PATH is not permitted in production (set RYKER_ALLOW_CONFIG_PATH=1 to override)"
                            .into(),
                    ));
                }

                let bytes = std::fs::read(path)?;
                // Try TOML first, then JSON.
                cfg = if let Ok(t) = toml::from_slice::<Self>(&bytes) {
                    t
                } else if let Ok(j) = serde_json::from_slice::<Self>(&bytes) {
                    j
                } else {
                    return Err(ConfigError::Invalid(
                        "RYKER_CONFIG_PATH: file is not valid TOML/JSON".into(),
                    ));
                };
            }
        }

        // Highest precedence after builder: env
        apply_env_overrides(&mut cfg)?;

        cfg.validate()?;
        Ok(cfg)
    }

    /// Builder-style ergonomic override.
    pub fn with_overrides<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }
}

/* -------------------- Env parsing helpers (DRY) -------------------- */

fn parse_size(s: &str) -> Result<u64, std::num::ParseIntError> {
    let s = s.trim();
    let sl = s.to_ascii_lowercase();
    if let Some(kib) = sl.strip_suffix("kib") {
        return kib.trim().parse::<u64>().map(|n| n * 1024);
    }
    if let Some(mib) = sl.strip_suffix("mib") {
        return mib.trim().parse::<u64>().map(|n| n * 1024 * 1024);
    }
    if let Some(b) = sl.strip_suffix('b') {
        return b.trim().parse::<u64>();
    }
    // Fall back to plain integer (bytes)
    sl.parse::<u64>()
}

fn parse_bool(s: &str) -> bool {
    match s {
        "1" | "true" | "TRUE" | "True" | "yes" | "YES" | "on" | "ON" => true,
        "0" | "false" | "FALSE" | "False" | "no" | "NO" | "off" | "OFF" => false,
        _ => false,
    }
}

// Warn helper: prefers tracing::warn! when feature enabled, else stderr.
fn warn(msg: &str) {
    #[cfg(feature = "tracing")]
    {
        tracing::warn!("{msg}");
    }
    #[cfg(not(feature = "tracing"))]
    {
        eprintln!("WARN: {msg}");
    }
}

macro_rules! env_set {
    // numeric u32
    ($cfg:expr, $env:literal, $setter:expr, u32) => {{
        if let Ok(v) = std::env::var($env) {
            match v.parse::<u32>() {
                Ok(n) => $setter($cfg, n),
                Err(e) => return Err(ConfigError::Parse { key: $env, msg: e.to_string() }),
            }
        }
    }};
    // size (KiB/MiB/B; case-insensitive)
    ($cfg:expr, $env:literal, $setter:expr, size) => {{
        if let Ok(v) = std::env::var($env) {
            match parse_size(&v) {
                Ok(n) => $setter($cfg, n),
                Err(e) => return Err(ConfigError::Parse { key: $env, msg: e.to_string() }),
            }
        }
    }};
    // duration
    ($cfg:expr, $env:literal, $setter:expr, dur) => {{
        if let Ok(v) = std::env::var($env) {
            match humantime::parse_duration(&v) {
                Ok(d) => $setter($cfg, d),
                Err(e) => return Err(ConfigError::Parse { key: $env, msg: e.to_string() }),
            }
        }
    }};
    // bool
    ($cfg:expr, $env:literal, $setter:expr, bool) => {{
        if let Ok(v) = std::env::var($env) {
            $setter($cfg, parse_bool(&v));
        }
    }};
    // enum SpanLevel via FromStr
    ($cfg:expr, $env:literal, $setter:expr, span) => {{
        if let Ok(v) = std::env::var($env) {
            match SpanLevel::from_str(&v) {
                Ok(lvl) => $setter($cfg, lvl),
                Err(msg) => return Err(ConfigError::Parse { key: $env, msg }),
            }
        }
    }};
}

fn apply_env_overrides(cfg: &mut RykerConfig) -> Result<(), ConfigError> {
    // Defaults
    env_set!(cfg, "RYKER_DEFAULT_MAILBOX_CAPACITY", |c, n| c.defaults.mailbox_capacity = n, u32);
    env_set!(cfg, "RYKER_DEFAULT_MAX_MSG_BYTES",   |c, n| c.defaults.max_msg_bytes = n, size);
    env_set!(cfg, "RYKER_DEFAULT_DEADLINE",       |c, d| c.defaults.deadline = d, dur);

    // Supervisor — prefer new key; if only old is set, honor and WARN.
    let new_jitter = std::env::var("RYKER_DECORRELATED_JITTER").ok();
    let old_jitter = std::env::var("RYKER_JITTER").ok();

    env_set!(cfg, "RYKER_BACKOFF_BASE_MS",        |c, n| c.supervisor.backoff_base_ms = n, u32);
    env_set!(cfg, "RYKER_BACKOFF_CAP_MS",         |c, n| c.supervisor.backoff_cap_ms = n, u32);
    env_set!(cfg, "RYKER_RAPID_FAIL_THRESHOLD",   |c, n| c.supervisor.rapid_fail_threshold = n, u32);
    env_set!(cfg, "RYKER_RAPID_FAIL_WINDOW",      |c, d| c.supervisor.rapid_fail_window = d, dur);

    // Apply jitter with aliasing rules:
    match (new_jitter.as_deref(), old_jitter.as_deref()) {
        (Some(nv), Some(_ov)) => {
            // Both set: new wins, WARN about old being ignored
            cfg.supervisor.decorrelated_jitter = parse_bool(nv);
            warn("RYKER_JITTER is deprecated and ignored because RYKER_DECORRELATED_JITTER is set; please remove the old variable.");
        }
        (Some(nv), None) => {
            cfg.supervisor.decorrelated_jitter = parse_bool(nv);
        }
        (None, Some(ov)) => {
            cfg.supervisor.decorrelated_jitter = parse_bool(ov);
            warn("RYKER_JITTER is deprecated; use RYKER_DECORRELATED_JITTER instead.");
        }
        (None, None) => {}
    }

    // Fairness
    env_set!(cfg, "RYKER_BATCH_MESSAGES",         |c, n| c.fairness.batch_messages = n, u32);
    env_set!(cfg, "RYKER_YIELD_EVERY_N",          |c, n| c.fairness.yield_every_n_msgs = n, u32);

    // Observability
    env_set!(cfg, "RYKER_ENABLE_METRICS",         |c, b| c.observability.enable_metrics = b, bool);
    env_set!(cfg, "RYKER_QUEUE_DEPTH_SAMPLING",   |c, b| c.observability.queue_depth_sampling = b, bool);
    env_set!(cfg, "RYKER_SPAN_LEVEL",             |c, s| c.observability.span_level = s, span);

    // Amnesia
    env_set!(cfg, "RYKER_AMNESIA",                |c, b| c.amnesia.enabled = b, bool);
    env_set!(cfg, "RYKER_ZEROIZE",                |c, b| c.amnesia.zeroize_on_drop = b, bool);

    Ok(())
}

/* -------------------- Optional: reload seam for hosts -------------------- */

/// Hosts can implement this to react to applied config snapshots (e.g., bump sampling).
pub trait RykerReloadHook: Send + Sync + 'static {
    fn on_reload(&self, old: &RykerConfig, new: &RykerConfig);
}

/* -------------------- Minimal unit tests (doc-anchored) -------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_bounds_enforced() {
        let mut cfg = RykerConfig::default();
        cfg.defaults.deadline = Duration::from_millis(9);
        assert!(cfg.validate().is_err());
        cfg.defaults.deadline = Duration::from_secs(61);
        assert!(cfg.validate().is_err());
        cfg.defaults.deadline = Duration::from_millis(10);
        assert!(cfg.validate().is_ok());
        cfg.defaults.deadline = Duration::from_secs(60);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn max_msg_bytes_bounds_enforced() {
        let mut cfg = RykerConfig::default();
        cfg.defaults.max_msg_bytes = 1023;
        assert!(cfg.validate().is_err());
        cfg.defaults.max_msg_bytes = 1024 * 1024 + 1;
        assert!(cfg.validate().is_err());
        cfg.defaults.max_msg_bytes = 1024;
        assert!(cfg.validate().is_ok());
        cfg.defaults.max_msg_bytes = 1024 * 1024;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn yield_not_less_than_batch() {
        let mut cfg = RykerConfig::default();
        cfg.fairness.batch_messages = 64;
        cfg.fairness.yield_every_n_msgs = 63;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rapid_window_minimum() {
        let mut cfg = RykerConfig::default();
        cfg.supervisor.rapid_fail_window = Duration::from_millis(999);
        assert!(cfg.validate().is_err());
        cfg.supervisor.rapid_fail_window = Duration::from_secs(1);
        assert!(cfg.validate().is_ok());
    }
}
````

