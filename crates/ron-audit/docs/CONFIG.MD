
---

````markdown
---
title: Configuration — ron-audit
crate: ron-audit
owner: Stevan White
last-reviewed: 2025-10-07
status: draft
template_version: 1.0
---

# Configuration — ron-audit

This document defines **all configuration** for `ron-audit`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md` and `docs/SECURITY.md`.

> **Tiering**
> - **Library crate (this crate):** exposes a typed `Config` and helpers; **does not** bind sockets or host endpoints.
> - **Service crates (hosts):** import `ron-audit` and surface these configs as env/file/flags.

---

## 1) Sources & Precedence (Authoritative)

`ron-audit` ingests config via the host (which may read file/env/flags). Effective precedence (highest wins):

1. **Process flags** (host CLI)  
2. **Environment variables**  
3. **Config file** (e.g., `Config.toml` of the host service)  
4. **Built-in defaults** (hard-coded in `ron-audit`)

> On dynamic reload, recompute under the same precedence.

**File formats:** TOML (preferred), JSON (optional).  
**Env prefix (suggested for hosts):** `RON_AUDIT_`.

---

## 2) Quickstart Examples

### 2.1 Minimal (Micronode-style, RAM-only with strict bounds)
```bash
RON_AUDIT_SINK=ram \
RON_AUDIT_QUEUE_DEPTH=1024 \
RON_AUDIT_MAX_ATTRS_BYTES=1KiB \
RON_AUDIT_MAX_RECORD_BYTES=4KiB \
RON_AUDIT_PRIVACY_IP_MODE=drop \
RON_AUDIT_AMNESIA=true \
cargo run -p svc-gateway
````

### 2.2 Macronode (durable WAL + periodic checkpoints & exports)

```bash
RON_AUDIT_SINK=wal \
RON_AUDIT_WAL_DIR=/var/lib/ron/audit \
RON_AUDIT_WAL_ROTATE_BYTES=128MiB \
RON_AUDIT_WAL_FSYNC=on_checkpoint \
RON_AUDIT_EXPORT_DIR=/var/lib/ron/audit/exports \
RON_AUDIT_CHECKPOINT_EVERY=50000 \
RON_AUDIT_EXPORT_EVERY=15m \
RON_AUDIT_CHECKPOINT_SIGNER_KEY_ID=ops-hsm-1 \
RON_AUDIT_CHECKPOINT_ALG=ed25519 \
RON_AUDIT_QUEUE_DEPTH=8192 \
RON_AUDIT_WORKERS=2 \
cargo run -p svc-storage
```

### 2.3 Config file (TOML) embedded by a host service

```toml
# audit.toml
sink = "ram"        # "ram" | "wal" | "export" | "wal+export"

# Global bounds & flow-control
queue_depth        = 2048
workers            = 1
max_attrs_bytes    = "1KiB"
max_record_bytes   = "4KiB"
shed_policy        = "drop"     # "drop" | "block" (block is forbidden in hot paths; only in tests)

# Privacy / redaction
[privacy]
ip_mode        = "prefix"       # "drop" | "prefix" | "hash"
hash_salt_path = "/etc/ron/audit/ip_hash_salt"  # required when ip_mode="hash"

# WAL (durable) sink
[wal]
dir            = "/var/lib/ron/audit"
rotate_bytes   = "128MiB"
fsync          = "on_checkpoint"   # "always" | "on_checkpoint" | "never" (never is test-only)

# Export + Checkpoints (Macronode)
[export]
dir               = "/var/lib/ron/audit/exports"
checkpoint_every  = 50000          # records
export_every      = "15m"
signer_key_id     = "ops-hsm-1"
alg               = "ed25519"      # tag only; signing is host-provided

# Streams (optional special-casing)
[streams."ingress"]
queue_depth      = 4096

[streams."policy"]
queue_depth      = 1024
```

---

## 3) Schema (Typed, With Defaults)

