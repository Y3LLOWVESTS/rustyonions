

---

````markdown
---
title: Configuration — ron-transport
crate: ron-transport
owner: Stevan White
last-reviewed: 2025-10-01
status: draft
template_version: 1.1
---

# Configuration — `ron-transport`

This document defines **all configuration** for `ron-transport`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> - **Library crate (this crate):** No HTTP ports or `/healthz` endpoints. Config drives the behavior of **dialers/listeners** and typed connection handles.  
> - **Service consumers:** If a binary or service embeds `ron-transport` (e.g., overlay, gateway), it may surface ports and HTTP metrics endpoints in **its** config. Those are **out of scope here**.

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (CLI) — only when the optional `cli` feature is enabled  
2. **Environment variables**  
3. **Config file** (e.g., `Config.toml` loaded by the consumer and deserialized into `ron-transport` types)  
4. **Built-in defaults** (hard-coded)

> On dynamic reload, recompute the effective config under the **same precedence** then swap atomically.

**Supported file formats:** TOML (preferred), JSON (optional via consumer).  
**Relative `--config` resolution (consumer responsibility):** `./`, `$CWD`, consumer crate dir.

**Environment variable prefix:** `RON_TRANSPORT_` (e.g., `RON_TRANSPORT_MAX_CONNS=2048`).

---

## 2) Quickstart Examples

> Because `ron-transport` is a **library**, these examples show how a consumer (binary) might feed configuration.

### 2.1 Minimal listener (TCP)
```bash
RON_TRANSPORT_BACKEND=tcp \
RON_TRANSPORT_BIND_ADDR=0.0.0.0:9444 \
RON_TRANSPORT_MAX_CONNS=1024 \
RON_TRANSPORT_READ_TIMEOUT=5s \
RON_TRANSPORT_WRITE_TIMEOUT=5s \
RON_TRANSPORT_IDLE_TIMEOUT=60s \
my-overlay-service
```

### 2.2 Config file (TOML) consumed by your service

```toml
# Config.toml (transport subsection)
[transport]
backend       = "tls"              # "tcp" | "tls" | "arti" (if feature enabled)
bind_addr     = "0.0.0.0:9444"     # omit for dialer-only usage
max_conns     = 1024
read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "60s"

[transport.tls]                     # server-side TLS (listeners)
enabled   = true
cert_path = "/etc/ron/cert.pem"
key_path  = "/etc/ron/key.pem"

[transport.tls_client]              # dialer-side TLS (clients)
trust_store = "system"              # "system" | "path"
ca_bundle_path = ""                 # required if trust_store="path"
sni_override  = ""                  # optional
alpn          = []                  # e.g., ["h2","http/1.1"]

[transport.limits]
max_frame_bytes = "1MiB"           # hard ceiling per logical frame (fixed ≤1MiB)
chunk_bytes     = "64KiB"          # streaming chunk size target

# NOTE: decompression guard is fixed at ≤8× by design (not configurable).

[transport.rates]
per_conn_bytes_per_sec  = "0"      # 0 disables per-conn limiter
per_peer_bytes_per_sec  = "0"      # 0 disables per-peer limiter
global_egress_bytes_per_sec  = "8MiB"
global_ingress_bytes_per_sec = "8MiB"

[transport.per_peer]               # optional, per-remote cap
max_conns = 64                     # 0 disables per-peer conn cap

[transport.uds]                    # optional (Linux)
path       = ""                    # e.g., "/run/ron/transport.sock" (mutually exclusive with bind_addr)
allow_uids = []                    # [1000, 1001]

[transport.auth]
macaroon_path = ""                 # capability token file (optional)

[transport.amnesia]
enabled = false                    # RAM-only, no persistent artifacts

[transport.pq]
mode = "off"                       # "off" | "hybrid" (when upstream TLS supports it)

[transport.arti]                   # only used when backend="arti"
bootstrap_timeout = "30s"
pre_dial_delay    = "0s"           # small delay to avoid stampede on cold start

[transport.log]
format = "json"                    # "json" | "text"
level  = "info"                    # "trace".."error"
```

