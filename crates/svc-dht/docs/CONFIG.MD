````markdown
---
title: Configuration — svc-dht
crate: svc-dht
owner: Stevan White
last-reviewed: 2025-10-10
status: draft
template_version: 1.1
---

# Configuration — svc-dht

This document defines **all configuration** for `svc-dht`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/SECURITY.md`, and the IDB.

> **Tiering:** Service crate — all sections apply (network, readiness, observability, etc.).

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (e.g., `Config.toml` beside the binary or passed via `--config`)  
4. **Built-in defaults** (hard-coded in `src/config.rs`)

When reloading dynamically, the effective config is recomputed under the same precedence.

**Supported file formats:** TOML (preferred), JSON (optional).  
**Path resolution order for `--config` (if relative):** `./`, `$CWD`, crate dir.  
**Environment variable prefix:** `SVC_DHT_` (e.g., `SVC_DHT_BIND_ADDR`).

---

## 2) Quickstart Examples

### 2.1 Minimal service start

```bash
RUST_LOG=info \
SVC_DHT_BIND_ADDR=0.0.0.0:8080 \
SVC_DHT_METRICS_ADDR=127.0.0.1:0 \
cargo run -p svc-dht
````

### 2.2 Config file (TOML) — recommended baseline

```toml
# Config.toml
bind_addr     = "0.0.0.0:8080"
metrics_addr  = "127.0.0.1:0"
max_conns     = 1024
read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "60s"

[transport]
read_timeout     = "1500ms"
write_timeout    = "1500ms"
idle_timeout     = "60s"
max_frame_bytes  = "1MiB"

[tls]
enabled = false
# cert_path = "/etc/ron/cert.pem"
# key_path  = "/etc/ron/key.pem"

[limits]
max_body_bytes       = "1MiB"
decompress_ratio_cap = 10
max_inflight         = 512
max_rps              = 500

[dht]
alpha       = 3
beta        = 2
k           = 20
hop_budget  = 5
rpc_timeout = "1500ms"
hedge_after = "250ms"

[provider]
ttl_secs         = 86400    # 24h
refresh_secs     = 43200    # 12h
max_record_bytes = "16KiB"

[bootstrap]
required              = 3
ready_bucket_fill_pct = 60
seeds = [
  "node1.ron.example:6891",
  "node2.ron.example:6891"
]

[diversity]
asn_max_pct     = 40
asn_entropy_min = 1.5

[nat]
happy_eyeballs_stagger = "75ms"

[pq]
dual_sign     = true
require_pq_on = "2026-06-01"
algorithms    = ["ed25519","mldsa87"]

[amnesia]
enabled = false

[log]
format = "json"         # json|text
level  = "info"         # trace|debug|info|warn|error
```

### 2.3 CLI flags (override file/env)

```bash
cargo run -p svc-dht -- \
  --bind 0.0.0.0:8080 \
  --metrics 127.0.0.1:0 \
  --max-conns 2048 \
  --alpha 4 --beta 2 --k 20 --hop-budget 5 \
  --rpc-timeout 1500ms --hedge-after 250ms \
  --bootstrap-required 3 --ready-bucket-fill 60 \
  --bootstrap-seed node1.ron.example:6891 \
  --bootstrap-seed node2.ron.example:6891 \
  --asn-max-pct 40 --asn-entropy-min 1.5 \
  --pq-dual-sign --pq-require-pq-on 2026-06-01 \
  --profile micronode
```

### 2.4 Docker Compose (example)

```yaml
version: "3.9"
services:
  svc-dht:
    image: ghcr.io/rustyonions/svc-dht:latest
    environment:
      RUST_LOG: info
      SVC_DHT_BIND_ADDR: 0.0.0.0:8080
      SVC_DHT_METRICS_ADDR: 0.0.0.0:9090
      SVC_DHT_ALPHA: 3
      SVC_DHT_PQ_DUAL_SIGN: "true"
    ports:
      - "8080:8080"
      - "9090:9090"
    read_only: true
    tmpfs:
      - /tmp:size=32m,mode=1777
```