> Durations accept `ms`, `s`, `m`, `h`. Sizes accept `B`, `KB`, `MB`, `MiB`, etc.
> Env vars use `RON_AUDIT_` prefix.

| Key / Env Var                                                 | Type     | Default         | Description                                               | Security Notes                  |                |                    |
| ------------------------------------------------------------- | -------- | --------------- | --------------------------------------------------------- | ------------------------------- | -------------- | ------------------ |
| `sink` / `RON_AUDIT_SINK`                                     | enum     | `ram`           | Audit sink strategy: `ram`, `wal`, `export`, `wal+export` | `ram` required in amnesia mode  |                |                    |
| `queue_depth` / `RON_AUDIT_QUEUE_DEPTH`                       | u32      | `1024`          | Bounded enqueue size per worker                           | Prevents unbounded memory       |                |                    |
| `workers` / `RON_AUDIT_WORKERS`                               | u16      | `1`             | Sink worker threads/tasks                                 | Keep small to reduce contention |                |                    |
| `max_attrs_bytes` / `RON_AUDIT_MAX_ATTRS_BYTES`               | size     | `1KiB`          | Max serialized `attrs` size (IDB [I-9])                   | DoS guard                       |                |                    |
| `max_record_bytes` / `RON_AUDIT_MAX_RECORD_BYTES`             | size     | `4KiB`          | Max canonical record size excl. `self_hash` (IDB [I-9])   | DoS guard                       |                |                    |
| `shed_policy` / `RON_AUDIT_SHED_POLICY`                       | enum     | `drop`          | `drop` (shed on full) or `block` (test only)              | Never `block` on hot paths      |                |                    |
| `amnesia` / `RON_AUDIT_AMNESIA`                               | bool     | `true`          | When `true`, forbid disk writes silently                  | Enforced in Micronode           |                |                    |
| `privacy.ip_mode` / `RON_AUDIT_PRIVACY_IP_MODE`               | enum     | `drop`          | `drop`, `prefix`(/24,/64), `hash`(salted)                 | PII minimization                |                |                    |
| `privacy.hash_salt_path` / `RON_AUDIT_PRIVACY_HASH_SALT_PATH` | path     | `""`            | Required if `ip_mode=hash`                                | File perms 0600                 |                |                    |
| `wal.dir` / `RON_AUDIT_WAL_DIR`                               | path     | `""`            | WAL directory                                             | Dir 0700                        |                |                    |
| `wal.rotate_bytes` / `RON_AUDIT_WAL_ROTATE_BYTES`             | size     | `128MiB`        | WAL segment rotation threshold                            | Prevent giant files             |                |                    |
| `wal.fsync` / `RON_AUDIT_WAL_FSYNC`                           | enum     | `on_checkpoint` | `always`                                                  | `on_checkpoint`                 | `never` (test) | Durability vs perf |
| `export.dir` / `RON_AUDIT_EXPORT_DIR`                         | path     | `""`            | Export/manifest directory                                 | Dir 0700                        |                |                    |
| `export.checkpoint_every` / `RON_AUDIT_CHECKPOINT_EVERY`      | u64      | `50000`         | Records per Merkle checkpoint                             | Larger → fewer writes           |                |                    |
| `export.export_every` / `RON_AUDIT_EXPORT_EVERY`              | duration | `15m`           | Time-based export cadence                                 | Ops SLO                         |                |                    |
| `export.signer_key_id` / `RON_AUDIT_CHECKPOINT_SIGNER_KEY_ID` | string   | `""`            | Host key id for checkpoint signatures                     | No private keys in this crate   |                |                    |
| `export.alg` / `RON_AUDIT_CHECKPOINT_ALG`                     | string   | `ed25519`       | Signature alg tag only (host-provided ops)                | PQ-ready tag later              |                |                    |
| `streams.<name>.queue_depth`                                  | u32      | inherits        | Optional per-stream override                              | Finer control                   |                |                    |

> **N/A for library:** `bind_addr`, `metrics_addr`, TLS ports, etc. remain in host services.

---

## 4) Validation Rules (Fail-Closed)