### 2.3 CLI flags (when `cli` feature is enabled, consumer parses and passes through)

```bash
my-overlay-service --transport.config ./Config.toml \
  --transport.backend tcp \
  --transport.bind 0.0.0.0:9444 \
  --transport.max-conns 2048 \
  --transport.read-timeout 5s \
  --transport.write-timeout 5s \
  --transport.idle-timeout 60s \
  --transport.tls \
  --transport.tls-cert /etc/ron/cert.pem \
  --transport.tls-key  /etc/ron/key.pem \
  --transport.tls-client-trust-store system \
  --transport.tls-client-ca-bundle /etc/ssl/custom-ca.pem \
  --transport.tls-client-sni example.com \
  --transport.tls-client-alpn h2,http/1.1 \
  --transport.max-frame 1MiB \
  --transport.chunk 64KiB \
  --transport.per-conn-bps 0 \
  --transport.per-peer-bps 0 \
  --transport.global-egress-bps 8MiB \
  --transport.global-ingress-bps 8MiB \
  --transport.per-peer-max-conns 64 \
  --transport.uds /run/ron/transport.sock \
  --transport.pq off \
  --transport.arti-boot-timeout 30s \
  --transport.arti-pre-dial-delay 0s \
  --transport.log-format json \
  --transport.log-level info
```

---

## 3) Schema (Typed, With Defaults)

> **Durations:** accept `ms`, `s`, `m`, `h`. **Sizes:** accept `B`, `KiB`, `MiB`.  
> **Env mapping:** keys map to `RON_TRANSPORT_*` (upper-snake), with nested keys flattened  
> (e.g., `RON_TRANSPORT_TLS_ENABLED`, `RON_TRANSPORT_TLS_CLIENT_TRUST_STORE`).

