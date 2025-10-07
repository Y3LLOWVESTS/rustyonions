---

title: Configuration — oap
crate: oap
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
template_version: 1.0
---------------------

# Configuration — `oap`

This document defines **all configuration** for the `oap` crate, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/SECURITY.md`, and the OAP/1 IDB.

> **Tiering:**
>
> * **Library crate (this crate):** OAP is a **pure protocol library** (framing & envelopes). It has **no ports, sockets, TLS, auth, or metrics endpoints** of its own. Most service-level options are **N/A** here and live in embedding services (e.g., `svc-gateway`, `svc-overlay`).
> * **Service crates:** bind addresses, timeouts, and observability endpoints are configured **in those services**, not in `oap`.

---

## 1) Sources & Precedence (Authoritative)

`oap` itself does not parse process flags or read files. It accepts configuration **as constructor parameters** from the host (service or SDK). If a host wishes to map CLI/env/file into `oap`, the **effective precedence** must be:

1. **Process flags** (CLI) — host-owned
2. **Environment variables** — host-owned
3. **Config file** (TOML/JSON) — host-owned
4. **Built-in defaults** (hard-coded in host, plus OAP constants)

> **Important:** OAP/1 **normative constants** (e.g., `max_frame = 1 MiB`) are **not configurable** anywhere. Hosts must not expose toggles that change protocol invariants.

**Supported file formats (if host uses a file):** TOML (preferred), JSON (optional).
**Path resolution for `--config` (if relative):** host-defined.

**Amnesia mode (micronode):** hosts **SHOULD** avoid disk-based config by default and prefer env/flags; if a file is used, mount it **read-only** and **do not persist secrets**.

---

## 2) Quickstart Examples

### 2.1 Minimal embed (service perspective)

```rust
use oap::{ParserConfig, WriterConfig, OapFramer};

let pcfg = ParserConfig::default();   // strict headers on; constants baked-in
let wcfg = WriterConfig::default();   // credit window defaults; uses host I/O chunk size

let mut framer = OapFramer::new(pcfg, wcfg);
// host wires framer to its transport (ron-transport) and DTOs (ron-proto)
```

### 2.2 Host-side config file (TOML) mapped into OAP (illustrative)

```toml
# service Config.toml — host will map some values into OAP configs
[overlay.io]
stream_chunk_bytes = "64KiB"  # I/O hint; NOT the OAP frame cap

