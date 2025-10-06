---

title: Configuration — ron-policy
crate: ron-policy
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
template_version: 1.0
---------------------

# Configuration — ron-policy

This document defines **all configuration** for `ron-policy`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**
> **Library crate.** `ron-policy` is **pure** (no I/O, no sockets). It exposes a typed `Config`
> and validators for consumers (e.g., `svc-gateway`, `svc-storage`). Service-only notes are marked **(service-only)**.

---

## 1) Sources & Precedence (Authoritative)

`ron-policy` does not read files/env by itself. Consumers provide a `ron_policy::Config`
instance. If a consumer also supports external sources, use this precedence (**highest wins**):

1. **Process flags** (CLI) **(service-only)**
2. **Environment variables** **(service-only)**
3. **Config file** (JSON/TOML) **(service-only)**
4. **Built-in defaults** (the `Config::default()` in the consumer)

> On dynamic reload, recompute the effective config under the same precedence and re-validate with `ron_policy::validate_config()`.

**Supported on-wire formats (when consumers persist config):** JSON (canonical), TOML (authoring convenience).
**Relative `--config` paths (service-only):** resolve as `./`, then `$CWD`, then service crate dir.

---

## 2) Quickstart Examples

### 2.1 Library embedding (typical)

```rust
use ron_policy::{Config, Validation, PolicyBundle};

let cfg = Config::default();                // strict + tighten-only by default
cfg.validate()?;                            // fail-closed if misconfigured

// Load or receive a PolicyBundle from svc-registry (service does the I/O)
let bundle: PolicyBundle = obtain_bundle_from_registry()?;

// Validate bundle against config (tighten-only, churn, schema)
Validation::new(&cfg).validate_bundle(&bundle)?;
```

### 2.2 Consumer service overlay (e.g., svc-gateway) **(service-only)**

```bash
RON_POLICY_STRICT_DENY_UNKNOWN=true \
RON_POLICY_STRICT_TIGHTEN_ONLY=true \
RON_POLICY_REQUIRE_BREAK_CHANGE=true \
RON_POLICY_CANONICAL_WIRE=json \
RON_POLICY_PERF_MAX_RULES_HOT_PATH=1000 \
RON_POLICY_PERF_P95_BUDGET_MS=1 \
svc-gateway --config /etc/ron/gateway.toml
```

### 2.3 Config file snippet merged by consumer **(service-only)**

```toml
# policy-config.toml
[policy]
strict_deny_unknown = true
strict_tighten_only = true
require_break_change = true

[policy.limits]
max_body_upper_bound = "1MiB"
decompress_ratio_upper_bound = 10

[policy.perf]
max_rules_hot_path = 1000
p95_budget_ms = 1

[policy.schema]
canonical_wire = "json"
emit_json_schema = false
```

---

## 3) Schema (Typed, With Defaults)

> **Env prefix for consumers:** `RON_POLICY_…`
> **Durations:** `ms`, `s`, `m`, `h` (when services choose to expose durations).
> **Sizes:** `B`, `KB`, `MB`, `MiB`.

**Core (library) configuration**

| Key / Env Var (service-only)                                 | Type                 | Default | Description                                                                | Security Notes                          |
| ------------------------------------------------------------ | -------------------- | ------- | -------------------------------------------------------------------------- | --------------------------------------- |
| `strict_deny_unknown` / `RON_POLICY_STRICT_DENY_UNKNOWN`     | bool                 | `true`  | Reject unknown fields in bundles (`serde(deny_unknown_fields)`).           | Prevents config smuggling.              |
| `strict_tighten_only` / `RON_POLICY_STRICT_TIGHTEN_ONLY`     | bool (hard-enforced) | `true`  | Bundles may only **tighten** global caps (never relax).                    | Guards hardening invariants.            |
| `require_break_change` / `RON_POLICY_REQUIRE_BREAK_CHANGE`   | bool                 | `true`  | Widening residency/limits requires `metadata.break_change=true` + runbook. | Forces explicit, auditable rollouts.    |
| `amnesia_default_ok` / `RON_POLICY_AMNESIA_DEFAULT_OK`       | bool                 | `true`  | If true, evaluator emits degrade-writes obligations by default.            | Avoids silent persistence on Micronode. |
| `econ_zk_hints_allowed` / `RON_POLICY_ECON_ZK_HINTS_ALLOWED` | bool                 | `true`  | Allow declarative `RequireProof{…}` obligations (no crypto here).          | No keys/compute in this crate.          |