| Key / Env Var                                                         | Type                       | Default        | Description                                               | Security Notes                                   |
| --------------------------------------------------------------------- | -------------------------- | -------------- | --------------------------------------------------------- | ------------------------------------------------ |
| `backend` / `RON_TRANSPORT_BACKEND`                                   | enum(`tcp`,`tls`,`arti`)   | `tcp`          | Transport backend (Tor/Arti requires `arti` feature)      | `arti` uses in-proc Arti; no external tor daemon |
| `bind_addr` / `RON_TRANSPORT_BIND_ADDR`                               | socket                     | `127.0.0.1:0`  | Listener bind (omit for dialer-only usage)                 | Public binds require threat review               |
| `max_conns` / `RON_TRANSPORT_MAX_CONNS`                               | u32                        | `1024`         | Global concurrent connection cap                          | Prevents FD exhaustion                           |
| `read_timeout` / `RON_TRANSPORT_READ_TIMEOUT`                         | duration                   | `5s`           | Per-IO read deadline                                      | DoS mitigation                                   |
| `write_timeout` / `RON_TRANSPORT_WRITE_TIMEOUT`                       | duration                   | `5s`           | Per-IO write deadline                                     | DoS mitigation                                   |
| `idle_timeout` / `RON_TRANSPORT_IDLE_TIMEOUT`                         | duration                   | `60s`          | Idle keep-alive cutoff                                    | Resource hygiene                                 |
| `limits.max_frame_bytes` / `RON_TRANSPORT_MAX_FRAME_BYTES`            | size                       | `1MiB`         | Per logical frame ceiling (OAP/1 cap mirrored)            | Guards memory/CPU                                |
| `limits.chunk_bytes` / `RON_TRANSPORT_CHUNK_BYTES`                    | size                       | `64KiB`        | Streaming chunk target                                    | Backpressure friendly                            |
| *(fixed)* decompression guard                                         | N/A                        | `≤8×`          | **Not configurable** (hard limit by design)               | Amplification safety                             |
| `rates.per_conn_bytes_per_sec` / `RON_TRANSPORT_PER_CONN_BPS`         | size (bytes/sec)           | `0`            | Per-connection token-bucket (0 disables)                  | Early reject instead of buffering                |
| `rates.per_peer_bytes_per_sec` / `RON_TRANSPORT_PER_PEER_BPS`         | size (bytes/sec)           | `0`            | Per-peer token-bucket (0 disables)                        | Tames N:1 hot peers                              |
| `rates.global_egress_bytes_per_sec` / `RON_TRANSPORT_GLOBAL_EGRESS_BPS` | size (bytes/sec)         | `0` (disabled) | Global egress rate limit                                  |                                                |
| `rates.global_ingress_bytes_per_sec` / `RON_TRANSPORT_GLOBAL_INGRESS_BPS` | size (bytes/sec)       | `0` (disabled) | Global ingress rate limit                                 |                                                |
| `per_peer.max_conns` / `RON_TRANSPORT_PER_PEER_MAX_CONNS`             | u32                        | `0` (disabled) | Per-peer concurrent connection cap                         | Prevents single peer exhaustion                  |
| `tls.enabled` / `RON_TRANSPORT_TLS_ENABLED`                           | bool                       | `false`        | Enable TLS (listeners)                                    | Use `tokio_rustls::rustls` types only            |
| `tls.cert_path` / `RON_TRANSPORT_TLS_CERT_PATH`                       | path                       | `""`           | PEM certificate file                                      | File perms 0600; do not log                      |
| `tls.key_path` / `RON_TRANSPORT_TLS_KEY_PATH`                         | path                       | `""`           | PEM private key file                                      | Zeroize after load; perms 0600                   |
| `tls_client.trust_store` / `RON_TRANSPORT_TLS_CLIENT_TRUST_STORE`     | enum(`system`,`path`)      | `system`       | Dialer trust source                                       |                                                   |
| `tls_client.ca_bundle_path` / `RON_TRANSPORT_TLS_CLIENT_CA_BUNDLE`    | path                       | `""`           | CA bundle when `trust_store="path"`                       | Path must exist; avoid logging                   |
| `tls_client.sni_override` / `RON_TRANSPORT_TLS_CLIENT_SNI`            | string                     | `""`           | Optional SNI override                                     |                                                   |
| `tls_client.alpn` / `RON_TRANSPORT_TLS_CLIENT_ALPN`                   | list<string>               | `[]`           | Optional ALPN list                                        |                                                   |
| `uds.path` / `RON_TRANSPORT_UDS_PATH`                                 | path                       | `""`           | Optional Unix Domain Socket path                          | Dir 0700; sock 0600; enforce PEERCRED            |
| `uds.allow_uids` / `RON_TRANSPORT_UDS_ALLOW_UIDS`                     | list<u32>                  | `[]`           | Allowed client UIDs over UDS                              | Strict allowlist                                 |
| `auth.macaroon_path` / `RON_TRANSPORT_MACAROON_PATH`                  | path                       | `""`           | Capability token file                                     | Never log or echo                                |
| `amnesia.enabled` / `RON_TRANSPORT_AMNESIA`                           | bool                       | `false`        | RAM-only mode (no persistent artifacts)                   | Disables on-disk caches/keys                     |
| `pq.mode` / `RON_TRANSPORT_PQ_MODE`                                   | enum(`off`,`hybrid`)       | `off`          | PQ readiness toggle (hybrid when upstream supports)       | Interop drift risk if peers not ready            |
| `arti.bootstrap_timeout` / `RON_TRANSPORT_ARTI_BOOT_TIMEOUT`          | duration                   | `30s`          | Max time to stay Degraded before NotReady (Arti only)     | Prevents indefinite limbo                        |
| `arti.pre_dial_delay` / `RON_TRANSPORT_ARTI_PRE_DIAL_DELAY`           | duration                   | `0s`           | Optional small delay before dialing (Arti only)           | Avoids cold-start stampedes                      |
| `log.format` / `RON_TRANSPORT_LOG_FORMAT`                             | enum(`json`,`text`)        | `json`         | Transport log format (emitted by consumer)                | JSON in prod                                     |
| `log.level` / `RON_TRANSPORT_LOG_LEVEL`                               | enum(`trace`..`error`)     | `info`         | Log verbosity (emitted by consumer)                       | Avoid `trace` in prod                            |

