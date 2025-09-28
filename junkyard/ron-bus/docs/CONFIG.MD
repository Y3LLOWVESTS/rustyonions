

**Path:** `crates/ron-bus/docs/CONFIG.md`

````markdown
---
title: Configuration — ron-bus
crate: ron-bus
owner: Stevan White
last-reviewed: 2025-09-27
status: draft
template_version: 1.0
---

# Configuration — ron-bus

This document defines **all configuration** for `ron-bus`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> - **Library crate (this crate):** The bus does **not** read files/env/CLI. The **host** process is responsible for reading configuration and constructing `Bus::new(cfg)`.  
> - **Service crates (N/A here):** Network binds, /healthz, TLS, etc., are **out of scope** for `ron-bus`.

---

## 1) Sources & Precedence (Authoritative)

**ron-bus does not fetch configuration directly.** The host composes a `BusConfig` (or passes a capacity integer) from its own sources. Recommended precedence **in the host**:

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (e.g., `Config.toml` of the host)  
4. **Built-in defaults** (hard-coded in host)

> When the host supports dynamic reload, recompute the effective config under the same precedence and **recreate** the bus if needed (see §5).

**Supported file formats:** (host choice) TOML preferred; JSON optional.

**Suggested host env prefix:** `RON_BUS_` (e.g., `RON_BUS_CAPACITY=256`).

---

## 2) Quickstart Examples

> Examples show **host-side** usage. `ron-bus` itself does not parse env/CLI.

### 2.1 Minimal (host constructs bus with default capacity)