---

## 3) Schema (Typed, With Defaults)

> **Durations** accept `ms`, `s`, `m`, `h`. **Sizes** accept `B`, `KB`, `MB`, `MiB`, `GiB`.

| Key / Env Var                                                   | Type                          |                 Default | Description                        | Security Notes                              |
| --------------------------------------------------------------- | ----------------------------- | ----------------------: | ---------------------------------- | ------------------------------------------- |
| `bind_addr` / `SVC_DHT_BIND_ADDR`                               | socket                        |           `127.0.0.1:0` | HTTP/ingress bind address          | Public binds require firewall/threat review |
| `metrics_addr` / `SVC_DHT_METRICS_ADDR`                         | socket                        |           `127.0.0.1:0` | Prometheus endpoint bind           | Prefer localhost in prod; gateway scrape    |
| `max_conns` / `SVC_DHT_MAX_CONNS`                               | u32                           |                  `1024` | Max concurrent connections         | Prevents FD exhaustion                      |
| `read_timeout` / `SVC_DHT_READ_TIMEOUT`                         | duration                      |                    `5s` | Per-request read timeout           | DoS mitigation                              |
| `write_timeout` / `SVC_DHT_WRITE_TIMEOUT`                       | duration                      |                    `5s` | Per-request write timeout          | DoS mitigation                              |
| `idle_timeout` / `SVC_DHT_IDLE_TIMEOUT`                         | duration                      |                   `60s` | Keep-alive idle shutdown           | Resource hygiene                            |
| `transport.read_timeout` / `SVC_DHT_T_READ_TIMEOUT`             | duration                      |                `1500ms` | Socket read timeout                | Lower than HTTP caps                        |
| `transport.write_timeout` / `SVC_DHT_T_WRITE_TIMEOUT`           | duration                      |                `1500ms` | Socket write timeout               |                                             |
| `transport.idle_timeout` / `SVC_DHT_T_IDLE_TIMEOUT`             | duration                      |                   `60s` | Socket idle                        |                                             |
| `transport.max_frame_bytes` / `SVC_DHT_T_MAX_FRAME_BYTES`       | size                          |                  `1MiB` | Max framed payload                 | Guard against bloat                         |
| `limits.max_body_bytes` / `SVC_DHT_MAX_BODY_BYTES`              | size                          |                  `1MiB` | Request payload cap                | Decompression bomb guard                    |
| `limits.decompress_ratio_cap` / `SVC_DHT_DECOMPRESS_RATIO_CAP`  | u32                           |                    `10` | Max allowed decompression ratio    | Zip bomb guard                              |
| `limits.max_inflight` / `SVC_DHT_MAX_INFLIGHT`                  | u32                           |                   `512` | Global inflight RPC ceiling        | Backpressure                                |
| `limits.max_rps` / `SVC_DHT_MAX_RPS`                            | u32                           |                   `500` | Global RPS ceiling                 | Backpressure                                |
| `tls.enabled` / `SVC_DHT_TLS_ENABLED`                           | bool                          |                 `false` | Enable TLS (tokio-rustls)          | Use tokio-rustls only                       |
| `tls.cert_path` / `SVC_DHT_TLS_CERT_PATH`                       | path                          |                    `""` | PEM cert path                      | Secrets on disk; perms 0600                 |
| `tls.key_path` / `SVC_DHT_TLS_KEY_PATH`                         | path                          |                    `""` | PEM key path                       | Zeroize in memory                           |
| `uds.path` / `SVC_DHT_UDS_PATH`                                 | path                          |                    `""` | Unix Domain Socket path (optional) | Dir 0700, sock 0600                         |
| `uds.allow_uids` / `SVC_DHT_UDS_ALLOW_UIDS`                     | list<u32>                     |                    `[]` | PEERCRED allowlist                 | Strict production control                   |
| `dht.alpha` / `SVC_DHT_ALPHA`                                   | u8                            |                     `3` | Parallelism for queries            | Bounded concurrency                         |
| `dht.beta` / `SVC_DHT_BETA`                                     | u8                            |                     `2` | Value-lookup concurrency           | Bounded concurrency                         |
| `dht.k` / `SVC_DHT_K`                                           | u8                            |                    `20` | Kademlia bucket size               | Must match sims                             |
| `dht.hop_budget` / `SVC_DHT_HOP_BUDGET`                         | u8                            |                     `5` | Max hops per lookup                | SLO p99 ≤ 5                                 |
| `dht.rpc_timeout` / `SVC_DHT_RPC_TIMEOUT`                       | duration                      |                `1500ms` | Per-RPC deadline                   | Cancel losers promptly                      |
| `dht.hedge_after` / `SVC_DHT_HEDGE_AFTER`                       | duration                      |                 `250ms` | Hedge delay to spawn extra dial    | Tail latency control                        |
| `bootstrap.required` / `SVC_DHT_BOOTSTRAP_REQUIRED`             | u8                            |                     `3` | Seeds that must be reachable       | Gate `/readyz`                              |
| `bootstrap.ready_bucket_fill_pct` / `SVC_DHT_READY_BUCKET_FILL` | u8                            |                    `60` | % buckets non-empty to be ready    | Gate `/readyz`                              |
| `bootstrap.seeds` / `SVC_DHT_BOOTSTRAP_SEEDS`                   | list<string>                  |                    `[]` | Seed addresses/hostnames           | Prefer diverse ASNs/regions                 |
| `provider.ttl_secs` / `SVC_DHT_PROVIDER_TTL_SECS`               | u32                           |                 `86400` | Record TTL (seconds)               | Expiring, republished                       |
| `provider.refresh_secs` / `SVC_DHT_PROVIDER_REFRESH_SECS`       | u32                           |                 `43200` | Republish interval                 |                                             |
| `provider.max_record_bytes` / `SVC_DHT_PROVIDER_MAX_BYTES`      | size                          |                 `16KiB` | Max provider record size           | Reject oversize                             |
| `diversity.asn_max_pct` / `SVC_DHT_ASN_MAX_PCT`                 | u8                            |                    `40` | Max % per ASN per bucket           | Anti-eclipse                                |
| `diversity.asn_entropy_min` / `SVC_DHT_ASN_ENTROPY_MIN`         | f32                           |                   `1.5` | Shannon entropy floor per bucket   | Anti-eclipse                                |
| `nat.happy_eyeballs_stagger` / `SVC_DHT_HE_STAGGER`             | duration                      |                  `75ms` | v4/v6 race stagger                 | Dual-stack UX                               |
| `pq.dual_sign` / `SVC_DHT_PQ_DUAL_SIGN`                         | bool                          |                  `true` | Dual-sign envelope enabled         | PQ migration                                |
| `pq.require_pq_on` / `SVC_DHT_PQ_REQUIRE_ON`                    | date(YYYY-MM-DD)              |          `"2026-06-01"` | Warn on classical-only after date  | Interop window                              |
| `pq.algorithms` / `SVC_DHT_PQ_ALGS`                             | list<string>                  | `["ed25519","mldsa87"]` | Allowed signature algs             | KMS-backed                                  |
| `amnesia.enabled` / `SVC_DHT_AMNESIA`                           | bool                          |                 `false` | RAM-only mode (no disk)            | No persistence                              |
| `log.format` / `SVC_DHT_LOG_FORMAT`                             | enum(`json`,`text`)           |                  `json` | Structured logs                    | JSON required in prod                       |
| `log.level` / `SVC_DHT_LOG_LEVEL`                               | enum                          |                  `info` | `trace`..`error`                   | Avoid `trace` in prod                       |
| `profile` / `SVC_DHT_PROFILE`                                   | enum(`micronode`,`macronode`) |                  `null` | Tiered overrides (see §14)         |                                             |

