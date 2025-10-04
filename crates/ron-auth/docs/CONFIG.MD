

````markdown
---
title: Configuration — ron-auth
crate: ron-auth
owner: Stevan White
last-reviewed: 2025-10-04
status: draft
template_version: 1.0
---

# Configuration — ron-auth

This document defines **all configuration** for `ron-auth` (a pure library), including
sources, precedence, schema (types/defaults), validation, feature flags, reload patterns,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> - **Library crate (this crate):** no network/disk I/O, no ports, no `/metrics` or `/healthz`.  
> - **Service crates (consumers):** parse env/flags/files and **inject** config into `ron-auth`.

---

## 1) Sources & Precedence (Authoritative)

`ron-auth` is **injected** with config; it does not read files or bind sockets.

**Effective precedence (highest wins):**
1. **Builder overrides** in host code (e.g., `VerifierConfig::builder().max_token_bytes(4096)`)  
2. **Host-provided environment / flags / files** (parsed by the service, then passed in)  
3. **Built-in defaults** (safe, conservative)

> The library **never** reads env or files by default. Optional helper parsing is available behind the `config-env` feature (no disk I/O, env only).

**Supported file formats (when parsed by the host):** TOML (preferred), JSON (optional).  
**Prefix convention if env is used by the host:** `RON_AUTH_…` (e.g., `RON_AUTH_MAX_TOKEN_BYTES`).

---

## 2) Quickstart Examples