On initialization or reload:

* **Amnesia:** if `amnesia=true` then `sink` **must be** `ram` or `export` (export may buffer to RAM then hand off via host hook; no disk writes in this crate).
* **Bounds:** `max_attrs_bytes ≥ 256B`, `max_record_bytes ≥ max_attrs_bytes + 512B`.
* **Queue:** `queue_depth > 0`, `workers ∈ [1..8]`.
* **Privacy:** if `privacy.ip_mode="hash"`, `hash_salt_path` must exist, be non-empty, file mode `0600`.
* **WAL:** if `sink` includes `wal`, then `wal.dir` must exist (dir `0700`), writable; `wal.rotate_bytes ≥ 8MiB`. If `wal.fsync="never"`, mark **TEST ONLY** and reject in `profile=prod`.
* **Export:** if `sink` includes `export`, `export.dir` must exist (dir `0700`); `checkpoint_every ≥ 1000`; for signed checkpoints, `signer_key_id` non-empty; `alg` non-empty tag.

**On violation:** return error (lib). Hosts should **fail fast** at boot with a structured message.

---

## 5) Dynamic Reload (If Supported by Host)

* **Trigger:** SIGHUP in host **or** a kernel bus `ConfigUpdated { version }`.
* **Semantics:**

  * **Non-disruptive:** bounds, queue depth (new applies to future enqueues), workers (graceful resize), privacy mode.
  * **Potentially disruptive:** switching `sink` type; enabling `wal`/`export` may spin new workers.
* **Atomicity:** Build new `Config` snapshot and **swap** under a mutex; do not hold `.await` while swapping.
* **Audit:** Host emits `KernelEvent::ConfigUpdated { version }` with a non-secret diff in logs.

---

## 6) CLI Flags (Canonical, for Hosts)

> `ron-audit` is lib-only; hosts SHOULD expose a subset of these:

```
--audit-config <path>          # Load audit.toml
--audit-sink <ram|wal|export|wal+export>
--audit-queue-depth <n>
--audit-workers <n>
--audit-max-attrs <size>
--audit-max-record <size>
--audit-shed <drop|block>
--audit-amnesia                 # boolean
--audit-privacy-ip <drop|prefix|hash>
--audit-privacy-hash-salt <path>
--audit-wal-dir <path>
--audit-wal-rotate <size>
--audit-wal-fsync <always|on_checkpoint|never>
--audit-export-dir <path>
--audit-checkpoint-every <n>
--audit-export-every <dur>
--audit-checkpoint-signer <key-id>
--audit-checkpoint-alg <tag>
```

---

## 7) Feature Flags (Cargo)

| Feature  | Default | Effect                                                    |
| -------- | :-----: | --------------------------------------------------------- |
| `export` |   off   | Enable export helpers & manifest types                    |
| `wal`    |   off   | Enable WAL sink impls                                     |
| `kameo`  |   off   | Optional actor integration (host orchestration)           |
| `cli`    |   off   | Host-side parsing helpers (behind a small adapter module) |

> Keep `tls`/networking out of this crate (host concerns).

---

## 8) Security Implications

* **Amnesia**: With `amnesia=true`, disk writes are forbidden; `wal` is rejected.
* **File perms**: `wal.dir` and `export.dir` must be owned by the service user, `0700`. Manifest and segment files `0600`.
* **Keys**: This crate holds **no private keys**; `signer_key_id`/`alg` are tags only (host signs checkpoints).
* **PII**: Prefer `ip_mode=drop`. If `prefix`, store only `/24` (IPv4) or `/64` (IPv6). If `hash`, require salt on disk (0600) and rotate periodically.
* **Shedding**: Always prefer `shed_policy=drop` in production to protect SLOs.

See `docs/SECURITY.md` and `docs/IDB.md` (§ invariants) for the full model.

---

## 9) Compatibility & Migration