**Diversity entropy formula (Shannon H):**
For each bucket, let pᵢ be the fraction of entries from ASN i.
H = −Σ pᵢ · log₂(pᵢ). The config `diversity.asn_entropy_min` enforces a floor on H.

---

## 4) Validation Rules (Fail-Closed)

On startup or reload, apply **strict validation**:

* `bind_addr`, `metrics_addr` parse to valid sockets; ports <1024 require privileges.
* If `tls.enabled=true`, `cert_path` and `key_path` must exist, be readable, and not world-readable; zeroize key after load.
* `max_conns` > 0; `limits.max_body_bytes` ≥ 1 KiB; `decompress_ratio_cap` ≥ 1.
* `limits.max_inflight` ≥ `alpha + beta`; `limits.max_rps` ≥ `alpha * 50` (sane floor).
* `dht.hop_budget` ≥ 1 and ≤ 32; `alpha` ≥ 1; `k` ∈ [16, 32] (must match sims).
* `bootstrap.required` ≥ 1 if `bootstrap.seeds` non-empty; fqdn/host:port parseable.
* `diversity.asn_max_pct` in [10, 100]; `diversity.asn_entropy_min` in [0.0, 8.0].
* `provider.max_record_bytes` ≥ 1 KiB and ≤ 64 KiB.
* If `amnesia.enabled=true`: any disk-backed provider store is **refused**.
* For `pq.dual_sign=true`: `pq.algorithms` must include a classical alg (`ed25519`) and a PQ alg (`mldsa87`).
* `pq.require_pq_on` must parse as `YYYY-MM-DD` and be **strictly greater** than the current date.

