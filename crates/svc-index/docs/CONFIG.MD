

````markdown
---
title: Configuration — svc-index
crate: svc-index
owner: Stevan White
last-reviewed: 2025-10-03
status: draft
template_version: 1.1
---

# Configuration — `svc-index` (Expanded)

This document defines **all configuration** for `svc-index`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/SECURITY.md`, and the IDB.

> **Tiering:** Service crate — all sections apply (network, readiness, observability, etc.).  
> **Env prefix:** `SVC_INDEX_` (primary). Aliases `RON_INDEX_…` accepted during deprecation window (see §9).

---

## 1) Sources & Precedence (Authoritative)

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (`Config.toml` path via `--config` or default beside the binary)  
4. **Built-in defaults**

Dynamic reload recomputes the **effective config** using the same precedence.

**Formats:** TOML (preferred), JSON (optional).  
**Relative `--config` resolution:** `./`, `$CWD`, crate dir.

---

## 2) Quickstart Examples

### 2.1 Minimal service start
```bash
RUST_LOG=info \
SVC_INDEX_BIND_ADDR=0.0.0.0:8080 \
SVC_INDEX_METRICS_ADDR=127.0.0.1:9108 \
SVC_INDEX_DB_PATH=./_local/index.db \
cargo run -p svc-index
````

### 2.2 Config file (TOML) — production-ish baseline

```toml
# Config.toml

bind_addr    = "0.0.0.0:8080"
metrics_addr = "127.0.0.1:9108"
max_conns    = 2048

read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "60s"

[limits]
max_body_bytes       = "1MiB"
decompress_ratio_cap = 10

[server]                  # HTTP server tuning (optional)
http1_only       = true
tcp_nodelay      = true
tcp_backlog      = 4096
connection_limit_per_ip = 256
keepalive_secs    = 60
acceptor_threads  = 1

[db]
path    = "./_local/index.db"
backend = "sled"              # enum: "sled" | "rocks"

[db.rocks]                     # only if backend="rocks"
write_buffer_size       = "128MiB"
max_background_jobs     = 4
target_file_size_base   = "64MiB"

[dht]
alpha        = 3
beta         = 2
hedge_ms     = 150
deadline_ms  = 1200
bootstrap    = ["node1.example:4100","node2.example:4100"]  # optional seed peers
retry_ms     = 250
max_retries  = 3

[cache]
entries     = 150000
ttl_ms      = 15000
neg_ttl_ms  = 3000
warmup_keys = []               # optional set of hot names/cids to prefill

[rank]
region_bias            = 0.6
freshness_half_life_s  = 600
latency_buckets_ms     = [30, 80, 200, 500]
allowed_regions        = []    # optional allowlist; empty = any
provider_denylist      = []    # optional provider IDs to exclude

[breaker]
error_window_s = 30
trip_threshold = 0.5
cooldown_s     = 10
half_open_probes = 3

[ratelimit]                         # simple token-bucket, per-IP or global
enabled       = true
burst         = 200
rate_per_sec  = 100
scope         = "per_ip"            # "per_ip" | "global"

[quotas]                            # coarse global quotas (optional)
max_resolves_per_min  = 60000
max_providers_per_min = 60000

[auth]
macaroon_path = ""                  # capability token file for admin/facets
allowed_roles = ["admin","ops"]     # labels checked by your verifier, optional

[amnesia]
enabled            = false
purge_interval_ms  = 30000

[uds]
enabled    = false
path       = "/run/ron/index.sock"
allow_uids = []

[log]
format = "json"                     # "json" | "text"
level  = "info"                     # "trace".."error"

[tracing]                           # OTLP/Jaeger exporter (optional)
enabled      = false
otlp_endpoint = "http://127.0.0.1:4317"
sample_ratio  = 0.05                # 0.0..1.0

[metrics]
namespace          = "ron"
histogram_buckets_ms = [10,25,50,80,120,200,350,600,1000]

[cors]                               # if exposed behind browsers
enabled       = false
allow_origins = ["https://example.com"]
allow_methods = ["GET","POST"]
allow_headers = ["Content-Type","Authorization"]

[compression]                        # response compression
enabled = true
min_length_bytes = 1024
```

### 2.3 CLI flags (override file/env)