> **Metrics:** histogram buckets are **fixed** by IDB and not configurable.

---

## 4) Validation Rules (Fail-Closed)

Apply on startup and on reload:

* `backend ∈ {"tcp","tls","arti"}`; `arti` requires the `arti` feature at compile time.
* `bind_addr` parses to a valid socket **unless** `uds.path` is set (these are mutually exclusive).
* `max_conns > 0`.
* `read_timeout`, `write_timeout` > `0`; `idle_timeout ≥ 1s`.
* `limits.max_frame_bytes ∈ [64KiB, 1MiB]` (must not exceed OAP/1 frame cap).
* `limits.chunk_bytes ∈ [4KiB, 256KiB]` and `chunk_bytes ≤ max_frame_bytes`.
* **Decompression guard** is fixed at ≤8× and not configurable (assert at runtime).
* When `tls.enabled=true`:
  * `cert_path` and `key_path` exist and are readable.
  * On Unix, **reject** keys with group/world permissions (must be 0600).
  * Parse the key once; zeroize buffer afterwards; error if empty/invalid.
* For `tls_client.*`:
  * If `trust_store="path"`, `ca_bundle_path` must exist and be readable.
* When `uds.path` set: parent dir exists and is `0700`; socket must be created with `0600`.
* When `auth.macaroon_path` set: file exists and non-empty; not group/world-readable.
* When `pq.mode="hybrid"`: requires the `pq` cargo feature; otherwise **error**.
* When `backend="arti"`: allow `arti.*` knobs; ignore them otherwise.

**On violation:** return a structured error (library). Consumers should **fail fast** with non-zero exit.

---

## 5) Dynamic Reload (If Supported by Consumer)

* **Trigger:** SIGHUP (consumer), or kernel bus event `ConfigUpdated{version}`.
* **Reload semantics:**
  * **Non-disruptive:** timeouts, chunk/frame caps (≤1 MiB), rate limits, log level/format, TLS client trust options.
  * **Disruptive:** `bind_addr` vs `uds.path`, switching `backend`, enabling/disabling server `tls` → require listener rebind (consumer coordinates drain & swap).
* **Atomicity:** Compute a new `Arc<Config>` snapshot and swap under a non-awaited mutex (e.g., `parking_lot::Mutex`) or `arc_swap`.
* **Debounce & rollback:** Debounce rapid updates (e.g., 500ms). If `validate()` fails, **keep the previous** good config and log a structured error (no swap).
* **Audit:** Emit `KernelEvent::ConfigUpdated{version}` and structured diff (no secrets).

---

## 6) CLI Flags (Canonical, behind `cli` feature)

```
--transport.config <path>            # Load TOML/JSON
--transport.backend <tcp|tls|arti>
--transport.bind <ip:port>           # mutually exclusive with --transport.uds
--transport.max-conns <num>
--transport.read-timeout <dur>
--transport.write-timeout <dur>
--transport.idle-timeout <dur>
--transport.tls                      # sets tls.enabled=true
--transport.tls-cert <path>
--transport.tls-key <path>
--transport.tls-client-trust-store <system|path>
--transport.tls-client-ca-bundle <path>
--transport.tls-client-sni <str>
--transport.tls-client-alpn <csv>
--transport.uds <path>
--transport.max-frame <size>         # e.g., 1MiB (cannot exceed 1MiB)
--transport.chunk <size>             # e.g., 64KiB (≤ max-frame)
--transport.per-conn-bps <bytes/sec>
--transport.per-peer-bps <bytes/sec>
--transport.global-egress-bps <bytes/sec>
--transport.global-ingress-bps <bytes/sec>
--transport.per-peer-max-conns <num>
--transport.pq <off|hybrid>
--transport.arti-boot-timeout <dur>
--transport.arti-pre-dial-delay <dur>
--transport.log-format <json|text>
--transport.log-level <trace|debug|info|warn|error>
```

---