**On violation:** log structured error, **exit non-zero** (service) or **return error** (tests).
**Post-reload:** re-run `validate()`; if invalid, **keep the old config**, emit WARN with reasons.

---

## 5) Dynamic Reload (If Supported)

**Triggers:** SIGHUP (preferred), or bus `KernelEvent::ConfigUpdated { version }`.

**Reload semantics:**

* **Debounce** incoming reload triggers by **5s** (latest wins).
* **Non-disruptive:** timeouts, limits, log level/format, DHT α/β/k/hop_budget, diversity thresholds, PQ flags.
* **Disruptive (socket rebind):** `bind_addr`, `metrics_addr`, `tls.*`, `uds.*`.

**Atomicity:** Build a new config snapshot; run `validate()`; on success, swap under a short critical section (no `.await` held).
**Audit:** Emit `KernelEvent::ConfigUpdated { version, failed: bool }`; log a **redacted** diff (paths/counts; never secrets).

---

## 6) CLI Flags (Canonical)

```
--config <path>                   # Load Config.{toml,json}
--bind <ip:port>                  # bind_addr
--metrics <ip:port>               # metrics_addr
--max-conns <num>
--read-timeout <dur>              # 5s, 250ms
--write-timeout <dur>
--idle-timeout <dur>

--t-read-timeout <dur>            # transport.read_timeout
--t-write-timeout <dur>
--t-idle-timeout <dur>
--t-max-frame <size>              # e.g., 1MiB

--alpha <n> --beta <n> --k <n> --hop-budget <n>
--rpc-timeout <dur> --hedge-after <dur>

--bootstrap-required <n>
--ready-bucket-fill <pct>
--bootstrap-seed <host:port>      # repeatable

--provider-ttl <secs> --provider-refresh <secs>
--provider-max-bytes <size>

--asn-max-pct <pct> --asn-entropy-min <bits>
--he-stagger <dur>

--pq-dual-sign | --no-pq-dual-sign
--pq-require-pq-on <YYYY-MM-DD>
--pq-alg <name>                   # repeatable; ed25519|mldsa87

--amnesia                         # sets amnesia.enabled=true
--profile <micronode|macronode>   # tiered overrides (§14)
--log-format <json|text>
--log-level <trace|debug|info|warn|error>
--help
```