```bash
cargo run -p svc-index -- \
  --config ./Config.toml \
  --bind 0.0.0.0:8080 \
  --metrics 127.0.0.1:9108 \
  --max-conns 4096 \
  --alpha 3 --beta 2 --hedge-ms 150 --deadline-ms 1200 \
  --db-path ./_local/index.db \
  --uds /run/ron/index.sock
```

---

## 3) Schema (Typed, With Defaults)

> **Durations:** `ms`, `s`, `m`, `h`. **Sizes:** `B`, `KB`, `MB`, `MiB`.
> All env vars start with `SVC_INDEX_…`. Only key highlights are listed below; full set mirrors TOML.

| Key / Env Var                                                     | Type                  | Default                 | Description                       | Security Notes              |
| ----------------------------------------------------------------- | --------------------- | ----------------------- | --------------------------------- | --------------------------- |
| `bind_addr` / `SVC_INDEX_BIND_ADDR`                               | socket                | `127.0.0.1:0`           | HTTP bind address                 | Public binds need review    |
| `metrics_addr` / `SVC_INDEX_METRICS_ADDR`                         | socket                | `127.0.0.1:0`           | Prometheus bind                   | Prefer localhost            |
| `max_conns` / `SVC_INDEX_MAX_CONNS`                               | u32                   | `1024`                  | Max concurrent connections        | FD hygiene                  |
| `read_timeout` / `SVC_INDEX_READ_TIMEOUT`                         | duration              | `5s`                    | Read timeout per request          | DoS guard                   |
| `write_timeout` / `SVC_INDEX_WRITE_TIMEOUT`                       | duration              | `5s`                    | Write timeout per request         | DoS guard                   |
| `idle_timeout` / `SVC_INDEX_IDLE_TIMEOUT`                         | duration              | `60s`                   | Keep-alive idle close             | Resource hygiene            |
| `limits.max_body_bytes` / `SVC_INDEX_MAX_BODY_BYTES`              | size                  | `1MiB`                  | Request body cap                  | OAP bound guard             |
| `limits.decompress_ratio_cap` / `SVC_INDEX_DECOMPRESS_RATIO_CAP`  | u32                   | `10`                    | Max decompression ratio           | Zip-bomb guard              |
| `server.http1_only` / `SVC_INDEX_HTTP1_ONLY`                      | bool                  | `true`                  | Force HTTP/1.1                    | Keep simple                 |
| `server.tcp_nodelay` / `SVC_INDEX_TCP_NODELAY`                    | bool                  | `true`                  | Disable Nagle                     | Latency                     |
| `server.tcp_backlog` / `SVC_INDEX_TCP_BACKLOG`                    | u32                   | `4096`                  | Listen backlog                    | –                           |
| `server.connection_limit_per_ip` / `SVC_INDEX_CONN_LIMIT_PER_IP`  | u32                   | `256`                   | Per-IP cap                        | DoS guard                   |
| `server.keepalive_secs` / `SVC_INDEX_KEEPALIVE_SECS`              | u32                   | `60`                    | TCP keepalive                     | –                           |
| `server.acceptor_threads` / `SVC_INDEX_ACCEPTOR_THREADS`          | u16                   | `1`                     | Accept loop threads               | –                           |
| `db.path` / `SVC_INDEX_DB_PATH`                                   | path                  | `./index.db`            | Index DB root                     | Owner-only perms            |
| `db.backend` / `SVC_INDEX_DB_BACKEND`                             | enum `sled\|rocks`    | `sled`                  | Engine                            | Rocks adds subkeys          |
| `db.rocks.write_buffer_size` / `SVC_INDEX_ROCKS_WB`               | size                  | `128MiB`                | Rocks memtable buffer             | Memory planning             |
| `db.rocks.max_background_jobs` / `SVC_INDEX_ROCKS_BG`             | u32                   | `4`                     | Rocks background jobs             | –                           |
| `db.rocks.target_file_size_base` / `SVC_INDEX_ROCKS_SST`          | size                  | `64MiB`                 | SST size target                   | –                           |
| `dht.alpha` / `SVC_INDEX_ALPHA`                                   | u8                    | `3`                     | First-wave parallelism            | –                           |
| `dht.beta` / `SVC_INDEX_BETA`                                     | u8                    | `2`                     | Hedged second wave                | –                           |
| `dht.hedge_ms` / `SVC_INDEX_HEDGE_MS`                             | u32 (ms)              | `150`                   | Hedge after T ms                  | –                           |
| `dht.deadline_ms` / `SVC_INDEX_DEADLINE_MS`                       | u32 (ms)              | `1200`                  | Absolute per-lookup deadline      | –                           |
| `dht.bootstrap[]` / `SVC_INDEX_BOOTSTRAP`                         | list<string>          | `[]`                    | Seed peers                        | Trust carefully             |
| `dht.retry_ms` / `SVC_INDEX_RETRY_MS`                             | u32 (ms)              | `250`                   | Retry backoff base                | –                           |
| `dht.max_retries` / `SVC_INDEX_MAX_RETRIES`                       | u8                    | `3`                     | Retry attempts                    | –                           |
| `cache.entries` / `SVC_INDEX_CACHE_ENTRIES`                       | u32                   | `100000`                | LRU size                          | –                           |
| `cache.ttl_ms` / `SVC_INDEX_CACHE_TTL_MS`                         | u32 (ms)              | `10000`                 | Hit TTL                           | –                           |
| `cache.neg_ttl_ms` / `SVC_INDEX_NEG_TTL_MS`                       | u32 (ms)              | `2000`                  | Miss TTL                          | –                           |
| `cache.warmup_keys[]` / `SVC_INDEX_WARMUP_KEYS`                   | list<string>          | `[]`                    | Prefill hot IDs                   | –                           |
| `rank.region_bias` / `SVC_INDEX_RANK_REGION_BIAS`                 | f32 0..1              | `0.6`                   | Region weight in ranking          | –                           |
| `rank.freshness_half_life_s` / `SVC_INDEX_RANK_FRESH_HALF_LIFE_S` | u32 (s)               | `600`                   | Exponential decay                 | –                           |
| `rank.latency_buckets_ms[]` / `SVC_INDEX_RANK_LAT_BUCKETS_MS`     | list<u32>             | `[30,80,200,500]`       | Boundaries for latency classes    | –                           |
| `rank.allowed_regions[]` / `SVC_INDEX_ALLOWED_REGIONS`            | list<string>          | `[]`                    | Region allowlist                  | Policy gate                 |
| `rank.provider_denylist[]` / `SVC_INDEX_PROVIDER_DENYLIST`        | list<string>          | `[]`                    | Providers to exclude              | –                           |
| `breaker.error_window_s` / `SVC_INDEX_BREAKER_WINDOW_S`           | u32 (s)               | `30`                    | Sliding window                    | –                           |
| `breaker.trip_threshold` / `SVC_INDEX_BREAKER_TRIP`               | f32 0..1              | `0.5`                   | Trip error ratio                  | –                           |
| `breaker.cooldown_s` / `SVC_INDEX_BREAKER_COOLDOWN_S`             | u32 (s)               | `10`                    | Cooldown before half-open         | –                           |
| `breaker.half_open_probes` / `SVC_INDEX_BREAKER_PROBES`           | u32                   | `3`                     | Probe attempts in half-open       | –                           |
| `ratelimit.enabled` / `SVC_INDEX_RL_ENABLED`                      | bool                  | `false`                 | Enable simple rate limit          | –                           |
| `ratelimit.burst` / `SVC_INDEX_RL_BURST`                          | u32                   | `200`                   | Token bucket burst                | –                           |
| `ratelimit.rate_per_sec` / `SVC_INDEX_RL_RATE`                    | u32                   | `100`                   | Tokens added per second           | –                           |
| `ratelimit.scope` / `SVC_INDEX_RL_SCOPE`                          | enum `per_ip\|global` | `per_ip`                | Limiting scope                    | –                           |
| `quotas.max_resolves_per_min` / `SVC_INDEX_Q_RESOLVES_PM`         | u32                   | `0` (disabled)          | Coarse global quota               | –                           |
| `quotas.max_providers_per_min` / `SVC_INDEX_Q_PROVIDERS_PM`       | u32                   | `0` (disabled)          | Coarse global quota               | –                           |
| `auth.macaroon_path` / `SVC_INDEX_MACAROON_PATH`                  | path                  | `""`                    | Capability token for admin/facets | Custody via KMS recommended |
| `auth.allowed_roles[]` / `SVC_INDEX_ALLOWED_ROLES`                | list<string>          | `[]`                    | Optional role allowlist           | –                           |
| `amnesia.enabled` / `SVC_INDEX_AMNESIA`                           | bool                  | `false`                 | RAM-only secrets/caches           | No persistent spill         |
| `amnesia.purge_interval_ms` / `SVC_INDEX_AMNESIA_PURGE_MS`        | u32 (ms)              | `30000`                 | Purge cadence                     | –                           |
| `uds.enabled` / `SVC_INDEX_UDS_ENABLED`                           | bool                  | `false`                 | Enable UDS                        | Gated & perms               |
| `uds.path` / `SVC_INDEX_UDS_PATH`                                 | path                  | `"/run/ron/index.sock"` | Socket path                       | Dir 0700/sock 0600          |
| `uds.allow_uids[]` / `SVC_INDEX_UDS_ALLOW_UIDS`                   | list<u32>             | `[]`                    | PEERCRED allowlist                | Strict allowlist            |
| `log.format` / `SVC_INDEX_LOG_FORMAT`                             | enum `json\|text`     | `json`                  | Structured logs                   | JSON in prod                |
| `log.level` / `SVC_INDEX_LOG_LEVEL`                               | enum `trace..error`   | `info`                  | Global log level                  | Avoid `trace` in prod       |
| `tracing.enabled` / `SVC_INDEX_TRACING`                           | bool                  | `false`                 | Enable OTLP exporter              | –                           |
| `tracing.otlp_endpoint` / `SVC_INDEX_OTLP_ENDPOINT`               | url                   | `http://127.0.0.1:4317` | Collector endpoint                | –                           |
| `tracing.sample_ratio` / `SVC_INDEX_SAMPLE_RATIO`                 | f32 0..1              | `0.0`                   | Trace sampling fraction           | –                           |
| `metrics.namespace` / `SVC_INDEX_METRICS_NS`                      | string                | `ron`                   | Prometheus namespace prefix       | –                           |
| `metrics.histogram_buckets_ms[]` / `SVC_INDEX_METRIC_BUCKETS_MS`  | list<u64>             | see TOML above          | Custom latency buckets            | –                           |
| `cors.enabled` / `SVC_INDEX_CORS`                                 | bool                  | `false`                 | Enable CORS                       | –                           |
| `cors.allow_origins[]` / `SVC_INDEX_CORS_ORIGINS`                 | list<string>          | `[]`                    | Allowed origins                   | –                           |
| `cors.allow_methods[]` / `SVC_INDEX_CORS_METHODS`                 | list<string>          | `["GET"]`               | Methods                           | –                           |
| `cors.allow_headers[]` / `SVC_INDEX_CORS_HEADERS`                 | list<string>          | `[]`                    | Headers                           | –                           |
| `compression.enabled` / `SVC_INDEX_COMPRESS`                      | bool                  | `true`                  | Enable response compression       | –                           |
| `compression.min_length_bytes` / `SVC_INDEX_COMPRESS_MIN`         | u32                   | `1024`                  | Min size to compress              | –                           |