**Limits (upper bounds, aligned with Hardening Blueprint)**

| Key / Env Var (service-only)                                                | Type | Default | Description                                          | Security Notes        |
| --------------------------------------------------------------------------- | ---- | ------- | ---------------------------------------------------- | --------------------- |
| `limits.max_body_upper_bound` / `RON_POLICY_MAX_BODY_UPPER_BOUND`           | size | `1MiB`  | Hard ceiling; policy `max_body_bytes` must be ≤ this | Content-length guard. |
| `limits.decompress_ratio_upper_bound` / `RON_POLICY_DECOMPRESS_UPPER_BOUND` | u32  | `10`    | Hard ceiling; policy `decompress_ratio_max` ≤ this   | Zip-bomb guard.       |

**Performance (evaluation budgets)**

| Key / Env Var (service-only)                                     | Type | Default | Description                                      |
| ---------------------------------------------------------------- | ---- | ------- | ------------------------------------------------ |
| `perf.max_rules_hot_path` / `RON_POLICY_PERF_MAX_RULES_HOT_PATH` | u32  | `1000`  | Upper bound of rules considered in hot routes.   |
| `perf.p95_budget_ms` / `RON_POLICY_PERF_P95_BUDGET_MS`           | u32  | `1`     | Target p95 evaluation budget (ms) for hot paths. |

**Schema / Wire**

| Key / Env Var (service-only)                          | Type                | Default | Description                                             |
| ----------------------------------------------------- | ------------------- | ------- | ------------------------------------------------------- |
| `schema.canonical_wire` / `RON_POLICY_CANONICAL_WIRE` | enum(`json`,`toml`) | `json`  | Canonical wire format; TOML allowed for authoring only. |
| `schema.emit_json_schema` / `RON_POLICY_EMIT_SCHEMA`  | bool                | `false` | If consumer exposes, emit JSON Schema for bundles.      |

**Clock / Evaluation**

| Key / Env Var (service-only)           | Type                      | Default    | Description                                         |
| -------------------------------------- | ------------------------- | ---------- | --------------------------------------------------- |
| `eval.clock` / `RON_POLICY_EVAL_CLOCK` | enum(`provided`,`system`) | `provided` | Evaluator expects caller-supplied clock (epoch ms). |

**Service integration keys (owned by consumers; documented here for consistency) — (service-only)**

| Key / Env Var                                       | Type                    | Default    | Description                                                 |
| --------------------------------------------------- | ----------------------- | ---------- | ----------------------------------------------------------- |
| `policy.source` / `RON_POLICY_SOURCE`               | enum(`registry`,`file`) | `registry` | Where bundles come from (multi-sig registry vs local file). |
| `policy.bundle_path` / `RON_POLICY_BUNDLE_PATH`     | path                    | `""`       | File path if `source=file` (dev/test only).                 |
| `policy.reload_signal` / `RON_POLICY_RELOAD_SIGNAL` | enum(`sighup`,`bus`)    | `bus`      | How the service triggers revalidation/reload.               |

---

## 4) Validation Rules (Fail-Closed)

Applied by `ron_policy::Validation` and should be enforced by consumers at startup and on reload:

* **Tighten-only:** `max_body_bytes ≤ limits.max_body_upper_bound` and `decompress_ratio_max ≤ limits.decompress_ratio_upper_bound`.
* **Churn safety:** If residency/limits widen relative to the previous bundle, **require** `metadata.break_change=true` and a non-empty `runbook_url`.
* **Schema strictness:** Unknown fields in a bundle cause rejection when `strict_deny_unknown=true`.
* **Determinism knobs:** `eval.clock="provided"` requires a supplied clock; reject otherwise.
* **Performance intent:** If `perf.max_rules_hot_path` is exceeded by a bundle section, warn + optionally reject (consumer policy); bench gates live in CI.