---

## 7) Feature Flags (Cargo)

| Feature |  Default | Effect                                                          |
| ------- | -------: | --------------------------------------------------------------- |
| `tls`   |      off | Enables tokio-rustls integration and activates `tls.*` settings |
| `quic`  |      off | Enables QUIC transports via `ron-transport` adapters            |
| `arti`  |      off | Enables Tor routing via `ron-transport` adapters                |
| `pq`    |      off | Enables PQ envelope and `pq.*` config                           |
| `cli`   | on (bin) | Enables CLI parsing for flags above                             |
| `kameo` |      off | Optional actor integration                                      |

> Turning features on **does not** change defaults; it only enables code paths & config keys.

---

## 8) Security Implications

* **Public binds** (`0.0.0.0`) require sane caps (timeouts, body size, inflight, RPS) and upstream firewalling.
* **TLS**: only `tokio_rustls::rustls::ServerConfig`; never `rustls::ServerConfig` directly.
* **PQ**: dual-sign window should be time-bounded; alert if `pq_dual_sign_ratio < 0.95` during migration.
* **Amnesia mode**: disables disk-backed stores; ensure provider-store impl respects this (RAM-only).
* **UDS**: enforce `SO_PEERCRED`; gate access via `uds.allow_uids`.
* **Diversity knobs**: loosening `asn_max_pct` or `asn_entropy_min` weakens eclipse resistance; changes must be reviewed.
* **Seeds**: rotate quarterly; ensure ASN/geo diversity.

See `docs/SECURITY.md` for the full threat model.

---

## 9) Compatibility & Migration

* **Backward compatible** changes add keys with safe defaults.
* **Renames** keep env-var aliases ≥ 1 minor; warn on use.
* **Breaking** config changes require a **major** bump and a `CHANGELOG.md` entry with migration steps.

*Deprecation table (maintained):*

| Old Key             | New Key                     | Removal Target | Notes                |
| ------------------- | --------------------------- | -------------: | -------------------- |
| `provider.max_size` | `provider.max_record_bytes` |       vA+1.0.0 | Size units clarified |

---

## 10) Reference Implementation (Rust)

> Minimal `Config` with file + env + CLI (serde). Comments included; paste into `src/config.rs`.