```rust
// host/src/main.rs (example)
use ron_bus::{Bus, Event};

let bus = Bus::new(Default::default()); // uses BusConfig::default() or capacity default
let tx = bus.sender();
let mut rx = bus.subscribe();
````

### 2.2 Host config file (TOML snippet)

```toml
# Host's Config.toml
[bus]
capacity = 256                      # bounded broadcast buffer (messages)
overflow_warn_rate_per_min = 60     # rate-limit WARNs for overflow
metrics_namespace = "ron_bus"       # label/prefix used by host metrics registry
emit_amnesia_label = true           # attach amnesia={on|off} to metrics
```

### 2.3 Host env and CLI (override file)

```bash
# If your platform has a global amnesia toggle:
RON_AMNESIA=on \
RON_BUS_EMIT_AMNESIA_LABEL=true \
RON_BUS_CAPACITY=512 \
RON_BUS_OVERFLOW_WARN_RATE_PER_MIN=120 \
cargo run -p <host-crate> -- --bus-capacity 512
```

*(Host maps `RON_AMNESIA` → metrics label value `amnesia=on|off` when `emit_amnesia_label=true`.)*

---

## 3) Schema (Typed, With Defaults)

> **Prefix convention for hosts:** `RON_BUS_…`
> Durations are expressed by hosts; `ron-bus` only needs integers/booleans/strings below.

| Key / Host Env Var                                                  | Type       |   Default | Description                                                                                                   | Security Notes                                   |
| ------------------------------------------------------------------- | ---------- | --------: | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `capacity` / `RON_BUS_CAPACITY`                                     | u32        |       256 | **Bounded** broadcast buffer (per Tokio `broadcast::channel`).                                                | Too small → frequent loss; too big → memory use. |
| `overflow_warn_rate_per_min` / `RON_BUS_OVERFLOW_WARN_RATE_PER_MIN` | u32        |        60 | Rate-limit for WARN logs on overflow (max WARN events/minute, per host).                                      | Prevents log spam (DoS via log amplification).   |
| `metrics_namespace` / `RON_BUS_METRICS_NAMESPACE`                   | string     | "ron_bus" | Namespace/prefix the **host** may use when registering Prometheus metrics that ron-bus updates.               | Keep namespaces stable for dashboards.           |
| `emit_amnesia_label` / `RON_BUS_EMIT_AMNESIA_LABEL`                 | bool       |      true | Whether the host attaches metrics label `amnesia` with values **`on`** or **`off`** to bus metrics.           | Label only; no behavioral change.                |
| *(read-only)* `bus_queue_depth`                                     | gauge<i64> |       N/A | **Gauge exposed by the host**; updated by ron-bus to reflect estimated backlog/lag under `metrics_namespace`. | N/A                                              |

> **N/A for this crate:** network binds, TLS, timeouts, body caps, UDS, auth. Those belong to services that **use** the bus, not to `ron-bus` itself.

---

## 4) Validation Rules (Fail-Closed)

Validation occurs when the **host** constructs `BusConfig` (or calls `Bus::new`):

* `capacity >= 2` (recommended `>= 8`; `0` or `1` is **invalid**, return error).
* `overflow_warn_rate_per_min >= 1`.
* `metrics_namespace` non-empty ASCII identifier (for Prometheus label safety).

**On violation:** return an error to the caller (do **not** panic). The host decides whether to exit or patch.

---

## 5) Dynamic Reload (If Supported by Host)

* **Trigger:** The host may use SIGHUP, a control plane, or a `ConfigUpdated { version }` event elsewhere.
* **Reload semantics in ron-bus:**

  * `capacity` is **not** hot-swappable (Tokio `broadcast` capacity is fixed).
  * To change `capacity`, **create a new `Bus`** and migrate subscribers (spin up new receivers, then drop old).
  * `overflow_warn_rate_per_min`, `metrics_namespace`, `emit_amnesia_label` are **host-side** concerns (no behavioral change in bus logic).
* **Atomicity:** Construct a new `Bus` off to the side; swap handles under a short, non-`await`ed critical section in the host.
* **Audit:** The host may emit a `KernelEvent::ConfigUpdated { version }` and structured diff (excluding secrets; none here).

---

## 6) CLI Flags (Canonical for Hosts)

> `ron-bus` is library-only. Suggested **host** flags:

```
--bus-capacity <num>                   # override capacity
--bus-overflow-warn-rate <per_min>    # override WARN rate limit
--bus-metrics-namespace <string>      # override namespace
--bus-amnesia-label <true|false>      # toggle emitting amnesia={on|off} label in exported metrics
```

---

## 7) Feature Flags (Cargo)

| Feature   | Default | Effect                                                                     |
| --------- | ------: | -------------------------------------------------------------------------- |
| `tracing` |     off | Enable light spans around publish/recv in examples (library remains pure). |
| `loom`    |     off | Dev-only testing to model interleavings (not shipped).                     |

> No TLS/PQ/etc. features here; cryptography and networking are out of scope.

---

## 8) Security Implications

* **Capacity as a guardrail:** Prevents slow consumers from wedging publishers (bounded loss by design).
* **Logging overflow:** Rate-limit WARNs via host to avoid log-amplification DoS.
* **No secrets:** The bus must not transport secrets as raw payloads (policy: keep sensitive data at the edge or use dedicated KMS/policy crates).
* **Amnesia neutrality:** Behavior identical whether `amnesia` is on/off; the label simply aids dashboards.

See `docs/SECURITY.md` for the threat subset; the bus is pure in-proc and has no I/O.

---

## 9) Compatibility & Migration

* **Minor (additive):** You may add fields with sane defaults (host side).
* **Renames:** Keep old env alias for ≥1 minor; log a deprecation warning (host side).
* **Breaking:** Changing semantics (e.g., meaning of `capacity`) requires a **major** version bump and documented migration steps.

**Deprecation table (maintained by host):**

| Old Key                  | New Key                              | Removal Target | Notes             |
| ------------------------ | ------------------------------------ | -------------: | ----------------- |
| `RON_BUS_AMNESIA` (bool) | `RON_BUS_EMIT_AMNESIA_LABEL`         |         v2.0.0 | Clarified intent. |
| `RON_BUS_WARN_RATE`      | `RON_BUS_OVERFLOW_WARN_RATE_PER_MIN` |         v2.0.0 | Units clarified.  |

---

## 10) Reference Implementation (Rust)

> Copy-paste into `crates/ron-bus/src/config.rs` (or inline within `lib.rs` if you prefer a single file).
> Comments included intentionally.

```rust
// crates/ron-bus/src/config.rs
use serde::{Deserialize, Serialize};

/// Library-facing configuration for the bus. Constructed by the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusConfig {
    /// Bounded capacity for the Tokio broadcast channel.
    /// MUST be >= 2; recommended >= 8.
    pub capacity: u32,

    /// Max WARN logs per minute the host should emit for overflow.
    /// This is a host concern; the bus exposes counters/gauges only.
    pub overflow_warn_rate_per_min: u32,

    /// Optional namespace the host may use when registering metrics.
    /// ron-bus does not register metrics itself; it only updates handles.
    pub metrics_namespace: String,

    /// Whether the host should annotate exported metrics with an `amnesia` label.
    /// Expected values: "on" or "off" (the host maps its own env to this).
    pub emit_amnesia_label: bool,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            capacity: 256,
            overflow_warn_rate_per_min: 60,
            metrics_namespace: "ron_bus".to_string(),
            emit_amnesia_label: true,
        }
    }
}