**On violation:** return structured errors (library) → consumers **must** fail fast (exit non-zero) for startup config; on reload, revert to the last good snapshot.

---

## 5) Dynamic Reload

* **Triggers:** `PolicyChanged{version,id_b3}` bus event **(service-only)** or SIGHUP mapped by the consumer.
* **Semantics:** Consumers fetch the new bundle, then call `Validation::validate_bundle(&bundle)` under current `Config`; if valid, swap the in-memory snapshot atomically.
* **Atomicity:** Replace snapshot via lock-free swap or a short critical section **without holding `.await`**.
* **Audit:** Consumers should log the version bump and include a non-secret diff summary.

---

## 6) CLI Flags (Canonical) **(service-only)**

```
--policy-source <registry|file>
--policy-bundle-path <path>      # when --policy-source=file
--policy-reload <sighup|bus>
--policy-tighten-only <bool>
--policy-require-break-change <bool>
--policy-max-body-upper-bound <size>
--policy-decompress-upper-bound <u32>
--policy-perf-max-rules <u32>
--policy-perf-p95-budget-ms <u32>
--policy-wire <json|toml>
--policy-emit-schema <bool>
```

---

## 7) Cargo Features (Library)

| Feature      | Default | Effect                                                                |
| ------------ | ------: | --------------------------------------------------------------------- |
| `schema`     |     off | Include embedded JSON Schema and helpers to emit/validate schemas.    |
| `pq`         |     off | Enable advisory PQ flags in DTOs (no crypto).                         |
| `cli`        |     off | Parse env/flags in **example/test bins only** (no effect in library). |
| `kameo`      |     off | Optional actor integration (types only; still no I/O).                |
| `serde_json` |      on | JSON (canonical wire).                                                |
| `toml`       |      on | TOML (authoring convenience in tests/examples).                       |

> Enabling `cli` must not add I/O to the core library; it only affects examples/tests.

---

## 8) Security Implications

* **Tighten-only is non-negotiable.** Turning it off is forbidden by IDB; the validator always enforces upper bounds.
* **Churn protection:** Widening residency or limits without `break_change` must be rejected; requires a runbook link.
* **No keys/secrets:** ECON/ZK obligations are declarative; no secret material ever enters `ron-policy`.
* **Amnesia alignment:** When `amnesia_default_ok=true`, emit `DegradeWritesFirst` obligations; consumers must honor them.
* **Schema choice:** Canonical wire is JSON; TOML is for authoring. Avoid mixed formats in production pipelines.

---

## 9) Compatibility & Migration

* **Backward-compatible changes:** Add new config keys with safe defaults and document them.
* **Renames:** Keep env var aliases for ≥1 minor release; emit deprecation warnings in consumers.
* **Breaking changes:** Require a major and an explicit migration section in `CHANGELOG.md`.
* **Bundle schema evolution:** Use semver‐tracked JSON Schema; CI tests enforce compatibility for N→N+1 unless `break_change=true`.

**Deprecation table (maintained):**

| Old Key | New Key | Removal Target | Notes |
| ------: | :------ | -------------: | :---- |
|       — | —       |              — | —     |

---

## 10) Reference Implementation (Rust)