```rust
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsCfg {
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCfg {
    #[serde(with = "humantime_serde", default = "ms1500")]
    pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "ms1500")]
    pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "s60")]
    pub idle_timeout: Duration,
    #[serde(default = "one_mib")]
    pub max_frame_bytes: u64, // bytes
}
impl Default for TransportCfg {
    fn default() -> Self {
        Self { read_timeout: ms1500(), write_timeout: ms1500(), idle_timeout: s60(), max_frame_bytes: one_mib() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default = "one_mib")]
    pub max_body_bytes: u64, // bytes
    #[serde(default = "ten")]
    pub decompress_ratio_cap: u32,
    #[serde(default = "five_hundred")]
    pub max_rps: u32,
    #[serde(default = "five_hundred_twelve")]
    pub max_inflight: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self { max_body_bytes: one_mib(), decompress_ratio_cap: ten(), max_rps: five_hundred(), max_inflight: five_hundred_twelve() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtTuning {
    #[serde(default = "three_u8")]
    pub alpha: u8,
    #[serde(default = "two_u8")]
    pub beta: u8,
    #[serde(default = "twenty_u8")]
    pub k: u8,
    #[serde(default = "five_u8")]
    pub hop_budget: u8,
    #[serde(with = "humantime_serde", default = "ms1500")]
    pub rpc_timeout: Duration,
    #[serde(with = "humantime_serde", default = "ms250")]
    pub hedge_after: Duration,
}
impl Default for DhtTuning {
    fn default() -> Self {
        Self {
            alpha: three_u8(), beta: two_u8(), k: twenty_u8(), hop_budget: five_u8(),
            rpc_timeout: ms1500(), hedge_after: ms250()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCfg {
    #[serde(default = "secs_86400")]
    pub ttl_secs: u32,
    #[serde(default = "secs_43200")]
    pub refresh_secs: u32,
    #[serde(default = "sixteen_kib")]
    pub max_record_bytes: u64, // bytes
}
impl Default for ProviderCfg {
    fn default() -> Self {
        Self { ttl_secs: secs_86400(), refresh_secs: secs_43200(), max_record_bytes: sixteen_kib() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapCfg {
    #[serde(default = "three_u8")]
    pub required: u8,
    #[serde(default = "sixty_u8")]
    pub ready_bucket_fill_pct: u8,
    #[serde(default)]
    pub seeds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityCfg {
    #[serde(default = "forty_u8")]
    pub asn_max_pct: u8,
    #[serde(default = "entropy_1_5")]
    pub asn_entropy_min: f32,
}
impl Default for DiversityCfg {
    fn default() -> Self { Self { asn_max_pct: forty_u8(), asn_entropy_min: entropy_1_5() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatCfg {
    #[serde(with = "humantime_serde", default = "ms75")]
    pub happy_eyeballs_stagger: Duration,
}
impl Default for NatCfg {
    fn default() -> Self { Self { happy_eyeballs_stagger: ms75() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqCfg {
    #[serde(default = "true_val")]
    pub dual_sign: bool,
    #[serde(default = "default_require_on")]
    pub require_pq_on: String, // YYYY-MM-DD
    #[serde(default = "default_algs")]
    pub algorithms: Vec<String>, // ["ed25519","mldsa87"]
}
impl Default for PqCfg {
    fn default() -> Self { Self { dual_sign: true_val(), require_pq_on: default_require_on(), algorithms: default_algs() } }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AmnesiaCfg {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCfg {
    #[serde(default = "json_fmt")]
    pub format: String, // "json" or "text"
    #[serde(default = "info_lvl")]
    pub level: String,  // "trace".."error"
}
impl Default for LogCfg {
    fn default() -> Self { Self { format: json_fmt(), level: info_lvl() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Core sockets
    pub bind_addr: Option<SocketAddr>,      // None => 127.0.0.1:0
    pub metrics_addr: Option<SocketAddr>,   // None => 127.0.0.1:0
    #[serde(default = "one_zero_two_four")]
    pub max_conns: u32,
    #[serde(with = "humantime_serde", default = "s5")]
    pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "s5")]
    pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "s60")]
    pub idle_timeout: Duration,

    // Subsystems
    #[serde(default)]
    pub transport: TransportCfg,
    #[serde(default)]
    pub tls: TlsCfg,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub dht: DhtTuning,
    #[serde(default)]
    pub provider: ProviderCfg,
    #[serde(default)]
    pub bootstrap: BootstrapCfg,
    #[serde(default)]
    pub diversity: DiversityCfg,
    #[serde(default)]
    pub nat: NatCfg,
    #[serde(default)]
    pub pq: PqCfg,
    #[serde(default)]
    pub amnesia: AmnesiaCfg,
    #[serde(default)]
    pub log: LogCfg,

    // NEW: profile
    #[serde(default)]
    pub profile: Option<String>, // "micronode" | "macronode"
}

fn one_mib() -> u64 { 1 * 1024 * 1024 }
fn sixteen_kib() -> u64 { 16 * 1024 }
fn five_u8() -> u8 { 5 }
fn three_u8() -> u8 { 3 }
fn two_u8() -> u8 { 2 }
fn twenty_u8() -> u8 { 20 }
fn sixty_u8() -> u8 { 60 }
fn forty_u8() -> u8 { 40 }
fn entropy_1_5() -> f32 { 1.5 }
fn one_zero_two_four() -> u32 { 1024 }
fn five_hundred() -> u32 { 500 }
fn five_hundred_twelve() -> u32 { 512 }
fn s5() -> Duration { Duration::from_secs(5) }
fn s60() -> Duration { Duration::from_secs(60) }
fn ms1500() -> Duration { Duration::from_millis(1500) }
fn ms250() -> Duration { Duration::from_millis(250) }
fn ms75() -> Duration { Duration::from_millis(75) }
fn secs_86400() -> u32 { 86_400 }
fn secs_43200() -> u32 { 43_200 }
fn json_fmt() -> String { "json".into() }
fn info_lvl() -> String { "info".into() }
fn true_val() -> bool { true }
fn default_require_on() -> String { "2026-06-01".into() }
fn default_algs() -> Vec<String> { vec!["ed25519".into(), "mldsa87".into()] }
fn ten() -> u32 { 10 }

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_conns == 0 { anyhow::bail!("max_conns must be > 0"); }
        if self.limits.max_body_bytes < 1024 { anyhow::bail!("max_body_bytes too small"); }
        if self.limits.decompress_ratio_cap == 0 { anyhow::bail!("decompress_ratio_cap must be >= 1"); }
        if self.limits.max_inflight < (self.dht.alpha as u32 + self.dht.beta as u32) {
            anyhow::bail!("max_inflight must be >= alpha + beta");
        }
        if !(16..=32).contains(&self.dht.k) { anyhow::bail!("k must be in [16,32]"); }
        if self.dht.hop_budget == 0 { anyhow::bail!("hop_budget must be >= 1"); }
        if self.tls.enabled {
            match (&self.tls.cert_path, &self.tls.key_path) {
                (Some(c), Some(k)) if c.exists() && k.exists() => {},
                _ => anyhow::bail!("TLS enabled but cert/key missing"),
            }
        }
        // PQ checks
        if self.pq.dual_sign {
            let has_classical = self.pq.algorithms.iter().any(|a| a == "ed25519");
            let has_pq = self.pq.algorithms.iter().any(|a| a == "mldsa87");
            if !(has_classical && has_pq) {
                anyhow::bail!("pq.algorithms must include ed25519 and mldsa87 when dual_sign=true");
            }
        }
        // pq.require_pq_on must be future date
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&self.pq.require_pq_on, "%Y-%m-%d") {
            let today = chrono::Utc::now().date_naive();
            if date <= today {
                anyhow::bail!("pq.require_pq_on must be a future date");
            }
        } else {
            anyhow::bail!("pq.require_pq_on must be YYYY-MM-DD");
        }
        Ok(())
    }
}

// Helper to apply reloads atomically with validation
pub fn try_apply_reload(current: &Config, proposed: Config) -> anyhow::Result<Config> {
    if let Err(e) = proposed.validate() {
        tracing::warn!(error=%e, "config reload rejected; keeping previous snapshot");
        anyhow::bail!(e);
    }
    Ok(proposed)
}
```