### 2.1 Minimal host setup (service side)
```rust
use ron_auth::config::{VerifierConfig, CaveatPolicy, ContextDefaults};

let cfg = VerifierConfig::builder()
    .max_token_bytes(4096)
    .max_caveats(64)
    .clock_skew_secs(300)
    .caveat_policy(CaveatPolicy {
        allow_custom_namespaces: vec!["com.acme".into()],
        unknown_custom_behavior: ron_auth::config::UnknownCustom::Deny, // IDB fail-closed
    })
    .context_defaults(ContextDefaults {
        amnesia: false,                    // host will set per-request when ON
        policy_digest_hex: None,           // host injects live digest at request time
        redaction_digest_prefix_bytes: 8,  // only used if host logs redacted digests
    })
    .build()?;
````

### 2.2 (Optional) Env → config (only with `config-env` feature)

```rust
// In the service crate (not in ron-auth), with feature `config-env`.
let cfg = ron_auth::config::from_env_with_defaults(None)?; // reads RON_AUTH_* env vars
```

### 2.3 Per-request context injection

```rust
use ron_auth::RequestCtx;
let ctx = RequestCtx {
    now_unix_s: now(),
    method: "GET",
    path: "/o/b3:deadbeef...",
    peer_ip: Some(client_ip),
    object_addr: Some("b3:deadbeef..."),
    tenant: "tenant-123",
    amnesia: host_amnesia_flag,                 // dynamic
    policy_digest_hex: host_policy_digest_opt,  // dynamic
    extras: serde_cbor::Value::Null,
};
```

---

## 3) Schema (Typed, With Defaults)

> **Applies to the verification library.** Network/TLS keys are intentionally **N/A**.

| Key (struct field)                               | Type                 | Default | Description                                                   | Security Notes                                         |
| ------------------------------------------------ | -------------------- | ------- | ------------------------------------------------------------- | ------------------------------------------------------ |
| `max_token_bytes`                                | `u32`                | `4096`  | Hard cap on Base64URL-decoded token bytes                     | Guards resource abuse (IDB I-12)                       |
| `max_caveats`                                    | `u16`                | `64`    | Max number of caveats allowed                                 | Prevents pathologic tokens (I-12)                      |
| `clock_skew_secs`                                | `u32`                | `300`   | Allowed ± skew for `exp`/`nbf`                                | Time robustness (I-10)                                 |
| `caveat_policy.allow_custom_namespaces`          | `Vec<String>`        | `[]`    | Whitelisted namespaces for `Custom{ns,...}` caveats           | Narrow attack surface (P-3, I-9)                       |
| `caveat_policy.unknown_custom_behavior`          | `enum UnknownCustom` | `Deny`  | `Deny` or `Ignore` unknown custom caveats                     | **Fail-closed by default** (P-5, I-9)                  |
| `context_defaults.amnesia`                       | `bool`               | `false` | Default amnesia state if host omits it per request            | If `true`, host must enforce amnesia guarantees (I-21) |
| `context_defaults.policy_digest_hex`             | `Option<String>`     | `None`  | Default governance policy digest (if host has a fixed policy) | Used only if tokens bind to policy (I-22)              |
| `context_defaults.redaction_digest_prefix_bytes` | `u8`                 | `8`     | If host emits redacted digests, how many bytes to keep        | Never log raw token (I-15)                             |
| `perf.verify_target_us_base`                     | `u32`                | `60`    | Soft SLO: base microseconds per verification (for benches)    | Used in tests/benches (G-12)                           |
| `perf.verify_target_us_per_caveat`               | `u32`                | `8`     | Soft SLO: extra µs per caveat (for benches)                   | Used in tests/benches (G-12)                           |

**Env variable mapping (only if host uses `config-env` helper):**

* `RON_AUTH_MAX_TOKEN_BYTES`, `RON_AUTH_MAX_CAVEATS`, `RON_AUTH_CLOCK_SKEW_SECS`
* `RON_AUTH_ALLOW_CUSTOM_NAMESPACES` (comma-separated)
* `RON_AUTH_UNKNOWN_CUSTOM` = `DENY|IGNORE`
* `RON_AUTH_DEFAULT_AMNESIA` = `true|false`
* `RON_AUTH_POLICY_DIGEST_HEX` = hex string
* `RON_AUTH_REDACTION_PREFIX_BYTES` = `u8`
* `RON_AUTH_VERIFY_TARGET_US_BASE`, `RON_AUTH_VERIFY_TARGET_US_PER_CAVEAT`

---

## 4) Validation Rules (Fail-Closed)

On build (`VerifierConfig::build()`), enforce:

* `max_token_bytes ∈ [512, 16384]` (reject out-of-range)
* `max_caveats ∈ [1, 1024]`
* `clock_skew_secs ≤ 3600`
* `unknown_custom_behavior` defaults to **Deny** if unspecified
* If `context_defaults.policy_digest_hex` set, it must be valid **hex** of a BLAKE3 digest (length 64)
* Redaction prefix bytes ≤ 32

**On violation:** return a typed error (`ConfigError`) — never panic (I-16).

---

## 5) Dynamic Reload (Pattern)

Because `ron-auth` is pure, reloading is a **host concern**. Recommended pattern:

* Store `Arc<VerifierConfig>` (immutable) or `arc_swap::ArcSwap<VerifierConfig>`
* On host config change (SIGHUP/bus event), **build + validate** a new `VerifierConfig`, swap atomically
* `RequestCtx` remains **per request** (amnesia/policy digest are dynamic)

**Atomicity rule:** compute new config off-thread; swap without holding `.await`.

---

## 6) CLI Flags

**N/A for this library.** Host services should expose flags and map them into `VerifierConfig`. Canonical flag names in hosts:

```
--auth-max-token-bytes <u32>
--auth-max-caveats <u16>
--auth-clock-skew <secs>
--auth-allow-custom <ns,ns,...>
--auth-unknown-custom <deny|ignore>
--auth-default-amnesia <bool>
--auth-policy-digest <hex>
--auth-redaction-prefix <u8>
```

---

## 7) Feature Flags (Cargo)

| Feature         | Default | Effect                                                            |
| --------------- | :-----: | ----------------------------------------------------------------- |
| `verify`        |   yes   | Core verification APIs and types (default)                        |
| `pq-hybrid`     |    no   | Adds `SigAdapter` for hybrid envelopes (Ed25519+Dilithium2)       |
| `mint-internal` |    no   | **doc(hidden)**; used only by `svc-passport` + tests (I-23, G-16) |
| `config-env`    |    no   | Optional helper to parse `RON_AUTH_*` env into `VerifierConfig`   |
| `kameo`         |    no   | Optional actor integration (host-side only; no I/O in lib)        |

> CI must ensure `mint-internal` is OFF for all non-passport crates (G-16).

---

## 8) Security Implications

* **Fail-closed defaults:** unknown custom caveats **deny** (P-5, I-9).
* **Amnesia binding:** if tokens carry `Amnesia(true)`, hosts must propagate `amnesia=true` only when the process runs in amnesia mode (RAM-only caches, no persistent logs) (I-21).
* **Governance digest:** binding is equality-only; `ron-auth` never interprets policy content (I-22).
* **No secret logs:** library never logs tokens; if host logs correlators, use `b3(token)[..prefix]` only (I-15).
* **Config misuse:** expanding `allow_custom_namespaces` increases attack surface; keep **minimal**.

---

## 9) Compatibility & Migration

* Add new fields with sensible defaults; never widen defaults in ways that reduce safety.
* Renames require a deprecation alias (env helper) for ≥1 minor release.
* Breaking behavior changes require a **major** semver bump and CHANGELOG guidance.

---

## 10) Reference Implementation (Rust)

> Drop this into `src/config.rs`. No I/O; only types, defaults, validation.
> Env parsing (`from_env_with_defaults`) is behind `config-env` and reads **env only**.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UnknownCustom { Deny, Ignore }

impl Default for UnknownCustom { fn default() -> Self { UnknownCustom::Deny } }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaveatPolicy {
    #[serde(default)]
    pub allow_custom_namespaces: Vec<String>,
    #[serde(default)]
    pub unknown_custom_behavior: UnknownCustom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDefaults {
    #[serde(default)]
    pub amnesia: bool,
    #[serde(default)]
    pub policy_digest_hex: Option<String>,
    #[serde(default = "default_redaction_len")]
    pub redaction_digest_prefix_bytes: u8,
}
fn default_redaction_len() -> u8 { 8 }

impl Default for ContextDefaults {
    fn default() -> Self {
        Self { amnesia: false, policy_digest_hex: None, redaction_digest_prefix_bytes: 8 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierConfig {
    #[serde(default = "default_max_token_bytes")]
    pub max_token_bytes: u32,          // decoded token size cap (bytes)
    #[serde(default = "default_max_caveats")]
    pub max_caveats: u16,              // number of caveats cap
    #[serde(default = "default_clock_skew")]
    pub clock_skew_secs: u32,          // ± skew for exp/nbf
    #[serde(default)]
    pub caveat_policy: CaveatPolicy,   // custom caveat rules
    #[serde(default)]
    pub context_defaults: ContextDefaults, // defaults if host omits fields
    // Soft perf SLO hints used by benches/tests (not enforced at runtime):
    #[serde(default = "default_verify_base")]
    pub perf_verify_target_us_base: u32,
    #[serde(default = "default_verify_per_caveat")]
    pub perf_verify_target_us_per_caveat: u32,
}

fn default_max_token_bytes() -> u32 { 4096 }
fn default_max_caveats() -> u16 { 64 }
fn default_clock_skew() -> u32 { 300 }
fn default_verify_base() -> u32 { 60 }
fn default_verify_per_caveat() -> u32 { 8 }

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            max_token_bytes: default_max_token_bytes(),
            max_caveats: default_max_caveats(),
            clock_skew_secs: default_clock_skew(),
            caveat_policy: CaveatPolicy::default(),
            context_defaults: ContextDefaults::default(),
            perf_verify_target_us_base: default_verify_base(),
            perf_verify_target_us_per_caveat: default_verify_per_caveat(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("max_token_bytes must be in [512, 16384], got {0}")]
    MaxTokenBytes(u32),
    #[error("max_caveats must be in [1, 1024], got {0}")]
    MaxCaveats(u16),
    #[error("clock_skew_secs must be <= 3600, got {0}")]
    ClockSkew(u32),
    #[error("invalid policy_digest_hex (must be 64 hex chars)")]
    PolicyDigestHex,
    #[error("redaction_digest_prefix_bytes must be <= 32, got {0}")]
    RedactionPrefix(u8),
}

impl VerifierConfig {
    pub fn builder() -> VerifierConfigBuilder { VerifierConfigBuilder::default() }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(512..=16384).contains(&self.max_token_bytes) {
            return Err(ConfigError::MaxTokenBytes(self.max_token_bytes));
        }
        if !(1..=1024).contains(&self.max_caveats) {
            return Err(ConfigError::MaxCaveats(self.max_caveats));
        }
        if self.clock_skew_secs > 3600 {
            return Err(ConfigError::ClockSkew(self.clock_skew_secs));
        }
        if let Some(hex) = &self.context_defaults.policy_digest_hex {
            // BLAKE3 hex digest is 32 bytes = 64 hex chars.
            let ok_len = hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit());
            if !ok_len { return Err(ConfigError::PolicyDigestHex); }
        }
        if self.context_defaults.redaction_digest_prefix_bytes > 32 {
            return Err(ConfigError::RedactionPrefix(self.context_defaults.redaction_digest_prefix_bytes));
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct VerifierConfigBuilder {
    inner: VerifierConfig,
}

impl VerifierConfigBuilder {
    pub fn max_token_bytes(mut self, v: u32) -> Self { self.inner.max_token_bytes = v; self }
    pub fn max_caveats(mut self, v: u16) -> Self { self.inner.max_caveats = v; self }
    pub fn clock_skew_secs(mut self, v: u32) -> Self { self.inner.clock_skew_secs = v; self }
    pub fn caveat_policy(mut self, v: CaveatPolicy) -> Self { self.inner.caveat_policy = v; self }
    pub fn context_defaults(mut self, v: ContextDefaults) -> Self { self.inner.context_defaults = v; self }
    pub fn perf_verify_target_us_base(mut self, v: u32) -> Self { self.inner.perf_verify_target_us_base = v; self }
    pub fn perf_verify_target_us_per_caveat(mut self, v: u32) -> Self { self.inner.perf_verify_target_us_per_caveat = v; self }

    pub fn build(mut self) -> Result<VerifierConfig, ConfigError> {
        let cfg = std::mem::take(&mut self.inner);
        cfg.validate()?;
        Ok(cfg)
    }
}

#[cfg(feature = "config-env")]
pub mod env_helper {
    use super::*;
    /// Build VerifierConfig from `RON_AUTH_*` environment vars,
    /// overlaying `defaults` (or `Default::default()` if None).
    pub fn from_env_with_defaults(defaults: Option<VerifierConfig>) -> Result<VerifierConfig, ConfigError> {
        let mut cfg = defaults.unwrap_or_default();

        if let Ok(s) = std::env::var("RON_AUTH_MAX_TOKEN_BYTES") {
            if let Ok(v) = s.parse::<u32>() { cfg.max_token_bytes = v; }
        }
        if let Ok(s) = std::env::var("RON_AUTH_MAX_CAVEATS") {
            if let Ok(v) = s.parse::<u16>() { cfg.max_caveats = v; }
        }
        if let Ok(s) = std::env::var("RON_AUTH_CLOCK_SKEW_SECS") {
            if let Ok(v) = s.parse::<u32>() { cfg.clock_skew_secs = v; }
        }
        if let Ok(s) = std::env::var("RON_AUTH_ALLOW_CUSTOM_NAMESPACES") {
            cfg.caveat_policy.allow_custom_namespaces =
                s.split(',').filter(|x| !x.is_empty()).map(|x| x.trim().to_string()).collect();
        }
        if let Ok(s) = std::env::var("RON_AUTH_UNKNOWN_CUSTOM") {
            cfg.caveat_policy.unknown_custom_behavior = match s.to_ascii_uppercase().as_str() {
                "IGNORE" => UnknownCustom::Ignore,
                _ => UnknownCustom::Deny,
            };
        }
        if let Ok(s) = std::env::var("RON_AUTH_DEFAULT_AMNESIA") {
            cfg.context_defaults.amnesia = matches!(s.to_ascii_lowercase().as_str(), "1"|"true"|"yes"|"on");
        }
        if let Ok(s) = std::env::var("RON_AUTH_POLICY_DIGEST_HEX") {
            let s = s.trim().to_string();
            cfg.context_defaults.policy_digest_hex = if s.is_empty() { None } else { Some(s) };
        }
        if let Ok(s) = std::env::var("RON_AUTH_REDACTION_PREFIX_BYTES") {
            if let Ok(v) = s.parse::<u8>() { cfg.context_defaults.redaction_digest_prefix_bytes = v; }
        }
        if let Ok(s) = std::env::var("RON_AUTH_VERIFY_TARGET_US_BASE") {
            if let Ok(v) = s.parse::<u32>() { cfg.perf_verify_target_us_base = v; }
        }
        if let Ok(s) = std::env::var("RON_AUTH_VERIFY_TARGET_US_PER_CAVEAT") {
            if let Ok(v) = s.parse::<u32>() { cfg.perf_verify_target_us_per_caveat = v; }
        }

        cfg.validate()?;
        Ok(cfg)
    }
}
```