---

## 4) Validation Rules (Fail-Closed)

* Sockets parse; ports <1024 require privileges.
* `max_conns > 0`; `limits.max_body_bytes ≥ 1KiB`; `decompress_ratio_cap ≥ 1`.
* `rank.latency_buckets_ms` strictly increasing and non-empty.
* `breaker.trip_threshold ∈ [0,1]`; window/cooldown/probes > 0.
* If `uds.enabled`: parent dir exists; **dir 0700**, **sock 0600**; `allow_uids` non-empty in prod.
* If `auth.macaroon_path` set: file exists; non-empty.
* If `amnesia.enabled = true`: **reject** if `db.path` is not tmpfs/ephemeral (no persistent spill).
* If `db.backend="rocks"` and any `[db.rocks]` set: sizes sane (wb ≥ 8MiB), background jobs > 0.
* If `ratelimit.enabled=true`: `rate_per_sec > 0`.
* If `quotas.* > 0`: enforce counters; otherwise disabled.

**On violation:** structured error → **exit non-zero**.

---

## 5) Dynamic Reload

**Triggers:** SIGHUP or bus `ConfigUpdated { version }`.
**Non-disruptive:** timeouts, limits, caches, rank weights, breaker/ratelimit/quotas, log level, tracing sample.
**Disruptive (rebind):** `bind_addr`, `metrics_addr`, any `uds.*`.
**Requires restart:** `db.backend`, `db.path`, `server.acceptor_threads`.