---

## 11) Test Matrix

| Scenario                              | Expected Outcome                                                    |
| ------------------------------------- | ------------------------------------------------------------------- |
| Missing `Config.toml`                 | Start with defaults; log info                                       |
| Invalid `bind_addr`                   | Fail fast with explicit error                                       |
| TLS enabled but no keys               | Fail fast                                                           |
| Body over `max_body_bytes`            | `413 Payload Too Large` (service)                                   |
| Ratio > `decompress_ratio_cap`        | `400 Bad Request` + metric                                          |
| α+β > max_inflight                    | Fail validation                                                     |
| `k` out of [16,32]                    | Fail validation                                                     |
| SIGHUP received (single)              | Non-disruptive reload for safe keys; disruptive ones rebind sockets |
| Rapid SIGHUP burst                    | Only one reload after 5s debounce; no thrash                        |
| Reload with invalid TOML              | Old config retained; WARN logged                                    |
| `amnesia=true` with disk-backed store | Refuse to start                                                     |
| `pq.require_pq_on` ≤ today            | Fail validation with clear message                                  |
| Profile: `micronode`                  | Amnesia on; lower limits applied; service healthy                   |
| ASN entropy drops < 1.5               | WARNING alert fired; runbook link present                           |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config File] --> D
  C[Env Vars] --> D
  E[CLI Flags] --> D
  P[Profile] --> D
  RT[Reload Trigger (SIGHUP/bus, 5s debounce)] --> D
  D --> V{Validate}
  V -- ok --> R[Runtime Snapshot]
  V -- fail --> X[Keep Old & Warn]
  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Keep **prod config under version control** (private repo/secret store).