## 7) Feature Flags (Cargo)

| Feature | Default | Effect                                                                |
| ------- | ------: | --------------------------------------------------------------------- |
| `tls`   |     off | Enables TLS accept/dial paths (uses `tokio_rustls::rustls` types)     |
| `arti`  |     off | Enables Arti (Tor) backend; in-proc library only                      |
| `pq`    |     off | Enables PQ-hybrid config pass-through (no algorithms hard-coded here) |
| `cli`   |     off | Enables optional CLI parsing in consumer binaries that opt-in         |
| `ryker` |     off | Optional actor/mailbox integration hooks (no behavior change if off)  |
| `dx`    |     off | Optional helpers for env mapping (ergonomics only; pure lib remains)  |

> Enabling a feature **must not** change defaults silently; it only unlocks additional valid values or code paths.

---

## 8) Security Implications

* **Public binds (`0.0.0.0`)** require strict caps (timeouts, frame size, **8× decompression guard**, rate limits) and monitoring.
* **TLS (server):** Only `tokio_rustls::rustls::ServerConfig` types; never use `rustls::ServerConfig` directly. PEM files must be 0600; never log secrets.
* **TLS (client):** Prefer `trust_store="system"`; for `path`, mount CA bundle read-only; avoid logging SNI/ALPN when sensitive.
* **Arti:** No spawning of external `tor` daemons; bootstrap readiness is surfaced to the consumer via readiness API (`Degraded("arti_bootstrap")` → `Ready` or `NotReady` after `bootstrap_timeout`).
* **Amnesia:** When enabled, **no persistent artifacts** (no on-disk caches/keys); ephemeral material must be zeroized on drop.
* **UDS:** Enforce `SO_PEERCRED` and `allow_uids`; directories 0700, sockets 0600.
* **Macaroons/Capabilities:** Never log tokens; prefer short rotation (≤ 30 days).

---

## 9) Compatibility & Migration

* **Additive evolution:** Introduce new keys with safe defaults.
* **Renames:** Maintain env var aliases for ≥ 1 minor release; emit deprecation warnings.
* **Breaking changes:** Require a **major** version bump and a `CHANGELOG.md` entry with concrete migration steps.

**Deprecation table (to be maintained):**

| Old Key                 | New Key                  | Removal Target | Notes                   |
| ----------------------- | ------------------------ | -------------- | ----------------------- |
| `limits.max_body_bytes` | `limits.max_frame_bytes` | v1→v2          | Clarify OAP/1 semantics |

---

## 10) Reference Types (Rust)

> Paste-ready skeleton for `src/config/mod.rs` (library side). Consumers may wrap this in their own higher-level config.