Atomic swap a validated snapshot **without holding `.await`**. Emit `KernelEvent::ConfigUpdated { version }` and a redacted diff.

---

## 6) CLI Flags (Canonical)

```
--config <path>                --bind <ip:port>        --metrics <ip:port>
--max-conns <u32>             --read-timeout <dur>    --write-timeout <dur>   --idle-timeout <dur>
--db-path <path>              --db-backend <sled|rocks>
--alpha <u8> --beta <u8>      --hedge-ms <u32>        --deadline-ms <u32>     --bootstrap <host:port,host:port>
--cache-entries <u32>         --cache-ttl-ms <u32>    --neg-ttl-ms <u32>
--rank-region-bias <f32>      --rank-fresh-half-life-s <u32> --rank-lat-buckets <csv>
--breaker-window-s <u32>      --breaker-trip <f32>    --breaker-cooldown-s <u32> --breaker-probes <u32>
--ratelimit <on|off>          --rl-burst <u32>        --rl-rate <u32>         --rl-scope <per_ip|global>
--quotas-resolves-pm <u32>    --quotas-providers-pm <u32>
--uds <path>                  --log-format <json|text> --log-level <trace|debug|info|warn|error>
--otlp <endpoint>             --sample-ratio <0..1>
```

---

## 7) Feature Flags (Cargo)