* Prefer **env vars** for containers; mount secrets read-only; rotate keys regularly.
* Document default ports and firewall expectations near `bind_addr` in ops playbooks.
* Pair knobs with dashboards/alerts: hops p99, rejects by reason, PQ dual-sign ratio, churn success.
* Rotate **bootstrap seeds** quarterly; verify ASN/region diversity.
* Pin alert severities and escalation (on-call rotation) in the paging policy.
* Include this file in PR reviews whenever config changes are introduced.

---

## 14) Profiles (Tiered Overrides)

Profiles provide opinionated overrides for common deployments. They are loaded **after** file/env/defaults but **before** CLI. Use `--profile <name>` or `SVC_DHT_PROFILE=<name>`.

### 14.1 micronode (edge/dev, amnesia-first)

```toml
[profile.micronode]
amnesia.enabled = true
limits.max_body_bytes = "512KiB"
limits.max_rps = 200
limits.max_inflight = 256
dht.alpha = 3
dht.beta = 2
bootstrap.required = 1
log.format = "json"
log.level = "info"
```

### 14.2 macronode (core/cluster)

```toml
[profile.macronode]
amnesia.enabled = false
limits.max_body_bytes = "2MiB"
limits.max_rps = 2000
limits.max_inflight = 2048
dht.alpha = 4
dht.beta = 3
bootstrap.required = 3
diversity.asn_max_pct = 40
diversity.asn_entropy_min = 1.5
log.format = "json"
log.level = "info"
```

---

## 15) Alerts (PromQL & Runbooks)

> All alerts should include a link to the appropriate **runbook** (e.g., `docs/RUNBOOK.md#…`), represented below as `runbook://…`.

### 15.1 Hop SLO coverage (CRITICAL)

**Expr:**

```
(1 - histogram_quantile(0.99, sum by (le) (rate(dht_lookup_hops_histogram_bucket[5m]))))
```

**Condition:** < 0.01 for **5m** (i.e., p99 exceeds 5 hops)
**Runbook:** `runbook://svc-dht/hops-slo`

### 15.2 Quota storm (WARNING)

**Expr:**

```
sum(rate(rejected_total{reason="quota"}[5m])) > 5
```

**For:** 10m
**Runbook:** `runbook://svc-dht/limits-tuning`

### 15.3 PQ migration compliance (WARNING→CRITICAL)

**Expr:**

```
avg_over_time(pq_dual_sign_ratio[15m]) < 0.95
```

**For:** 30m (WARNING); 2h (CRITICAL)
**Runbook:** `runbook://svc-dht/pq-migration`

### 15.4 Diversity erosion (WARNING)

**Expr:**

```
avg_over_time(dht_bucket_asn_entropy_bits[15m]) < 1.5
```

**For:** 30m
**Runbook:** `runbook://svc-dht/eclipse-mitigation`

> Note: expose `dht_bucket_asn_entropy_bits` as a gauge (per bucket); roll up via `avg by (bucket)`.

```
```
