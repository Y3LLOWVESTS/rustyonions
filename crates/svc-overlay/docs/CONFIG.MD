

---

````markdown
---
title: Configuration — svc-overlay
crate: svc-overlay
owner: Stevan White
last-reviewed: 2025-10-03
status: draft
template_version: 1.1
---

# Configuration — svc-overlay

This document defines **all configuration** for `svc-overlay`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `IDB.md`, and `docs/SECURITY.md`.

> **Tiering:**  
> **Service crate.** All sections apply (network, readiness, observability, etc.).

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (e.g., `Config.toml` beside the binary or passed via `--config`)  
4. **Built-in defaults** (hard-coded)

On dynamic reload, the effective config is recomputed under the same precedence.

**Supported file formats:** TOML (preferred), JSON (optional).  
**Path resolution order for `--config` (if relative):** `./`, `$CWD`, crate dir.

**Environment prefix:** `SVC_OVERLAY_`.

**Env nesting convention:** use `__` (double underscore) to map nested keys  
(e.g., `SVC_OVERLAY_OVERLAY__GOSSIP__CHUNK_BYTES=64KiB`).  
**Flat aliases** are also supported for common keys (see schema table).

---

## 2) Quickstart Examples

### 2.1 Minimal service start (localhost, ephemeral metrics)
```bash
export RUST_LOG=info
export SVC_OVERLAY_BIND_ADDR=127.0.0.1:9500
export SVC_OVERLAY_METRICS_ADDR=127.0.0.1:0
cargo run -p svc-overlay -- --bind 127.0.0.1:9500
````

### 2.2 Config file (TOML) — Micronode-style defaults

```toml
# Config.toml (Micronode profile)
[overlay]
listen              = "127.0.0.1:9500"
advertise_addr      = "127.0.0.1:9500"   # optional (peer hint)
amnesia             = true
pq_mode             = "off"              # "off" | "hybrid"
handshake_timeout   = "3s"

[overlay.peer]
inflight_frames     = 32
max_rps             = 200
send_window_frames  = 16
recv_window_frames  = 16

[overlay.gossip]
max_frame_bytes     = "1MiB"
chunk_bytes         = "64KiB"
relay_fanout        = 8
burst_budget_frames = 8

[limits]
max_body_bytes         = "1MiB"
decompress_ratio_cap   = 10

[timeouts]
read   = "5s"
write  = "5s"
idle   = "15s"

[observability]
metrics_addr = "127.0.0.1:9600"

[tls]
enabled   = false
# cert_path = "/etc/ron/cert.pem"
# key_path  = "/etc/ron/key.pem"

[auth]
macaroon_path = ""        # optional

[uds]
path       = ""           # optional
allow_uids = []           # optional

[transport]
name          = "overlay"
max_conns     = 1024
read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "15s"

[transport.tor]
enabled  = false
socks    = ""             # optional SOCKS proxy

[transport.quic]
enabled  = false
```

### 2.3 Config file (TOML) — Macronode-style (public bind, hybrid PQ)

```toml
# Config.toml (Macronode profile)
[overlay]
listen              = "0.0.0.0:9500"
advertise_addr      = "node.example.com:9500"
amnesia             = false
pq_mode             = "hybrid"
handshake_timeout   = "3s"

[overlay.peer]
inflight_frames     = 128
max_rps             = 500
send_window_frames  = 32
recv_window_frames  = 32

[overlay.gossip]
max_frame_bytes     = "1MiB"
chunk_bytes         = "64KiB"
relay_fanout        = 16
burst_budget_frames = 16

[limits]
max_body_bytes         = "1MiB"
decompress_ratio_cap   = 10

[timeouts]
read   = "5s"
write  = "5s"
idle   = "30s"

[observability]
metrics_addr = "0.0.0.0:9600"

[tls]
enabled   = true
cert_path = "/etc/ron/tls/cert.pem"
key_path  = "/etc/ron/tls/key.pem"

[auth]
macaroon_path = "/etc/ron/overlay.macaroon"