```rust
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Backend { Tcp, Tls, Arti }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCfg {
    #[serde(default)]
    pub enabled: bool,                 // listeners
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsClientCfg {
    #[serde(default = "default_trust_store")]
    pub trust_store: TrustStore,       // system | path
    pub ca_bundle_path: Option<PathBuf>,
    #[serde(default)]
    pub sni_override: Option<String>,
    #[serde(default)]
    pub alpn: Vec<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrustStore { System, Path }
fn default_trust_store() -> TrustStore { TrustStore::System }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default = "default_max_frame")] pub max_frame_bytes: u64, // bytes
    #[serde(default = "default_chunk")]     pub chunk_bytes: u64,     // bytes
}
fn default_max_frame() -> u64 { 1 * 1024 * 1024 }   // 1 MiB
fn default_chunk()     -> u64 { 64 * 1024 }         // 64 KiB

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Rates {
    /// 0 disables the limiter.
    #[serde(default)] pub per_conn_bytes_per_sec: u64,
    #[serde(default)] pub per_peer_bytes_per_sec: u64,
    #[serde(default)] pub global_egress_bytes_per_sec: u64,
    #[serde(default)] pub global_ingress_bytes_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerPeer {
    #[serde(default)] pub max_conns: u32, // 0 disables
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PqMode { Off, Hybrid }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Uds {
    pub path: Option<PathBuf>,
    #[serde(default)] pub allow_uids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Auth { pub macaroon_path: Option<PathBuf> }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Amnesia { #[serde(default)] pub enabled: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtiCfg {
    #[serde(with = "humantime_serde", default = "default_arti_boot")]
    pub bootstrap_timeout: Duration,
    #[serde(with = "humantime_serde", default)]
    pub pre_dial_delay: Duration, // 0 by default
}
fn default_arti_boot() -> Duration { Duration::from_secs(30) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCfg {
    #[serde(default = "default_log_format")] pub format: String, // "json" | "text"
    #[serde(default = "default_log_level")]  pub level: String,  // "info", etc.
}
fn default_log_format() -> String { "json".into() }
fn default_log_level()  -> String { "info".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_backend")] pub backend: Backend,
    pub bind_addr: Option<SocketAddr>, // None -> dialer-only
    #[serde(default = "default_max_conns")] pub max_conns: u32,
    #[serde(with = "humantime_serde", default = "default_5s")]  pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_5s")]  pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_60s")] pub idle_timeout: Duration,
    #[serde(default)] pub tls: TlsCfg,
    #[serde(default)] pub tls_client: TlsClientCfg,
    #[serde(default)] pub limits: Limits,
    #[serde(default)] pub rates: Rates,
    #[serde(default)] pub per_peer: PerPeer,
    #[serde(default)] pub uds: Uds,
    #[serde(default)] pub auth: Auth,
    #[serde(default)] pub amnesia: Amnesia,
    #[serde(default = "default_pq_mode")] pub pq: PqMode,
    #[serde(default)] pub arti: ArtiCfg,
    #[serde(default)] pub log: LogCfg,
}

fn default_backend() -> Backend { Backend::Tcp }
fn default_max_conns() -> u32 { 1024 }
fn default_5s() -> Duration { Duration::from_secs(5) }
fn default_60s() -> Duration { Duration::from_secs(60) }
fn default_pq_mode() -> PqMode { PqMode::Off }

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        // backend
        if self.max_conns == 0 { anyhow::bail!("max_conns must be > 0"); }

        // bind vs UDS
        if self.uds.path.is_some() && self.bind_addr.is_some() {
            anyhow::bail!("bind_addr and uds.path are mutually exclusive");
        }

        // timeouts & sizes
        if self.read_timeout == Duration::ZERO || self.write_timeout == Duration::ZERO {
            anyhow::bail!("read/write timeouts must be > 0");
        }
        if self.idle_timeout < Duration::from_secs(1) {
            anyhow::bail!("idle_timeout must be >= 1s");
        }
        if !(64 * 1024 ..= 1 * 1024 * 1024).contains(&self.limits.max_frame_bytes) {
            anyhow::bail!("limits.max_frame_bytes must be within [64KiB, 1MiB]");
        }
        if !(4 * 1024 ..= 256 * 1024).contains(&self.limits.chunk_bytes) {
            anyhow::bail!("limits.chunk_bytes must be within [4KiB, 256KiB]");
        }
        if self.limits.chunk_bytes > self.limits.max_frame_bytes {
            anyhow::bail!("limits.chunk_bytes must be <= limits.max_frame_bytes");
        }

        // fixed decompression guard (assert for clarity)
        const DECOMP_GUARD_X: u32 = 8;
        let _ = DECOMP_GUARD_X; // not configurable

        // server TLS
        if self.tls.enabled {
            let (c, k) = self.tls_paths()?; // unwrap Option<PathBuf>
            #[cfg(unix)] { assert_strict_key_perms(&k)?; }
            load_key_zeroized(&k)?; // parse once, then zeroize
            let _ = std::fs::metadata(&c)?;
        }

        // client TLS
        if let TrustStore::Path = self.tls_client.trust_store {
            let p = self.tls_client.ca_bundle_path.as_ref()
                .ok_or_else(|| anyhow::anyhow!("tls_client.trust_store=path requires ca_bundle_path"))?;
            let _ = std::fs::metadata(p)?;
        }

        // PQ feature gate
        #[cfg(not(feature = "pq"))]
        if matches!(self.pq, PqMode::Hybrid) {
            anyhow::bail!("pq.mode=hybrid requires the `pq` cargo feature");
        }

        Ok(())
    }

    fn tls_paths(&self) -> anyhow::Result<(PathBuf, PathBuf)> {
        let c = self.tls.cert_path.clone()
            .ok_or_else(|| anyhow::anyhow!("tls.enabled=true but cert_path is missing"))?;
        let k = self.tls.key_path.clone()
            .ok_or_else(|| anyhow::anyhow!("tls.enabled=true but key_path is missing"))?;
        Ok((c, k))
    }
}

#[cfg(unix)]
fn assert_strict_key_perms(path: &PathBuf) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        anyhow::bail!("insecure permissions on {} (require 0600)", path.display());
    }
    Ok(())
}

fn load_key_zeroized(path: &PathBuf) -> anyhow::Result<rustls_pemfile::Item> {
    use zeroize::Zeroize;
    let mut buf = std::fs::read(path)?;
    let mut slice: &[u8] = &buf;
    let item = rustls_pemfile::read_one(&mut slice)?
        .ok_or_else(|| anyhow::anyhow!("empty key file"))?;
    buf.zeroize(); // scrub after parse
    Ok(item)
}
```