impl BusConfig {
    /// Strict validation. Return an error to the host if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.capacity < 2 {
            return Err("bus.capacity must be >= 2 (recommended >= 8)".into());
        }
        if self.overflow_warn_rate_per_min == 0 {
            return Err("bus.overflow_warn_rate_per_min must be >= 1".into());
        }
        if self.metrics_namespace.trim().is_empty() {
            return Err("bus.metrics_namespace must be non-empty".into());
        }
        Ok(())
    }
}

// Example constructor in lib:
//
// impl Bus {
//     pub fn new(cfg: impl Into<BusConfig>) -> Result<Self, BusError> {
//         let cfg = cfg.into();
//         cfg.validate().map_err(BusError::Config)?;
//         let (tx, _rx) = tokio::sync::broadcast::channel::<Event>(cfg.capacity as usize);
//         Ok(Self { tx, cap: cfg.capacity as usize /* ... */ })
//     }
// }
```

### 10.1 Host-side parsing example (figment)

```rust
// host/src/config_bus.rs (example)
use figment::{
    providers::{Format, Toml, Env},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
struct HostBusCfg {
    #[serde(default)]
    bus: ron_bus::config::BusConfig, // reuse the library's BusConfig
}

fn load_bus_cfg() -> ron_bus::config::BusConfig {
    // Load Config.toml (optional) + env with prefix RON_BUS_
    let figment = Figment::new()
        .merge(Toml::file("Config.toml").nested())
        .merge(Env::prefixed("RON_BUS_").split("_"));

    let cfg: HostBusCfg = figment.extract().unwrap_or_default();
    let bus = cfg.bus;
    bus.validate().expect("invalid bus config");
    bus
}

// Example mapping (Env):
// RON_BUS_CAPACITY=512
// RON_BUS_OVERFLOW_WARN_RATE_PER_MIN=120
// RON_BUS_METRICS_NAMESPACE=ron_bus
// RON_BUS_EMIT_AMNESIA_LABEL=true
//
// Optional global flag the host may have:
// RON_AMNESIA=on  -> host adds metrics label amnesia=on (if emit_amnesia_label=true)
```

---

## 11) Test Matrix

| Scenario                          | Expected Outcome                                                                                           |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `capacity=0` or `1`               | `Err("bus.capacity must be >= 2")`                                                                         |
| `overflow_warn_rate_per_min=0`    | `Err("must be >= 1")`                                                                                      |
| `metrics_namespace=""`            | `Err("non-empty")`                                                                                         |
| `capacity=8` (saturation load)    | Valid; overflow counter increases; host WARNs rate-limited; publishers non-blocking                        |
| `queue_depth` under burst load    | Host `bus_queue_depth` Gauge increases during burst and returns to baseline after; publishers non-blocking |
| `reload: capacity changed`        | Host constructs new bus; subscribers migrate; old bus dropped                                              |
| `reload: namespace/label changed` | No bus behavior change; only host metrics wiring affected                                                  |

---

## 12) Mermaid — Config Resolution Flow (Host + Bus)

```mermaid
flowchart TB
  A[Defaults (BusConfig::default)] --> M[Host Merge]
  B[Host Config File] --> M
  C[Host Env Vars (RON_BUS_*)] --> M
  E[Host CLI Flags] --> M
  M --> V{BusConfig::validate}
  V -- ok --> B2[Bus::new(cfg)]
  V -- fail --> X[Return error to host]
  style B2 fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* **Dashboarding:** Plot `bus_overflow_dropped_total` (Counter) and `bus_queue_depth` (Gauge). Label with `service` and (optionally) `amnesia`.
* **Tuning:** Prefer fixing slow subscribers first. Increase `capacity` only if overflow is chronic and bounded.
* **Reload strategy:** For `capacity` changes, **spin up a new bus** and cut subscribers over with a cancel token.
* **Documentation:** Any host that uses the bus should include its bus-related knobs in its own `CONFIG.md` and link back here.

```