* **Additive keys** come with safe defaults; hosts can opt-in gradually.
* **Renames**: keep env aliases ≥ 1 minor and log a warning when used.
* **Breaking**: changes that affect persistence layout (e.g., WAL segment format) or canonicalization require a **major** bump and a migration tool in the host repo.

**Deprecation table**

|    Old Key | New Key   | Removal Target | Notes                          |
| ---------: | :-------- | :------------: | :----------------------------- |
| `WAL_PATH` | `wal.dir` |     v2.0.0     | Align naming with other crates |

---

## 10) Reference Implementation (Rust)

> Paste-ready minimal config types for `ron-audit` (host may wrap or merge with its own).

```rust
use std::{path::PathBuf, time::Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkKind { Ram, Wal, Export, WalExport }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShedPolicy { Drop, Block } // Block is for tests only

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsyncPolicy { Always, OnCheckpoint, Never } // Never is test-only

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpMode { Drop, Prefix, Hash }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrivacyCfg {
    #[serde(default = "default_ip_mode")]
    pub ip_mode: IpMode,
    pub hash_salt_path: Option<PathBuf>, // required for IpMode::Hash
}
fn default_ip_mode() -> IpMode { IpMode::Drop }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalCfg {
    pub dir: Option<PathBuf>,
    #[serde(default = "default_rotate")]
    pub rotate_bytes: humansize::Size,   // use your size type or u64 bytes
    #[serde(default = "default_fsync")]
    pub fsync: FsyncPolicy,
}
fn default_rotate() -> humansize::Size { humansize::Size::from_bytes(128 * 1024 * 1024) }
fn default_fsync() -> FsyncPolicy { FsyncPolicy::OnCheckpoint }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportCfg {
    pub dir: Option<PathBuf>,
    #[serde(default = "default_checkpoint_every")]
    pub checkpoint_every: u64,
    #[serde(with = "humantime_serde", default = "default_export_every")]
    pub export_every: Duration,
    pub signer_key_id: Option<String>,
    #[serde(default = "default_alg")]
    pub alg: String,
}
fn default_checkpoint_every() -> u64 { 50_000 }
fn default_export_every() -> Duration { Duration::from_secs(15 * 60) }
fn default_alg() -> String { "ed25519".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamOverride {
    pub queue_depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_sink")]
    pub sink: SinkKind,
    #[serde(default = "default_queue")]
    pub queue_depth: u32,
    #[serde(default = "default_workers")]
    pub workers: u16,
    #[serde(default = "default_max_attrs")]
    pub max_attrs_bytes: humansize::Size,
    #[serde(default = "default_max_record")]
    pub max_record_bytes: humansize::Size,
    #[serde(default = "default_shed")]
    pub shed_policy: ShedPolicy,
    #[serde(default = "default_amnesia")]
    pub amnesia: bool,
    #[serde(default)]
    pub privacy: PrivacyCfg,
    #[serde(default)]
    pub wal: WalCfg,
    #[serde(default)]
    pub export: ExportCfg,
    /// Optional per-stream overrides
    #[serde(default)]
    pub streams: std::collections::BTreeMap<String, StreamOverride>,
}

fn default_sink() -> SinkKind { SinkKind::Ram }
fn default_queue() -> u32 { 1024 }
fn default_workers() -> u16 { 1 }
fn default_max_attrs() -> humansize::Size { humansize::Size::from_str("1KiB").unwrap() }
fn default_max_record() -> humansize::Size { humansize::Size::from_str("4KiB").unwrap() }
fn default_shed() -> ShedPolicy { ShedPolicy::Drop }
fn default_amnesia() -> bool { true }

impl AuditConfig {
    pub fn validate(&self, profile_prod: bool) -> anyhow::Result<()> {
        use anyhow::{bail, Context as _};
        if self.queue_depth == 0 { bail!("queue_depth must be > 0"); }
        if !(1..=8).contains(&self.workers) { bail!("workers must be in 1..=8"); }
        let attrs = self.max_attrs_bytes.to_bytes();
        let rec   = self.max_record_bytes.to_bytes();
        if attrs < 256 { bail!("max_attrs_bytes must be >= 256B"); }
        if rec < attrs + 512 { bail!("max_record_bytes must be >= max_attrs_bytes + 512B"); }

        // Amnesia posture
        if self.amnesia {
            match self.sink {
                SinkKind::Ram | SinkKind::Export => {},
                _ => bail!("amnesia=true forbids WAL sinks; use sink=ram or export"),
            }
        }

        // Privacy
        if matches!(self.privacy.ip_mode, IpMode::Hash) && self.privacy.hash_salt_path.is_none() {
            bail!("privacy.ip_mode=hash requires privacy.hash_salt_path");
        }

        // WAL
        if matches!(self.sink, SinkKind::Wal | SinkKind::WalExport) {
            let dir = self.wal.dir.as_ref().context("wal.dir is required")?;
            if !dir.exists() { bail!("wal.dir does not exist: {}", dir.display()); }
            if self.wal.rotate_bytes.to_bytes() < 8 * 1024 * 1024 {
                bail!("wal.rotate_bytes must be >= 8MiB");
            }
            if profile_prod && matches!(self.wal.fsync, FsyncPolicy::Never) {
                bail!("wal.fsync=never is forbidden in production");
            }
        }

        // Export
        if matches!(self.sink, SinkKind::Export | SinkKind::WalExport) {
            let dir = self.export.dir.as_ref().context("export.dir is required")?;
            if !dir.exists() { bail!("export.dir does not exist: {}", dir.display()); }
            if self.export.checkpoint_every < 1_000 {
                bail!("export.checkpoint_every must be >= 1000");
            }
            if self.export.signer_key_id.as_deref().unwrap_or("").is_empty() {
                // Allowed if host chooses unsigned checkpoints, but warn at host.
            }
        }
        Ok(())
    }

    /// Resolve per-stream queue depth (override or global).
    pub fn queue_depth_for(&self, stream: &str) -> u32 {
        self.streams.get(stream).and_then(|s| s.queue_depth).unwrap_or(self.queue_depth)
    }
}
```