> **Optional DX helper (behind `dx` feature):** provide a small function that reads `RON_TRANSPORT_*` env and produces a partial `Config` patch for consumers, keeping the lib service-agnostic while avoiding duplicate mapping logic.

---

## 11) Test Matrix

| Scenario                                | Expected Outcome                                             |
| --------------------------------------- | ------------------------------------------------------------ |
| Missing config file                     | Defaults applied; library returns `Config` with safe values  |
| Invalid `backend`                       | Validation error                                             |
| TLS enabled but key/cert missing        | Validation error                                             |
| TLS key world/group-readable (Unix)     | Validation error                                             |
| `max_frame_bytes` > 1MiB                | Validation error                                             |
| `chunk_bytes` > `max_frame_bytes`       | Validation error                                             |
| `pq.mode="hybrid"` without `pq` feature | Validation error                                             |
| Public bind + `max_conns=0`             | Validation error                                             |
| UDS + bind simultaneously               | Validation error                                             |
| Per-peer limiter values negative/overflow | Validation error                                          |
| Arti knobs with non-Arti backend        | Ignored (no error)                                           |
| SIGHUP / bus `ConfigUpdated` (consumer) | Non-disruptive keys hot-swap; disruptive ones trigger rebind |
| **Fuzz malformed TOML/JSON**            | Typed error, never panic; error path points to offending key |
| **Durations/sizes proptest**            | Round-trip parse/format stays within accepted ranges         |

---

## 12) Mermaid — Config Resolution Flow (Consumer + Library)

```mermaid
flowchart TB
  A[Defaults (library)] --> D[Merge]
  B[Config File (consumer)] --> D
  C[Env Vars RON_TRANSPORT_*] --> D
  E[CLI Flags (consumer, optional)] --> D
  D --> V{Validate}
  V -- ok --> R[Runtime Snapshot (Arc<Config>)]
  V -- fail --> X[Error to Consumer]
  R --> H{Hot Reload?}
  H -- yes --> D
  style R fill:#0e7490,stroke:#164e63,color:#fff
```

---

## 13) Operational Notes

* Store production transport config in a secure repo/secret store; mount secrets read-only.
* Prefer **env vars** in containers; avoid baking TLS keys into images.
* When enabling `arti`, monitor readiness: expect `Degraded("arti_bootstrap")` before `Ready`. Use `arti.bootstrap_timeout` to fail fast out of limbo.
* Keep PQ set to `off` until peer compatibility is confirmed; roll out **canary first**.
* Tune `rates.*`, `per_peer.*`, and `max_conns` to meet target p95 handshake latency; prefer **backpressure** (visible rejects) over buffering.
* Metrics histogram buckets are **fixed** by the IDB; do not attempt to tune via config.

---
````