[oap]
strict_headers = true
ack_window_frames = 32
allow_pq_hello_flags = true
```

### 2.3 Host CLI flags (override file/env) — N/A to oap directly

Hosts may expose flags like `--oap-strict-headers=true`; `oap` just receives the final booleans/ints.

---

## 3) Schema (Typed, With Defaults)

> **Scope note:** Only **library-level knobs** that affect framing/validation belong here.
> **Not here:** sockets, TLS, timeouts, body caps, metrics endpoints — those are **service** concerns.

| Key (host → oap)                       | Type                 | Default  | Description                                                                                                                                                         | Security Notes                        |
| -------------------------------------- | -------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `oap.strict_headers`                   | bool                 | `true`   | Reject unknown/duplicate/malformed header fields                                                                                                                    | Fail-early reduces attack surface     |
| `oap.ack_window_frames`                | u32                  | `32`     | Target in-flight window (frames) for `ACK{up_to}` credit accounting. **Align with host’s ledger/rewarder batch sizes where applicable** to avoid settlement stalls. | Backpressure tuning; bounded inflight |
| `oap.allow_pq_hello_flags`             | bool                 | `true`   | Accept opaque PQ capability bits in `HELLO`; OAP/1 semantics unchanged                                                                                              | Interop toggle; never alters flow     |
| `oap.enforce_obj_header_on_objects`    | bool                 | `true`   | Require `obj:"b3:<hex>"` when DATA bytes are a content-addressed object                                                                                             | Integrity boundary clarity            |
| `oap.seq_rollover_policy`              | enum(`error`,`wrap`) | `error`  | Behavior if `seq` would overflow host’s 64-bit counter                                                                                                              | Prefer `error` to avoid ambiguity     |
| `overlay.io.stream_chunk_bytes` (host) | size (hint)          | `64 KiB` | Host I/O **hint** only; OAP frame cap remains **1 MiB** (non-configurable)                                                                                          | Misuse must not bypass cap            |

**Normative constants (baked into OAP/1, not configurable):**

* `max_frame_bytes = 1_048_576` (**1 MiB**)
* Envelope set: `HELLO, START, DATA, END, ACK, ERROR`

---

## 4) Validation Rules (Fail-Closed)

Applied by `oap` at construction/use, and by hosts before passing values:

* `ack_window_frames >= 1 && ack_window_frames <= 1024` (reject outside range).
* `strict_headers == true` in production profiles (hosts may relax only in tests).
* If `enforce_obj_header_on_objects = false`, host **must** enforce integrity at materialization boundaries (storage/overlay); otherwise reject.
* `allow_pq_hello_flags` only admits **opaque flags**; it must not change framing/state machines. If flags attempt to alter OAP state machines, treat as a **configuration error** and fail fast.
* `stream_chunk_bytes` is a **host I/O hint**; changing it **must not** impact OAP’s frame parsing or limits.

**On violation:** return structured `OapConfigError` (libs). Service hosts should fail fast (non-zero exit).

---

## 5) Dynamic Reload (If Supported by Host)

`oap` does **not** watch files or handle signals. If the host supports reload:

* **Trigger:** SIGHUP or bus `ConfigUpdated { version }` (host feature).
* **Reload semantics in OAP:**

  * Safe to update: `ack_window_frames`, `strict_headers` (tightening is safe; loosening allowed in non-prod).
  * No-op in OAP: transport timeouts, bind addresses, TLS keys (host-owned).
  * **Never** change OAP/1 invariants at runtime (e.g., frame cap).
* **Atomicity:** Host creates a new `ParserConfig`/`WriterConfig`, swaps under a lock **without** holding `.await`.

---

## 6) CLI Flags (Canonical)

**N/A for `oap` directly.** Hosts **may** expose flags and map them into OAP configs. Suggested flag names if a host wants parity:

```
--oap-strict-headers=<bool>
--oap-ack-window-frames=<u32>
--oap-allow-pq-hello-flags=<bool>
--oap-enforce-obj-header=<bool>
--overlay-stream-chunk-bytes=<size>   # host I/O hint; not the OAP cap
```

---

## 7) Feature Flags (Cargo)

| Feature       | Default | Effect                                                                 |
| ------------- | :-----: | ---------------------------------------------------------------------- |
| `pq`          |   off   | Enables parsing/forwarding of opaque PQ capability bits in `HELLO`     |
| `serde`       |    on   | Enables serde for configs/DTOs where applicable (host interop)         |
| `std`         |    on   | Uses `std`; `no_std` (future) may restrict alloc-heavy paths           |
| `dev-vectors` |   off   | Includes extra conformance/fuzz vectors for tests (not for production) |

> No `cli` feature here; CLI parsing belongs to hosts. TLS/transport features live in `ron-transport`.

---

## 8) Security Implications

* **Fail-early posture:** `strict_headers=true` curbs malicious/ambiguous envelopes (recommended for prod).
* **Integrity boundary:** `enforce_obj_header_on_objects=true` ensures DATA carrying objects is explicitly bound to `b3:<hex>`. Verification remains at materialization sites (storage/overlay).
* **Backpressure:** Reasonable `ack_window_frames` prevents unbounded memory/latency; hosts must align with their connection limits.
* **PQ flags:** Accepting opaque PQ bits must **not** change parsing/flow. All changes to cryptography occur below OAP (transport). PQ capability flags are non-sensitive but **MUST NOT** be propagated to unauthenticated logs or untrusted headers; treat them as opaque toggles for lower layers.
* **Non-configurable cap:** The **1 MiB** frame cap is protocol law; hosts cannot override it.

See `docs/SECURITY.md` and IDB §Invariants/Anti-Scope.

---

## 9) Compatibility & Migration

* Additive changes: introduce new knobs with safe defaults and `#[serde(default)]`.
* Renames: provide de/serialization aliases at host boundary for ≥1 minor release.
* Breaking changes to OAP configuration semantics require **major version** and explicit migration notes.
* Protocol constants (frame cap, envelopes) are **frozen** for OAP/1.