---

## 11) Test Matrix

| Scenario                                       | Expected Outcome                                    |
| ---------------------------------------------- | --------------------------------------------------- |
| Omit all fields                                | `VerifierConfig::default()` + `validate()` OK       |
| `max_token_bytes = 128`                        | `Err(ConfigError::MaxTokenBytes)`                   |
| `max_caveats = 0`                              | `Err(ConfigError::MaxCaveats)`                      |
| `clock_skew_secs = 7200`                       | `Err(ConfigError::ClockSkew)`                       |
| `policy_digest_hex` length ≠ 64 or non-hex     | `Err(ConfigError::PolicyDigestHex)`                 |
| `redaction_digest_prefix_bytes = 64`           | `Err(ConfigError::RedactionPrefix)`                 |
| Env helper: IGNORE unknown custom              | Sets `UnknownCustom::Ignore`, still `validate()` OK |
| Env helper: allow_custom_namespaces = "a,b,,c" | Parses to `["a","b","c"]`                           |

---

## 12) Mermaid — Config Resolution (Library Context)

```mermaid
flowchart TB
  A[Built-in defaults] --> M[Host merge (file/env/flags)]
  M --> B[Builder overrides]
  B --> V{Validate}
  V -- ok --> S[Arc<VerifierConfig> Snapshot]
  V -- fail --> E[ConfigError]
  style S fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes for Host Services

* Treat `amnesia` and `policy_digest_hex` as **dynamic** request context; don’t hardcode in global config unless truly static.
* Keep `allow_custom_namespaces` minimal; review any addition in security PRs.
* Benchmarks: wire `perf_*` fields into criterion baselines; they’re **not** runtime enforcers.

```
```