```rust
// crates/ron-policy/src/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsCfg {
    #[serde(default = "default_max_body_upper_bound")]
    pub max_body_upper_bound: u64,      // bytes (1 MiB)
    #[serde(default = "default_decompress_ratio_upper_bound")]
    pub decompress_ratio_upper_bound: u32, // 10x
}
fn default_max_body_upper_bound() -> u64 { 1 * 1024 * 1024 }
fn default_decompress_ratio_upper_bound() -> u32 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfCfg {
    #[serde(default = "default_max_rules_hot_path")]
    pub max_rules_hot_path: u32,        // ≤ 1000
    #[serde(default = "default_p95_budget_ms")]
    pub p95_budget_ms: u32,             // 1 ms
}
fn default_max_rules_hot_path() -> u32 { 1000 }
fn default_p95_budget_ms() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalWire { Json, Toml }

impl Default for CanonicalWire { fn default() -> Self { CanonicalWire::Json } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCfg {
    #[serde(default)]
    pub emit_json_schema: bool,
    #[serde(default)]
    pub canonical_wire: CanonicalWire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub strict_deny_unknown: bool,
    #[serde(default = "default_true")]
    pub strict_tighten_only: bool,
    #[serde(default = "default_true")]
    pub require_break_change: bool,
    #[serde(default = "default_true")]
    pub amnesia_default_ok: bool,
    #[serde(default = "default_true")]
    pub econ_zk_hints_allowed: bool,

    #[serde(default)]
    pub limits: LimitsCfg,
    #[serde(default)]
    pub perf: PerfCfg,
    #[serde(default)]
    pub schema: SchemaCfg,
}
fn default_true() -> bool { true }

impl Default for Config {
    fn default() -> Self {
        Self {
            strict_deny_unknown: true,
            strict_tighten_only: true,
            require_break_change: true,
            amnesia_default_ok: true,
            econ_zk_hints_allowed: true,
            limits: LimitsCfg { max_body_upper_bound: default_max_body_upper_bound(), decompress_ratio_upper_bound: default_decompress_ratio_upper_bound() },
            perf: PerfCfg { max_rules_hot_path: default_max_rules_hot_path(), p95_budget_ms: default_p95_budget_ms() },
            schema: SchemaCfg { emit_json_schema: false, canonical_wire: CanonicalWire::Json },
        }
    }
}

// Example validator entrypoint
pub struct Validation<'a> { cfg: &'a Config }
impl<'a> Validation<'a> {
    pub fn new(cfg: &'a Config) -> Self { Self { cfg } }
    pub fn validate_config(&self) -> anyhow::Result<()> {
        if self.cfg.limits.max_body_upper_bound < 1024 {
            anyhow::bail!("max_body_upper_bound too small");
        }
        if self.cfg.limits.decompress_ratio_upper_bound < 1 {
            anyhow::bail!("decompress_ratio_upper_bound must be ≥ 1");
        }
        if self.cfg.perf.max_rules_hot_path == 0 {
            anyhow::bail!("max_rules_hot_path must be > 0");
        }
        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                                              | Expected Outcome                                 |
| ----------------------------------------------------- | ------------------------------------------------ |
| `strict_deny_unknown=true` and bundle has extra field | Rejected with descriptive error                  |
| Policy widens residency w/o `break_change`            | Rejected; error references runbook requirement   |
| Policy relaxes caps (body/ratio)                      | Rejected (tighten-only guard)                    |
| Hot path > `max_rules_hot_path`                       | Warning or reject per consumer policy            |
| `canonical_wire=toml` used as wire                    | Allowed for authoring; consumer converts to JSON |
| `amnesia_default_ok=true`                             | Evaluator emits `DegradeWritesFirst` obligation  |

---

## 12) Mermaid — Config Resolution Flow (Consumer)

```mermaid
flowchart TB
  A[Built-in Defaults] --> M[Merge]
  B[Config File (policy.*)] --> M
  C[Env (RON_POLICY_*)] --> M
  D[CLI Flags (--policy-*)] --> M
  M --> V{Validate with ron_policy}
  V -- ok --> S[Snapshot -> Evaluator]
  V -- fail --> X[Exit / Revert to last good]
  style S fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Treat **policy config and bundles as code**: version-control them, require code review, and accompany widening changes with a **runbook URL**.
* Keep `RON_POLICY_*` keys **consistent across services**; avoid per-service drift.
* Avoid mixing TOML and JSON in production pipelines; **JSON is canonical wire**.
* On reload failure, prefer **revert to last good** over continuing with a partially applied config.