---

## 11) Test Matrix

| Scenario                                         | Expected Outcome                                          |
| ------------------------------------------------ | --------------------------------------------------------- |
| `amnesia=true` with `sink=wal`                   | Validation error (forbidden)                              |
| `max_attrs_bytes=2KiB` & `max_record_bytes=2KiB` | Validation error (record < attrs+512)                     |
| `privacy.ip_mode=hash` w/o `hash_salt_path`      | Validation error                                          |
| `wal.sink` with missing `wal.dir`                | Validation error                                          |
| `wal.fsync=never` and `profile_prod=true`        | Validation error                                          |
| Queue overflow at runtime                        | Shed with `audit_drop_total{reason="audit_backpressure"}` |
| Switch `sink=ram → wal` at reload                | New workers started; old drain; chain preserved           |
| Export checkpoint timer fires                    | Manifest + root written; next batch starts                |

---

## 12) Mermaid — Config Resolution Flow (Host)

```mermaid
flowchart TB
  A[Defaults (ron-audit)] --> D[Merge]
  B[File: audit.toml] --> D
  C[Env: RON_AUDIT_*] --> D
  E[CLI: --audit-*] --> D
  D --> V{Validate}
  V -- ok --> S[Snapshot to sink workers]
  V -- fail --> X[Fail fast]
  style S fill:#0b7285,stroke:#0b7285,color:#fff
```

---

## 13) Operational Notes

* Keep `wal.dir` and `export.dir` on **dedicated volumes** with monitoring for space and IO latency.
* Back up **export manifests**; they are the index to your audit evidence.
* Treat `privacy.hash_salt_path` as a **secret** (0600) and rotate periodically; maintain a mapping table for audits if required by policy.
* Prefer **`shed_policy=drop`** in production; blocking can violate SLOs under surge.
* For Micronode, ship a **RAM-only** default profile and a CLI toggle to enable periodic offload handled by the host (not this crate).

```
---
```