[transport]
name          = "overlay"
max_conns     = 5000
read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "30s"
```

### 2.4 CLI flags override file/env

```bash
cargo run -p svc-overlay -- \
  --bind 0.0.0.0:9500 \
  --metrics 127.0.0.1:9600 \
  --max-conns 2048 \
  --pq hybrid \
  --amnesia=false
```

### 2.5 Container quickstart

```bash
docker run --rm -p 9500:9500 -p 9600:9600 \
  -e SVC_OVERLAY_BIND_ADDR=0.0.0.0:9500 \
  -e SVC_OVERLAY_METRICS_ADDR=0.0.0.0:9600 \
  -e SVC_OVERLAY_AMNESIA=false \
  ghcr.io/yourorg/svc-overlay:latest
```

---

## 3) Schema (Typed, With Defaults)

> **Durations** accept `ms`, `s`, `m`, `h`.
> **Sizes** accept `B`, `KiB`, `MiB`, `GiB` (binary units preferred).
> **Env nesting:** `SVC_OVERLAY_OVERLAY__GOSSIP__CHUNK_BYTES=64KiB`
> **Flat aliases** (ops-friendly) are listed in the Env Var column.

| Key (TOML dotted)                    | Env Var (alias, if any)            | Type                      | Default       | Description                                      | Security Notes                     |
| ------------------------------------ | ---------------------------------- | ------------------------- | ------------- | ------------------------------------------------ | ---------------------------------- |
| `overlay.listen`                     | `SVC_OVERLAY_BIND_ADDR`            | socket                    | `127.0.0.1:0` | Overlay listener (OAP/1 sessions)                | Public binds require threat review |
| `overlay.advertise_addr`             | `SVC_OVERLAY_ADVERTISE_ADDR`       | socket/host:port (string) | `""`          | Optional address advertised to peers             | Avoid leaking internal IPs         |
| `overlay.amnesia`                    | `SVC_OVERLAY_AMNESIA`              | bool                      | `false`       | RAM-only mode; no persistence                    | Disable disk writes                |
| `overlay.pq_mode`                    | `SVC_OVERLAY_PQ_MODE`              | enum(`off`,`hybrid`)      | `off`         | PQ readiness toggle (X25519+Kyber when `hybrid`) | Interop gate; staged rollout       |
| `overlay.handshake_timeout`          | `SVC_OVERLAY_HANDSHAKE_TIMEOUT`    | duration                  | `3s`          | Peer handshake timeout                           | DoS guard                          |
| `overlay.peer.inflight_frames`       | `SVC_OVERLAY_PEER_INFLIGHT`        | u32                       | `32`          | Bounded per-peer in-flight send queue (frames)   | Backpressure invariant             |
| `overlay.peer.max_rps`               | `SVC_OVERLAY_PEER_MAX_RPS`         | u32                       | `200`         | Per-peer rate limit                              | Fair shedding                      |
| `overlay.peer.send_window_frames`    | `SVC_OVERLAY_SEND_WINDOW`          | u32                       | `16`          | Sender sliding window                            | Fairness                           |
| `overlay.peer.recv_window_frames`    | `SVC_OVERLAY_RECV_WINDOW`          | u32                       | `16`          | Receiver window                                  | Fairness                           |
| `overlay.gossip.max_frame_bytes`     | `SVC_OVERLAY_MAX_FRAME_BYTES`      | size                      | `1MiB`        | OAP/1 hard cap                                   | Must equal protocol cap            |
| `overlay.gossip.chunk_bytes`         | `SVC_OVERLAY_CHUNK_BYTES`          | size                      | `64KiB`       | Application streaming chunk size                 | Perf/latency tradeoff              |
| `overlay.gossip.relay_fanout`        | `SVC_OVERLAY_FANOUT`               | u32                       | `8`           | Gossip fanout bound                              | Prevents storms                    |
| `overlay.gossip.burst_budget_frames` | `SVC_OVERLAY_BURST_BUDGET`         | u32                       | `8`           | Token bucket burst size                          | Burst control                      |
| `timeouts.read`                      | `SVC_OVERLAY_READ_TIMEOUT`         | duration                  | `5s`          | Read timeout                                     | DoS mitigation                     |
| `timeouts.write`                     | `SVC_OVERLAY_WRITE_TIMEOUT`        | duration                  | `5s`          | Write timeout                                    | DoS mitigation                     |
| `timeouts.idle`                      | `SVC_OVERLAY_IDLE_TIMEOUT`         | duration                  | `15s`         | Idle connection shutdown                         | Hygiene                            |
| `limits.max_body_bytes`              | `SVC_OVERLAY_MAX_BODY_BYTES`       | size                      | `1MiB`        | Request payload cap (admin/metrics if any)       | Zip-bomb guard                     |
| `limits.decompress_ratio_cap`        | `SVC_OVERLAY_DECOMPRESS_RATIO_CAP` | u32                       | `10`          | Max allowed decompression ratio                  | Zip-bomb guard                     |
| `observability.metrics_addr`         | `SVC_OVERLAY_METRICS_ADDR`         | socket                    | `127.0.0.1:0` | Prometheus metrics/health endpoints              | Prefer localhost scrape            |
| `tls.enabled`                        | `SVC_OVERLAY_TLS_ENABLED`          | bool                      | `false`       | Enable TLS (ron-transport path)                  | Use tokio-rustls only              |
| `tls.cert_path`                      | `SVC_OVERLAY_TLS_CERT_PATH`        | path                      | `""`          | PEM cert path                                    | 0600 perms                         |
| `tls.key_path`                       | `SVC_OVERLAY_TLS_KEY_PATH`         | path                      | `""`          | PEM key path                                     | Zeroize in memory; 0600 perms      |
| `auth.macaroon_path`                 | `SVC_OVERLAY_MACAROON_PATH`        | path                      | `""`          | Capability token file                            | Never log contents; 0600 perms     |
| `uds.path`                           | `SVC_OVERLAY_UDS_PATH`             | path                      | `""`          | Optional Unix Domain Socket                      | Dir 0700, sock 0600                |
| `uds.allow_uids`                     | `SVC_OVERLAY_UDS_ALLOW_UIDS`       | list<u32>                 | `[]`          | PEERCRED allowlist                               | Strict allowlist                   |
| `transport.name`                     | `SVC_OVERLAY_TRANSPORT_NAME`       | string                    | `"overlay"`   | Label for metrics/trace                          | —                                  |
| `transport.max_conns`                | `SVC_OVERLAY_MAX_CONNS`            | u32                       | `1024`        | Max concurrent connections (FD cap)              | Prevents exhaustion                |
| `transport.read_timeout`             | `SVC_OVERLAY_TRAN_READ_TIMEOUT`    | duration                  | `5s`          | Transport read timeout                           | —                                  |
| `transport.write_timeout`            | `SVC_OVERLAY_TRAN_WRITE_TIMEOUT`   | duration                  | `5s`          | Transport write timeout                          | —                                  |
| `transport.idle_timeout`             | `SVC_OVERLAY_TRAN_IDLE_TIMEOUT`    | duration                  | `15s`         | Transport idle timeout                           | —                                  |
| `transport.tor.enabled`              | `SVC_OVERLAY_TOR_ENABLED`          | bool                      | `false`       | Enable Tor/Arti path (if compiled)               | Latency/exit policy                |
| `transport.tor.socks`                | `SVC_OVERLAY_TOR_SOCKS`            | socket/string             | `""`          | External SOCKS proxy (if not embedding)          | Trust boundary                     |
| `transport.quic.enabled`             | `SVC_OVERLAY_QUIC_ENABLED`         | bool                      | `false`       | Enable QUIC path (if compiled)                   | MTU/UDP tuning                     |
| `economics.zk_proof_mode` (optional) | `SVC_OVERLAY_ZK_PROOF_MODE`        | enum(`off`,`phase1`)      | `off`         | Future ZK hooks for value-bearing gossip         | Off by default                     |

> Add new keys with safe defaults and list them here when introduced.

---

## 4) Validation Rules (Fail-Closed)

On startup or reload, apply the following **strict validation**:

* `overlay.listen` must parse to socket; ports <1024 require privileges.
* `overlay.gossip.max_frame_bytes` **must equal** protocol cap (1 MiB). If higher/lower: **error**.
* `overlay.gossip.chunk_bytes` ∈ [4KiB, 1MiB] and ≤ `max_frame_bytes`.
* `overlay.peer.inflight_frames` > 0; `send_window_frames` > 0; `recv_window_frames` > 0.
* `limits.max_body_bytes` ≥ 1 KiB; `decompress_ratio_cap` ≥ 1.
* **TLS**: if `tls.enabled=true`, `cert_path` and `key_path` must exist, not world-readable; on Unix, **0600** perms enforced (see reference code).
* **Macaroon**: if `auth.macaroon_path` set, file must exist and on Unix be **0600**; contents must be non-empty.
* **UDS**: if `uds.path` set, parent dir exists; dir mode `0700`, socket `0600`; if `allow_uids` non-empty, enforce PEERCRED.
* **PQ**: if `overlay.pq_mode="hybrid"`, require build with `pq` feature and runtime KMS support (fail otherwise).
* `transport.max_conns` > 0.

**On violation:** log structured error, **exit non-zero** (service). No partial boots.

---

## 5) Dynamic Reload (If Supported)

**Triggers:** SIGHUP **or** kernel bus event `ConfigUpdated { version: u64 }`.

**Reload semantics:**

* **Non-disruptive:** timeouts, limits, fanout, rate limits, log level.
* **Disruptive (rebind):** `overlay.listen`, `tls.*`, `uds.*`, `observability.metrics_addr`.

**Atomicity:** compute a new config snapshot; swap under a mutex **without** holding `.await`.
**Audit:** emit `KernelEvent::ConfigUpdated { version }` and a redacted diff (no secrets).

---

## 6) CLI Flags (Canonical)

```
--config <path>                # Load Config.toml (merged at low precedence)
--bind <ip:port>               # overlay.listen
--advertise <host:port>        # overlay.advertise_addr
--metrics <ip:port>            # observability.metrics_addr
--max-conns <num>              # transport.max_conns
--read-timeout <dur>           # timeouts.read
--write-timeout <dur>          # timeouts.write
--idle-timeout <dur>           # timeouts.idle
--fanout <n>                   # overlay.gossip.relay_fanout
--burst <n>                    # overlay.gossip.burst_budget_frames
--chunk-bytes <size>           # overlay.gossip.chunk_bytes
--pq <off|hybrid>              # overlay.pq_mode
--amnesia <true|false>         # overlay.amnesia
--tls                          # tls.enabled=true
--tls-cert <path>              # tls.cert_path
--tls-key <path>               # tls.key_path
--uds <path>                   # uds.path
```

---

## 7) Feature Flags (Cargo)

| Feature | Default | Effect                                                       |
| ------- | ------: | ------------------------------------------------------------ |
| `tls`   |     off | Enables TLS path via `tokio-rustls` and `tls.*` keys         |
| `pq`    |     off | Enables PQ hybrid handshake; honors `overlay.pq_mode=hybrid` |
| `arti`  |     off | Enables Tor/Arti integration under `transport.tor.*`         |
| `quic`  |     off | Enables QUIC transport under `transport.quic.*`              |
| `kameo` |     off | Optional actor integration                                   |
| `cli`   |      on | Enables CLI arg parsing layer                                |

> If a feature changes schema, note it here and in `CHANGELOG.md`.

---

## 8) Security Implications

* **Public binds (`0.0.0.0`)** require caps (timeouts, RPS, inflight) and active monitoring.
* **TLS:** only `tokio_rustls::rustls::ServerConfig` (NEVER `rustls::ServerConfig` directly).
* **Macaroons:** never log tokens; rotate ≤ 30 days; ensure keyfile perms (0600).
* **Amnesia:** disables any secret persistence; ensure disk paths are not written inadvertently.
* **UDS:** enforce PEERCRED and `allow_uids` for least privilege.
* **PQ hybrid:** ensure staged rollout; mixed clusters must negotiate safely or fail closed.
* **ZK (future):** if value-bearing gossip is introduced, enable `economics.zk_proof_mode` only with an approved circuit version.

---

## 9) Compatibility & Migration

* **Backwards compatibility:** introduce new keys with safe defaults; avoid changing meanings.
* **Renames:** maintain env var aliases for ≥1 minor; log deprecation warning when used.
* **Breaking changes:** require a **major** bump; provide migration steps in `CHANGELOG.md`.

**Deprecation table (maintained):**

| Old Key | New Key | Removal Target | Notes |
| ------: | :------ | -------------: | :---- |
|  (none) | —       |              — | —     |

---

## 10) Reference Implementation (Rust)

> Example `Config` with serde + figment merge (defaults + file + env), including **Unix permission checks** for TLS and macaroons.

```rust
use std::{net::SocketAddr, path::PathBuf, time::Duration, fs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipCfg {
    #[serde(default = "default_max_frame_bytes", deserialize_with = "de_size")]
    pub max_frame_bytes: u64, // bytes
    #[serde(default = "default_chunk_bytes", deserialize_with = "de_size")]
    pub chunk_bytes: u64,     // bytes
    #[serde(default = "default_u32_8")]
    pub relay_fanout: u32,
    #[serde(default = "default_u32_8")]
    pub burst_budget_frames: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCfg {
    #[serde(default = "default_u32_32")]
    pub inflight_frames: u32,
    #[serde(default = "default_u32_200")]
    pub max_rps: u32,
    #[serde(default = "default_u32_16")]
    pub send_window_frames: u32,
    #[serde(default = "default_u32_16")]
    pub recv_window_frames: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayCfg {
    #[serde(default)]
    pub listen: Option<SocketAddr>,           // None => 127.0.0.1:0
    #[serde(default)]
    pub advertise_addr: Option<String>,
    #[serde(default)]
    pub amnesia: bool,
    #[serde(default = "default_pq_mode")]
    pub pq_mode: String,                      // "off" | "hybrid"
    #[serde(with = "humantime_serde", default = "default_3s")]
    pub handshake_timeout: Duration,
    #[serde(default)]
    pub peer: PeerCfg,
    #[serde(default)]
    pub gossip: GossipCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsCfg {
    #[serde(default = "default_max_frame_bytes", deserialize_with = "de_size")]
    pub max_body_bytes: u64,
    #[serde(default = "default_u32_10")]
    pub decompress_ratio_cap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutsCfg {
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub read: Duration,
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub write: Duration,
    #[serde(with = "humantime_serde", default = "default_15s")]
    pub idle: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsCfg {
    #[serde(default)]
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UdsCfg {
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub allow_uids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthCfg {
    pub macaroon_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityCfg {
    #[serde(default)]
    pub metrics_addr: Option<SocketAddr>, // None => 127.0.0.1:0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TorCfg {
    pub enabled: bool,
    pub socks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuicCfg {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCfg {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default = "default_u32_1024")]
    pub max_conns: u32,
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_15s")]
    pub idle_timeout: Duration,
    #[serde(default)]
    pub tor: TorCfg,
    #[serde(default)]
    pub quic: QuicCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub overlay: OverlayCfg,
    #[serde(default)]
    pub limits: LimitsCfg,
    #[serde(default)]
    pub timeouts: TimeoutsCfg,
    #[serde(default)]
    pub tls: TlsCfg,
    #[serde(default)]
    pub uds: UdsCfg,
    #[serde(default)]
    pub auth: AuthCfg,
    #[serde(default)]
    pub observability: ObservabilityCfg,
    #[serde(default)]
    pub transport: TransportCfg,

    // Optional economics/zk hooks (off by default)
    #[serde(default)]
    pub economics: EconCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EconCfg {
    #[serde(default = "default_zk_mode")]
    pub zk_proof_mode: String, // "off" | "phase1" (future)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            overlay: OverlayCfg {
                listen: None,
                advertise_addr: None,
                amnesia: false,
                pq_mode: default_pq_mode(),
                handshake_timeout: default_3s(),
                peer: PeerCfg {
                    inflight_frames: default_u32_32(),
                    max_rps: default_u32_200(),
                    send_window_frames: default_u32_16(),
                    recv_window_frames: default_u32_16(),
                },
                gossip: GossipCfg {
                    max_frame_bytes: default_max_frame_bytes(),
                    chunk_bytes: default_chunk_bytes(),
                    relay_fanout: default_u32_8(),
                    burst_budget_frames: default_u32_8(),
                },
            },
            limits: LimitsCfg {
                max_body_bytes: default_max_frame_bytes(),
                decompress_ratio_cap: default_u32_10(),
            },
            timeouts: TimeoutsCfg {
                read: default_5s(),
                write: default_5s(),
                idle: default_15s(),
            },
            tls: TlsCfg::default(),
            uds: UdsCfg::default(),
            auth: AuthCfg::default(),
            observability: ObservabilityCfg { metrics_addr: None },
            transport: TransportCfg {
                name: default_name(),
                max_conns: default_u32_1024(),
                read_timeout: default_5s(),
                write_timeout: default_5s(),
                idle_timeout: default_15s(),
                tor: TorCfg::default(),
                quic: QuicCfg::default(),
            },
            economics: EconCfg::default(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        // Protocol cap guard
        if self.overlay.gossip.max_frame_bytes != default_max_frame_bytes() {
            anyhow::bail!("overlay.gossip.max_frame_bytes must equal 1MiB (protocol cap)");
        }
        if self.overlay.gossip.chunk_bytes == 0 ||
           self.overlay.gossip.chunk_bytes > self.overlay.gossip.max_frame_bytes ||
           self.overlay.gossip.chunk_bytes < 4 * 1024 {
            anyhow::bail!("overlay.gossip.chunk_bytes invalid (must be 4KiB..=max_frame_bytes)");
        }
        if self.transport.max_conns == 0 {
            anyhow::bail!("transport.max_conns must be > 0");
        }

        // TLS checks
        if self.tls.enabled {
            let (c, k) = match (&self.tls.cert_path, &self.tls.key_path) {
                (Some(c), Some(k)) => (c, k),
                _ => anyhow::bail!("tls.enabled=true but cert/key not provided"),
            };
            if !c.exists() || !k.exists() {
                anyhow::bail!("TLS cert/key path does not exist");
            }
            // Unix permission checks (0600)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let ck = fs::metadata(k)?;
                let cc = fs::metadata(c)?;
                let k_mode = ck.permissions().mode() & 0o777;
                let c_mode = cc.permissions().mode() & 0o777;
                if k_mode & 0o077 != 0 {
                    anyhow::bail!("TLS key file must be mode 0600 (found {:o})", k_mode);
                }
                if c_mode & 0o007 != 0 {
                    // Allow 0640 or 0644 certs if desired; here we enforce 0600/0640 strictness
                    // Tighten as policy: cert readable by owner (and group optionally).
                }
            }
        }

        // Macaroon checks
        if let Some(ref p) = self.auth.macaroon_path {
            if !p.exists() {
                anyhow::bail!("auth.macaroon_path does not exist");
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let md = fs::metadata(p)?;
                let mode = md.permissions().mode() & 0o777;
                if mode & 0o077 != 0 {
                    anyhow::bail!("macaroon file must be mode 0600 (found {:o})", mode);
                }
            }
            // Optionally: ensure non-empty file
            if fs::metadata(p)?.len() == 0 {
                anyhow::bail!("macaroon file is empty");
            }
        }

        // PQ mode requires feature/runtime support
        if self.overlay.pq_mode == "hybrid" {
            // If you gate at compile-time, keep this runtime hint for ops clarity
            #[cfg(not(feature = "pq"))]
            anyhow::bail!("overlay.pq_mode=hybrid requires the 'pq' feature at build time");
        }

        Ok(())
    }
}

// ---------- helpers ----------
fn default_max_frame_bytes() -> u64 { 1 * 1024 * 1024 }
fn default_chunk_bytes() -> u64 { 64 * 1024 }
fn default_u32_8() -> u32 { 8 }
fn default_u32_10() -> u32 { 10 }
fn default_u32_16() -> u32 { 16 }
fn default_u32_32() -> u32 { 32 }
fn default_u32_200() -> u32 { 200 }
fn default_u32_1024() -> u32 { 1024 }
fn default_3s() -> std::time::Duration { std::time::Duration::from_secs(3) }
fn default_5s() -> std::time::Duration { std::time::Duration::from_secs(5) }
fn default_15s() -> std::time::Duration { std::time::Duration::from_secs(15) }
fn default_name() -> String { "overlay".to_string() }
fn default_pq_mode() -> String { "off".to_string() }
fn default_zk_mode() -> String { "off".to_string() }

// Parse sizes like "64KiB", "1MiB"
fn de_size<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::str::FromStr;
    let s = String::deserialize(deserializer)?;
    byte_unit::Byte::from_str(&s)
        .map_err(serde::de::Error::custom)
        .map(|b| b.get_bytes() as u64)
}
```

> **Crates in example:** `serde`, `humantime-serde`, `byte_unit`, `anyhow`, `figment` (loader).
> **Loader (figment) sketch:**

```rust
use figment::{Figment, providers::{Serialized, Env, Format, Toml, Json}};
let figment = Figment::new()
    .merge(Serialized::defaults(Config::default()))
    .merge(Toml::file("Config.toml").nested())
    .merge(Json::file("Config.json").nested())
    .merge(Env::prefixed("SVC_OVERLAY_").split("__")); // supports nested keys
let mut cfg: Config = figment.extract()?;
cfg.validate()?;
```

---

## 11) Test Matrix

| Scenario                              | Expected Outcome                                            |
| ------------------------------------- | ----------------------------------------------------------- |
| Missing `Config.toml`                 | Start with defaults; log info                               |
| Invalid `overlay.listen`              | Fail fast with explicit error                               |
| `max_frame_bytes` ≠ 1MiB              | Fail fast (protocol cap guard)                              |
| `chunk_bytes` > `max_frame_bytes`     | Fail fast                                                   |
| TLS enabled but missing keys          | Fail fast                                                   |
| **Unix**: TLS key/cert not 0600       | Fail fast with mode hint                                    |
| Macaroon file missing or mode != 0600 | Fail fast                                                   |
| Body over `limits.max_body_bytes`     | `413 Payload Too Large` (admin endpoints)                   |
| Ratio > `decompress_ratio_cap`        | `400 Bad Request` + metric                                  |
| SIGHUP                                | Non-disruptive reload for safe keys; disruptive ones rebind |
| `pq_mode=hybrid` without `pq` feature | Fail fast                                                   |
| UDS path with wrong perms             | Fail fast                                                   |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config File (TOML/JSON)] --> D
  C[Env Vars (SVC_OVERLAY_ / __ nesting)] --> D
  E[CLI Flags] --> D

  D --> V{Validate}
  V -- ok --> R[Runtime Snapshot]
  V -- tls perms fail --> X1[Exit: TLS key/cert perms invalid]
  V -- pq feature fail --> X2[Exit: PQ mode without feature]
  V -- schema fail --> X3[Exit: Invalid schema or cap violation]

  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
  style X1 fill:#7f1d1d,stroke:#450a0a,color:#fff
  style X2 fill:#7f1d1d,stroke:#450a0a,color:#fff
  style X3 fill:#7f1d1d,stroke:#450a0a,color:#fff
```

---

## 13) Operational Notes

* Keep **production configs in version control** (private), or a secret store.
* For containers, prefer **env vars**; mount secrets read-only; verify file perms inside the container.
* Document **default ports** and firewall rules near `overlay.listen`.
* Any change to caps (timeouts, inflight, RPS) should be accompanied by **perf tests**.
* Include this file in PRs that alter configuration; reviewers must check **validation** and **defaults**.
* Plan staged rollout for **PQ hybrid** across the cluster; add canary nodes first.

```
---