| Feature | Default | Effect                                                        |
| ------- | :-----: | ------------------------------------------------------------- |
| `uds`   |   off   | Enables UDS listener and `uds.*` config                       |
| `tls`   |   off   | (If ever exposed directly) enables tokio-rustls TLS endpoints |
| `cli`   |    on   | Enables CLI parsing                                           |
| `kameo` |   off   | Optional actor integration                                    |

---

## 8) Security Implications

* **Public binds (`0.0.0.0`)** require strict caps (timeouts, body, inflight), rate-limits, and network policy.
* **Auth/macaroons:** store via **ron-kms** if available; rotate ≤30d; zeroize on drop; never log.
* **Amnesia:** when on, no disk spill (reject persistent paths); verify purge timers fire.
* **UDS:** enforce **SO_PEERCRED** allow-list; dir `0700`, socket `0600`.
* **Bootstrap peers:** treat as trusted inputs; prefer pinned certs/TLS if applicable.
* **CORS:** enable only for specific origins.
* **Tracing:** OTLP endpoints may exfiltrate metadata—use private collectors.

---

## 9) Compatibility & Migration

Backwards-compatible keys add safe defaults. Renames keep env aliases for ≥1 minor and warn.

| Old Env (alias)           | New Env (primary)         | Removal Target | Notes               |
| ------------------------- | ------------------------- | :------------: | ------------------- |
| `RON_INDEX_DB`            | `SVC_INDEX_DB_PATH`       |     v1.2.0     | DB path             |
| `RON_INDEX_MAX_CONNS`     | `SVC_INDEX_MAX_CONNS`     |     v1.2.0     | Connection cap      |
| `RON_INDEX_ALPHA`         | `SVC_INDEX_ALPHA`         |     v1.2.0     | DHT α               |
| `RON_INDEX_BETA`          | `SVC_INDEX_BETA`          |     v1.2.0     | DHT β               |
| `RON_INDEX_HEDGE_MS`      | `SVC_INDEX_HEDGE_MS`      |     v1.2.0     | Hedge               |
| `RON_INDEX_DEADLINE_MS`   | `SVC_INDEX_DEADLINE_MS`   |     v1.2.0     | Per-lookup deadline |
| `RON_INDEX_CACHE_ENTRIES` | `SVC_INDEX_CACHE_ENTRIES` |     v1.2.0     | Cache size          |
| `RON_INDEX_CACHE_TTL_MS`  | `SVC_INDEX_CACHE_TTL_MS`  |     v1.2.0     | Cache TTL           |
| `RON_INDEX_NEG_TTL_MS`    | `SVC_INDEX_NEG_TTL_MS`    |     v1.2.0     | Negative TTL        |

Breaking changes require a **major** bump and `CHANGELOG.md` migration steps.