---

## 10) Reference Implementation (Rust)

> **Illustrative** config shapes passed **into** `oap`. Hosts may wrap/derive these from their own config systems.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    #[serde(default = "default_true")]
    pub strict_headers: bool,
    #[serde(default = "default_enforce_obj_header")]
    pub enforce_obj_header_on_objects: bool,
    #[serde(default = "default_ack_window")]
    pub ack_window_frames: u32,
    #[serde(default = "default_allow_pq")]
    pub allow_pq_hello_flags: bool,
    #[serde(default = "default_seq_rollover")]
    pub seq_rollover_policy: SeqRolloverPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeqRolloverPolicy { Error, Wrap }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WriterConfig {
    // Writer-side knobs if any arise later; kept minimal for OAP/1
}

const MAX_FRAME_BYTES: usize = 1_048_576; // 1 MiB, normative cap (not configurable)

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            strict_headers: true,
            enforce_obj_header_on_objects: true,
            ack_window_frames: 32,
            allow_pq_hello_flags: true,
            seq_rollover_policy: SeqRolloverPolicy::Error,
        }
    }
}

fn default_true() -> bool { true }
fn default_enforce_obj_header() -> bool { true }
fn default_ack_window() -> u32 { 32 }
fn default_allow_pq() -> bool { true }
fn default_seq_rollover() -> SeqRolloverPolicy { SeqRolloverPolicy::Error }

impl ParserConfig {
    pub fn validate(&self) -> Result<(), OapConfigError> {
        if !(1..=1024).contains(&self.ack_window_frames) {
            return Err(OapConfigError::InvalidAckWindow(self.ack_window_frames));
        }
        // OAP/1 invariants (e.g., MAX_FRAME_BYTES) are constants and not user-configurable.
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum OapConfigError {
    #[error("ack_window_frames out of range: {0}")]
    InvalidAckWindow(u32),
}
```

---

## 11) Test Matrix

| Scenario                                 | Expected Outcome                                        |
| ---------------------------------------- | ------------------------------------------------------- |
| `ack_window_frames = 0`                  | `OapConfigError::InvalidAckWindow`                      |
| Unknown header encountered (strict=true) | Frame reject; `oap_rejects_total{reason="unknown_hdr"}` |
| DATA without `obj` when object expected  | Frame reject; `oap_rejects_total{reason="missing_obj"}` |
| PQ flags in HELLO (allowed=true)         | Accepted (opaque); normal flow                          |
| PQ flags in HELLO (allowed=false)        | Reject with `bad_hello`                                 |
| PQ flags alter state machine (shouldn’t) | **Configuration error at init**; no runtime change      |
| Host changes chunk hint at runtime       | No protocol change; I/O behavior shifts only in host    |

---

## 12) Mermaid — Config Resolution Flow (Host + OAP)

```mermaid
flowchart TB
  subgraph Host (e.g., svc-gateway / svc-overlay)
    A[Defaults] --> D[Merge]
    B[Config File] --> D
    C[Env Vars] --> D
    E[CLI Flags] --> D
    D --> V{Validate Host}
  end
  V -- ok --> P[Build ParserConfig/WriterConfig]
  P --> O[OAP Framer]
  V -- fail --> X[Exit / Error]
  O --> R[Runtime]
  F[[OAP/1 Constants\nmax_frame=1MiB, envelopes fixed]]:::consts
  F -. non-configurable .- O
  classDef consts fill:#f1f5f9,stroke:#94a3b8,color:#0f172a
```

---

## 13) Operational Notes

* Treat `oap` as **pure lib**: configure transports, TLS, timeouts, and observability **in your service**, then pass only the small set of OAP knobs shown here.
* Keep production defaults **strict**; relax only in tests/interop labs.
* Document your host’s I/O chunk size separately; never conflate it with the **1 MiB** frame cap.
* Prefer env/flags in micronode (amnesia) profiles; if files are used, mount **read-only** and keep secrets out.
* Include this file in PRs that modify OAP constructor parameters or validation.