---

## 10) Reference Implementation (Rust)

> Paste into `src/config.rs`. This is a superset of the earlier version (facets/ratelimit/tracing/metrics/server/rocks added). Keep comments.

```rust
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default = "default_body_bytes")]
    pub max_body_bytes: u64,
    #[serde(default = "default_decompress_ratio")]
    pub decompress_ratio_cap: u32,
}
fn default_body_bytes() -> u64 { 1 * 1024 * 1024 }
fn default_decompress_ratio() -> u32 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbBackend { Sled, Rocks }
impl Default for DbBackend { fn default() -> Self { DbBackend::Sled } }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RocksCfg {
    pub write_buffer_size: Option<String>, // parse to bytes at load if present
    pub max_background_jobs: Option<u32>,
    pub target_file_size_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbCfg {
    pub path: Option<PathBuf>, // default ./index.db
    #[serde(default)]
    pub backend: DbBackend,
    #[serde(default)]
    pub rocks: RocksCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DhtCfg {
    #[serde(default = "default_alpha")]    pub alpha: u8,
    #[serde(default = "default_beta")]     pub beta: u8,
    #[serde(default = "default_hedge_ms")] pub hedge_ms: u32,
    #[serde(default = "default_deadline")] pub deadline_ms: u32,
    #[serde(default)]                      pub bootstrap: Vec<String>,
    #[serde(default = "default_retry_ms")] pub retry_ms: u32,
    #[serde(default = "default_max_retries")] pub max_retries: u8,
}
fn default_alpha() -> u8 { 3 }
fn default_beta() -> u8 { 2 }
fn default_hedge_ms() -> u32 { 150 }
fn default_deadline() -> u32 { 1200 }
fn default_retry_ms() -> u32 { 250 }
fn default_max_retries() -> u8 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheCfg {
    #[serde(default = "default_cache_entries")] pub entries: u32,
    #[serde(default = "default_cache_ttl")]     pub ttl_ms: u32,
    #[serde(default = "default_neg_ttl")]       pub neg_ttl_ms: u32,
    #[serde(default)]                           pub warmup_keys: Vec<String>,
}
fn default_cache_entries() -> u32 { 100_000 }
fn default_cache_ttl() -> u32 { 10_000 }
fn default_neg_ttl() -> u32 { 2_000 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RankCfg {
    #[serde(default = "default_region_bias")]       pub region_bias: f32,
    #[serde(default = "default_fresh_halflife_s")]  pub freshness_half_life_s: u32,
    #[serde(default)]                               pub latency_buckets_ms: Vec<u32>,
    #[serde(default)]                               pub allowed_regions: Vec<String>,
    #[serde(default)]                               pub provider_denylist: Vec<String>,
}
fn default_region_bias() -> f32 { 0.6 }
fn default_fresh_halflife_s() -> u32 { 600 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BreakerCfg {
    #[serde(default = "default_br_window")]  pub error_window_s: u32,
    #[serde(default = "default_br_trip")]    pub trip_threshold: f32,
    #[serde(default = "default_br_cool")]    pub cooldown_s: u32,
    #[serde(default = "default_br_probes")]  pub half_open_probes: u32,
}
fn default_br_window() -> u32 { 30 }
fn default_br_trip() -> f32 { 0.5 }
fn default_br_cool() -> u32 { 10 }
fn default_br_probes() -> u32 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitCfg {
    #[serde(default)] pub enabled: bool,
    #[serde(default = "default_rl_burst")] pub burst: u32,
    #[serde(default = "default_rl_rate")]  pub rate_per_sec: u32,
    #[serde(default = "default_rl_scope")] pub scope: RateScope,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateScope { PerIp, Global }
fn default_rl_burst() -> u32 { 200 }
fn default_rl_rate() -> u32 { 100 }
fn default_rl_scope() -> RateScope { RateScope::PerIp }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuotasCfg {
    #[serde(default)] pub max_resolves_per_min: u32,
    #[serde(default)] pub max_providers_per_min: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthCfg {
    pub macaroon_path: Option<PathBuf>,
    #[serde(default)] pub allowed_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AmnesiaCfg {
    #[serde(default)] pub enabled: bool,
    #[serde(default = "default_amnesia_purge")] pub purge_interval_ms: u32,
}
fn default_amnesia_purge() -> u32 { 30_000 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UdsCfg {
    #[serde(default)] pub enabled: bool,
    pub path: Option<PathBuf>,
    #[serde(default)] pub allow_uids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCfg {
    #[serde(default = "default_log_format")] pub format: LogFormat,
    #[serde(default = "default_log_level")]  pub level: LogLevel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat { Json, Text }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error }
fn default_log_format() -> LogFormat { LogFormat::Json }
fn default_log_level() -> LogLevel { LogLevel::Info }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TracingCfg {
    #[serde(default)] pub enabled: bool,
    pub otlp_endpoint: Option<String>,
    #[serde(default)] pub sample_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsCfg {
    #[serde(default = "default_metrics_ns")] pub namespace: String,
    #[serde(default)] pub histogram_buckets_ms: Vec<u64>,
}
fn default_metrics_ns() -> String { "ron".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCfg {
    #[serde(default = "default_true")] pub http1_only: bool,
    #[serde(default = "default_true")] pub tcp_nodelay: bool,
    #[serde(default = "default_backlog")] pub tcp_backlog: u32,
    #[serde(default = "default_conn_limit_ip")] pub connection_limit_per_ip: u32,
    #[serde(default = "default_keepalive_secs")] pub keepalive_secs: u32,
    #[serde(default = "default_acceptor_threads")] pub acceptor_threads: u16,
}
fn default_true() -> bool { true }
fn default_backlog() -> u32 { 4096 }
fn default_conn_limit_ip() -> u32 { 256 }
fn default_keepalive_secs() -> u32 { 60 }
fn default_acceptor_threads() -> u16 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorsCfg {
    #[serde(default)] pub enabled: bool,
    #[serde(default)] pub allow_origins: Vec<String>,
    #[serde(default = "default_cors_methods")] pub allow_methods: Vec<String>,
    #[serde(default)] pub allow_headers: Vec<String>,
}
fn default_cors_methods() -> Vec<String> { vec!["GET".into(),"POST".into()] }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionCfg {
    #[serde(default = "default_true")] pub enabled: bool,
    #[serde(default = "default_compress_min")] pub min_length_bytes: u32,
}
fn default_compress_min() -> u32 { 1024 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind_addr: Option<SocketAddr>,    // default 127.0.0.1:0
    pub metrics_addr: Option<SocketAddr>, // default 127.0.0.1:0
    #[serde(default = "default_max_conns")] pub max_conns: u32,

    #[serde(with = "humantime_serde", default = "default_5s")]
    pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_60s")]
    pub idle_timeout: Duration,

    #[serde(default)] pub limits: Limits,
    #[serde(default)] pub server: ServerCfg,
    #[serde(default)] pub db: DbCfg,
    #[serde(default)] pub dht: DhtCfg,
    #[serde(default)] pub cache: CacheCfg,
    #[serde(default)] pub rank: RankCfg,
    #[serde(default)] pub breaker: BreakerCfg,
    #[serde(default)] pub ratelimit: RateLimitCfg,
    #[serde(default)] pub quotas: QuotasCfg,
    #[serde(default)] pub auth: AuthCfg,
    #[serde(default)] pub amnesia: AmnesiaCfg,
    #[serde(default)] pub uds: UdsCfg,
    #[serde(default)] pub log: LogCfg,
    #[serde(default)] pub tracing: TracingCfg,
    #[serde(default)] pub metrics: MetricsCfg,
    #[serde(default)] pub cors: CorsCfg,
    #[serde(default)] pub compression: CompressionCfg,
}

fn default_5s() -> Duration { Duration::from_secs(5) }
fn default_60s() -> Duration { Duration::from_secs(60) }
fn default_max_conns() -> u32 { 1024 }

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: Some("127.0.0.1:0".parse().unwrap()),
            metrics_addr: Some("127.0.0.1:0".parse().unwrap()),
            max_conns: default_max_conns(),
            read_timeout: default_5s(),
            write_timeout: default_5s(),
            idle_timeout: default_60s(),
            limits: Limits { max_body_bytes: default_body_bytes(), decompress_ratio_cap: default_decompress_ratio() },
            server: ServerCfg::default(),
            db: DbCfg { path: Some(PathBuf::from("./index.db")), backend: DbBackend::Sled, rocks: RocksCfg::default() },
            dht: DhtCfg::default(),
            cache: CacheCfg::default(),
            rank: RankCfg { region_bias: 0.6, freshness_half_life_s: 600, latency_buckets_ms: vec![30,80,200,500], allowed_regions: vec![], provider_denylist: vec![] },
            breaker: BreakerCfg::default(),
            ratelimit: RateLimitCfg::default(),
            quotas: QuotasCfg::default(),
            auth: AuthCfg::default(),
            amnesia: AmnesiaCfg::default(),
            uds: UdsCfg::default(),
            log: LogCfg { format: LogFormat::Json, level: LogLevel::Info },
            tracing: TracingCfg::default(),
            metrics: MetricsCfg::default(),
            cors: CorsCfg::default(),
            compression: CompressionCfg::default(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_conns == 0 { anyhow::bail!("max_conns must be > 0"); }
        if self.limits.max_body_bytes < 1024 { anyhow::bail!("max_body_bytes too small"); }
        if self.limits.decompress_ratio_cap == 0 { anyhow::bail!("decompress_ratio_cap must be >= 1"); }

        // Rank buckets strictly increasing
        if !self.rank.latency_buckets_ms.is_empty() {
            let mut last = 0;
            for (i, v) in self.rank.latency_buckets_ms.iter().enumerate() {
                if i > 0 && *v <= last { anyhow::bail!("rank.latency_buckets_ms must be strictly increasing"); }
                last = *v;
            }
        }

        // Amnesia: forbid persistent spill
        if self.amnesia.enabled {
            if let Some(p) = &self.db.path {
                let s = p.display().to_string();
                if !s.starts_with("/tmp") && !s.contains("memfs") && !s.contains("tmpfs") {
                    anyhow::bail!("amnesia.enabled=true conflicts with persistent db.path: {s}");
                }
            }
        }

        // UDS hygiene
        if self.uds.enabled {
            let path = self.uds.path.as_ref().ok_or_else(|| anyhow::anyhow!("uds.enabled=true but uds.path missing"))?;
            if let Some(dir) = path.parent() {
                if !dir.exists() { anyhow::bail!("uds parent dir missing: {}", dir.display()); }
            }
        }

        // Rocks sanity if selected
        if matches!(self.db.backend, DbBackend::Rocks) {
            if let Some(bg) = self.db.rocks.max_background_jobs { if bg == 0 { anyhow::bail!("rocks.max_background_jobs must be > 0"); } }
        }

        // Ratelimit/Quotas sanity
        if self.ratelimit.enabled && self.ratelimit.rate_per_sec == 0 {
            anyhow::bail!("ratelimit.rate_per_sec must be > 0 when ratelimit.enabled=true");
        }

        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                         | Expected Outcome                                                 |
| -------------------------------- | ---------------------------------------------------------------- |
| Missing `Config.toml`            | Start with defaults; structured warning                          |
| Invalid `bind_addr`              | Fail fast with explicit error                                    |
| Body > `max_body_bytes`          | `413 Payload Too Large` + metric                                 |
| Decompression ratio exceeds cap  | `400 Bad Request` + metric                                       |
| UDS enabled without path         | Fail validation                                                  |
| Amnesia with persistent db.path  | Fail validation                                                  |
| DHT hop deadlines exceeded       | 504/timeout per route + breaker increments + hedge metrics       |
| Per-IP connection limit exceeded | Immediate reject + `rejected_total{reason="over_capacity"}`      |
| Rate limit active                | 429 + token bucket metrics                                       |
| SIGHUP                           | Live reload for safe keys; rebind for listeners; emits bus event |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config File] --> D
  C[Env Vars (SVC_INDEX_ / aliases)] --> D
  E[CLI Flags] --> D
  D --> V{Validate}
  V -- ok --> R[Runtime Snapshot]
  V -- fail --> X[Exit / Error]
  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Version-control prod config in a private repo/secret store; mount secrets read-only.
* Treat `db.path` as part of node identity; **do not** share across processes.
* Keep **bench/SLO knobs** (`alpha`, `beta`, `deadline_ms`, buckets) documented alongside perf baselines.
* Expose `metrics_addr` only inside cluster/VPN; if public, protect via network policy.
* Use OTLP tracing sparingly in prod—sample and scrub metadata.

```
```
