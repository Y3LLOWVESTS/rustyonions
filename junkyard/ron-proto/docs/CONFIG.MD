
---

# ⚙️ Configuration — ron-proto

---

title: Configuration — ron-proto
crate: ron-proto
owner: Stevan White
last-reviewed: 2025-09-28
status: draft
template_version: 1.1
---------------------

`ron-proto` is a **library** crate (pure DTOs; no I/O). This document defines **all configuration** for `ron-proto`, including sources, precedence, schema (types/defaults), validation, feature flags, reload posture, facet impact, and security implications. It complements `README.md`, `docs/IDB.md`, `docs/OBSERVABILITY.md`, and `docs/SECURITY.md`.

> **Tiering:** Library crate — no listeners, ports, `/healthz`, or metrics endpoints here (N/A). Configuration controls **wire format, schema strictness, compatibility windows, and test vectors** only.

---

## 1) Sources & Precedence (Authoritative)

`ron-proto` does **not** read process flags or files at runtime on its own. Configuration is provided as:

1. **Programmatic overrides** from the caller (SDK/service) via a `ProtoConfig` struct.
2. **Environment variables (optional; dev/tests only)** — parsed by helper `ProtoConfig::from_env_prefixed("RON_PROTO_")` when explicitly called by the host.
3. **Built-in defaults** (hard-coded in this crate).

**Precedence (highest wins):**

1. Programmatic overrides → 2) Environment variables (opt-in helper) → 3) Built-in defaults.

> `ron-proto` **never** watches files or signals. Any “reload” is the **host** swapping a new `ProtoConfig` snapshot it owns.

**Supported file formats:** N/A (lib).
**Path resolution for test vectors:** Caller supplies absolute/relative paths; `ron-proto` does not search the filesystem implicitly.

---

## 2) Quickstart Examples

### 2.1 Minimal (Rust host sets explicit config)

```rust
use ron_proto::config::{ProtoConfig, WireFormat, OapVersion};

let cfg = ProtoConfig {
    wire_format: WireFormat::Json,        // or Bincode/Msgpack if enabled
    oap_version: OapVersion::V1,          // protocol constants bound to OAP/1
    accept_unknown_fields: false,         // deny by default (strict DTO hygiene)
    strict_enums: true,                   // unknown enum variants → error
    limits: Default::default(),           // 1 MiB frame, 64 KiB chunk
    pq_mode: ron_proto::config::PqMode::Off,
    ids: Default::default(),
    compat: Default::default(),
    addr_prefix: "b3:".into(),            // canonical content id prefix
    hex_lowercase: true,                  // enforce lowercase hex policy
    zeroize_transient: true,              // Micronode amnesia=ON → enable
    test_vectors_dir: None,               // host can set for interop tests
};
cfg.validate().expect("invalid ron-proto config");
```

### 2.2 From environment (dev/tests only)

```rust
let cfg = ron_proto::config::ProtoConfig::from_env_prefixed("RON_PROTO_")?; // explicit call
```

> Note: No CLI or config files are parsed by `ron-proto`. That’s the host’s job.

---

## 3) Schema (Typed, With Defaults & Facet Impact)

> **Spec cross-refs:** OAP/1 envelope cap = **1 MiB**; storage/streaming chunk ≈ **64 KiB**. See Interop Blueprint §3 (OAP/1).
> **Env prefix convention:** `RON_PROTO_` (only used if host *chooses* to call `from_env_prefixed`).

| Key / Env Var                                | Type                          | Default             | Description                                                               | Security Notes                                                 | Facet Impact                    |
| -------------------------------------------- | ----------------------------- | ------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------- | ------------------------------- |
| `wire_format` / `RON_PROTO_WIRE_FORMAT`      | enum `Json\|Bincode\|Msgpack` | `Json`              | Serialization for **payload DTOs** in SDK/service internals or vectors.   | JSON favored for auditability; parity required across SDKs.    | DX, Graph/Search (vectors)      |
| `oap_version` / `RON_PROTO_OAP_VERSION`      | enum `V1`                     | `V1`                | Protocol envelope constants (OAP/1).                                      | Altering requires cross-component compatibility review.        | Interop, RES                    |
| `accept_unknown_fields` / `…_ACCEPT_UNKNOWN` | bool                          | `false`             | If `true`, tolerate unknown JSON keys on input **(compat windows only)**. | Weakens strictness; avoid in prod.                             | SEC, RES                        |
| `strict_enums` / `…_STRICT_ENUMS`            | bool                          | `true`              | If `true`, unknown enum discriminants are errors.                         | Turning off can mask schema drift.                             | SEC, RES                        |
| `limits.max_frame_bytes` / `…_MAX_FRAME`     | bytes (u32)                   | `1_048_576` (1 MiB) | **OAP/1** frame cap (**must** match spec exact).                          | Raising breaks spec; validation rejects > 1 MiB.               | Media, RES                      |
| `limits.chunk_bytes` / `…_CHUNK`             | bytes (u32)                   | `65_536` (64 KiB)   | Storage/streaming chunk hint (not the OAP frame limit).                   | Too large hurts backpressure; must be ≤ frame cap.             | Media, PERF                     |
| `pq_mode` / `…_PQ_MODE`                      | enum `Off\|Hybrid`            | `Off`               | PQ-hybrid typing exposure for forward-compat DTOs (no crypto here).       | Host must ensure peer interop; never log PQ suite metadata.    | GOV (rollout), SEC              |
| `ids.uuid_version` / `…_UUID_VER`            | enum `V4`                     | `V4`                | UUID flavor for tenant/correlation IDs (when host asks to mint).          | Host typically mints; here only dictates DTO field type.       | DX                              |
| `ids.correlation_bits` / `…_CORR_BITS`       | u8                            | `64`                | Size for correlation IDs when non-UUID paths are used.                    | —                                                              | DX                              |
| `compat.allow_semver_minor` / `…_MINOR_OK`   | bool                          | `true`              | Treat added optional fields as compatible.                                | If `false`, minor additions become errors.                     | GOV, DX                         |
| `addr_prefix` / `…_ADDR_PREFIX`              | string                        | `"b3:"`             | Canonical content ID prefix.                                              | Must remain `b3:` for BLAKE3 content addressing.               | SEC, Interop                    |
| `hex_lowercase` / `…_HEX_LOWERCASE`          | bool                          | `true`              | Enforce lowercase hex for `b3:<hex>` IDs.                                 | Prevents ambiguous decodings/collisions.                       | SEC, DX                         |
| `zeroize_transient` / `…_ZEROIZE_TRANSIENT`  | bool                          | `true`              | Enable zeroization hooks for transient buffers (Micronode amnesia).       | Reduces residual risk in memory for DTO scratch areas.         | SEC                             |
| `test_vectors_dir` / `…_VECTORS_DIR`         | path (string)                 | `None`              | Optional path to interop vectors (dev/CI).                                | Lib only reads if host calls test APIs; no secrets in vectors. | DX, Graph/Search (parity tests) |

**N/A for a lib:** `bind_addr`, `metrics_addr`, timeouts, TLS paths, UDS, macaroon paths, log format.

---

## 4) Validation Rules (Fail-Closed)

`ProtoConfig::validate()` enforces:

* `limits.max_frame_bytes == 1_048_576` (exact) for **OAP/1**. Not exact → **error**.
* `limits.chunk_bytes` in `[4096, 1_048_576]` and **≤ max_frame_bytes**.
* `accept_unknown_fields = false` for public DTOs (compat windows must be temporary and explicit).
* `strict_enums = true` for production (CI may allow test toggles).
* `addr_prefix == "b3:"` and `hex_lowercase == true` (canonical ID policy).
* `pq_mode = Hybrid` requires host to assert peer support (observability flag only; no negotiation here).
* `test_vectors_dir`, when set, must exist and be a directory (if the host invokes vector loading APIs).

On violation: return a **typed error**; `ron-proto` never `exit(1)`.

---

## 5) Dynamic Reload (If Supported)

As a lib, `ron-proto` does **not** own reload semantics. Hosts may:

* Construct a **new** `ProtoConfig`, call `validate()`, then **swap atomically** in their state.
* `ron-proto` provides no global, mutable singletons and holds no locks across `.await`.

**Atomicity guidance for hosts:** store `Arc<ProtoConfig>` and perform whole-snapshot swaps; avoid partial mutations.

---

## 6) CLI Flags (Canonical)

**N/A for a lib crate.** (Hosts may define CLI; if they do, map flags to these fields verbatim.)

---

## 7) Feature Flags (Cargo)

| Feature        | Default | Effect                                                                       |
| -------------- | ------- | ---------------------------------------------------------------------------- |
| `json`         | on      | Enables JSON serde for DTOs (baseline).                                      |
| `bincode`      | off     | Enables `bincode` round-trips for DTOs (tests/SDK internals).                |
| `msgpack`      | off     | Enables `rmp-serde` round-trips.                                             |
| `pq`           | off     | Exposes PQ-hybrid fields in certain envelopes (typing only; no crypto here). |
| `ts-bindings`  | off     | Enable TS bindings export (build-time only).                                 |
| `kameo`        | off     | Test harness integrations where actors exchange DTOs (no runtime effect).    |
| `schema-guard` | off     | Enables schema snapshot tests and compatibility checks.                      |
| `zeroize`      | off     | Enables zeroization helpers for transient buffers.                           |

> Enabling non-JSON formats must not change canonical on-wire **OAP/1** rules; they apply only to host-internal serialization or test vectors.

---

## 8) Security Implications

* **Strict deserialization** is the default: `deny_unknown_fields`, `strict_enums = true`. Loosening these **increases** risk of drift/ambiguity; allow only during controlled migrations.
* **PQ mode** is **typing only** here. Keys/handshakes live in `ron-kms`/`ron-auth`/transport. Do **not** infer security posture from `ron-proto` alone; never log PQ suite parameters.
* **No secrets** are parsed or stored by `ron-proto`. If hosts pass paths (vectors), they are for **tests**; don’t embed secrets in vectors.
* **Canonical IDs**: require `b3:<hex>` lower-case, fixed length; prevents spoofing/ambiguity in logs and metrics.

---

## 9) Compatibility & Migration

* **Backward compatibility policy:** add fields as **optional** with sensible defaults; never reuse or repurpose existing fields.
* **SemVer:** any removal/rename is a **major** version change; minor versions may add optional fields guarded by `compat.allow_semver_minor = true`.
* **Envelope evolution:** use **versioned types** (e.g., `HelloV1`, `HelloV2`) rather than overloading one struct.

**Deprecation table (maintained):**

| Old Field/Type | New Field/Type | Removal Target | Notes                          |
| -------------- | -------------- | -------------: | ------------------------------ |
| –              | –              |              – | (Populate as evolutions occur) |

---

## 10) Reference Implementation (Rust)

```rust
// src/config.rs (sketch: copy-paste ready)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WireFormat { Json, #[cfg(feature="bincode")] Bincode, #[cfg(feature="msgpack")] Msgpack }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OapVersion { V1 }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PqMode { Off, #[cfg(feature="pq")] Hybrid }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Limits {
    pub max_frame_bytes: u32, // MUST be 1_048_576 (OAP/1)
    pub chunk_bytes: u32,     // default 65_536
}
impl Default for Limits {
    fn default() -> Self { Self { max_frame_bytes: 1_048_576, chunk_bytes: 65_536 } }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ids {
    pub uuid_version: String,   // "V4"
    pub correlation_bits: u8,   // 64
}
impl Default for Ids {
    fn default() -> Self { Self { uuid_version: "V4".into(), correlation_bits: 64 } }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Compat { pub allow_semver_minor: bool }
impl Default for Compat { fn default() -> Self { Self { allow_semver_minor: true } } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtoConfig {
    pub wire_format: WireFormat,
    pub oap_version: OapVersion,
    pub accept_unknown_fields: bool,
    pub strict_enums: bool,
    pub limits: Limits,
    pub pq_mode: PqMode,
    pub ids: Ids,
    pub compat: Compat,
    pub addr_prefix: String,        // "b3:"
    pub hex_lowercase: bool,        // true
    pub zeroize_transient: bool,    // true when amnesia=ON
    /// Optional test/interop vector directory (dev/CI only)
    pub test_vectors_dir: Option<std::path::PathBuf>,
}

impl Default for ProtoConfig {
    fn default() -> Self {
        Self {
            wire_format: WireFormat::Json,
            oap_version: OapVersion::V1,
            accept_unknown_fields: false,
            strict_enums: true,
            limits: Limits::default(),
            pq_mode: PqMode::Off,
            ids: Ids::default(),
            compat: Compat::default(),
            addr_prefix: "b3:".into(),
            hex_lowercase: true,
            zeroize_transient: true,
            test_vectors_dir: None,
        }
    }
}

impl ProtoConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.limits.max_frame_bytes != 1_048_576 {
            return Err("OAP/1 requires max_frame_bytes == 1_048_576 (1 MiB)".into());
        }
        if self.limits.chunk_bytes == 0 || self.limits.chunk_bytes > self.limits.max_frame_bytes {
            return Err("chunk_bytes must be > 0 and <= max_frame_bytes".into());
        }
        if !matches!(self.oap_version, OapVersion::V1) {
            return Err("Unsupported OAP version".into());
        }
        if self.addr_prefix != "b3:" { return Err("addr_prefix must be \"b3:\"".into()); }
        if !self.hex_lowercase { return Err("hex_lowercase must be true (canonical id policy)".into()); }
        Ok(())
    }

    /// Optional helper for tests/dev: read from env with a prefix (e.g., "RON_PROTO_").
    pub fn from_env_prefixed(prefix: &str) -> Result<Self, String> {
        let mut cfg = Self::default();
        let up = |k: &str| format!("{prefix}{k}");

        if let Ok(val) = std::env::var(up("WIRE_FORMAT")) {
            cfg.wire_format = match val.as_str() {
                "Json" => WireFormat::Json,
                #[cfg(feature="bincode")] "Bincode" => WireFormat::Bincode,
                #[cfg(feature="msgpack")] "Msgpack" => WireFormat::Msgpack,
                _ => return Err(format!("unknown WIRE_FORMAT={val}")),
            };
        }
        if let Ok(val) = std::env::var(up("ACCEPT_UNKNOWN")) {
            cfg.accept_unknown_fields = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var(up("STRICT_ENUMS")) {
            cfg.strict_enums = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var(up("MAX_FRAME")) {
            cfg.limits.max_frame_bytes = val.parse().map_err(|e| e.to_string())?;
        }
        if let Ok(val) = std::env::var(up("CHUNK")) {
            cfg.limits.chunk_bytes = val.parse().map_err(|e| e.to_string())?;
        }
        if let Ok(val) = std::env::var(up("PQ_MODE")) {
            cfg.pq_mode = match val.as_str() {
                "Off" => PqMode::Off,
                #[cfg(feature="pq")] "Hybrid" => PqMode::Hybrid,
                _ => return Err(format!("unknown PQ_MODE={val}")),
            };
        }
        if let Ok(val) = std::env::var(up("ADDR_PREFIX")) { cfg.addr_prefix = val; }
        if let Ok(val) = std::env::var(up("HEX_LOWERCASE")) {
            cfg.hex_lowercase = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var(up("ZEROIZE_TRANSIENT")) {
            cfg.zeroize_transient = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var(up("VECTORS_DIR")) {
            cfg.test_vectors_dir = Some(val.into());
        }
        cfg.validate()?;
        Ok(cfg)
    }
}
```

---

## 11) Test Matrix

| Scenario                              | Expected Outcome                                                |
| ------------------------------------- | --------------------------------------------------------------- |
| `max_frame_bytes` ≠ 1 MiB             | `validate()` returns error                                      |
| `chunk_bytes` > `max_frame_bytes`     | `validate()` returns error                                      |
| `accept_unknown_fields = true`        | Allowed with warning only during controlled migrations          |
| `strict_enums = false`                | Unknown enum → error only if host disables strict paths         |
| `pq_mode = Hybrid` (feature off)      | Build error (feature-guarded)                                   |
| `from_env_prefixed("RON_PROTO_")` set | Values override defaults; `validate()` passes/fails accordingly |
| Interop vectors path invalid          | Vector loader returns error; core config remains valid          |
| `addr_prefix` not `"b3:"`             | `validate()` returns error                                      |
| `hex_lowercase = false`               | `validate()` returns error                                      |

---

## 12) Mermaid — Config Use in Hosts

```mermaid
flowchart TB
  H[Host/App/Service] -->|constructs| C[ProtoConfig]
  C -->|validate()| V{OK?}
  V -- yes --> U[Use DTOs: serialize/deserialize]
  V -- no  --> E[Fail fast (return typed error)]
```

---

## 13) Operational Notes

* Treat `ron-proto` as **pure contract**: keep strict defaults on; only loosen during **planned migrations**.
* Don’t mix up **OAP frame limit (1 MiB)** with **storage chunk size (~64 KiB)**. The former is protocol, the latter is I/O shaping.
* If exporting TypeScript bindings (`ts-bindings`), publish them alongside the crate version to keep SDKs in lock-step.
* When a service changes DTOs, update **IDB**, **CONFIG**, **OBSERVABILITY** and **CHANGELOG** together and run schema-compat CI.
* For Micronode amnesia mode, set `zeroize_transient = true` and verify zeroization property tests.

---

## 14) CI Teeth (copy-paste)

```yaml
- name: ron-proto config checks
  run: |
    cargo test -p ron-proto --features schema-guard
    cargo public-api -p ron-proto --deny-breaking
```

---

## 15) References

* **Interop Blueprint — OAP/1** (frame=1 MiB; chunk≈64 KiB)
* **Hardening Blueprint §2** (DoS, size caps)
* **IDB.md** (invariants & gates)
* **OBSERVABILITY.md** (proto version/fingerprint exposure)
* **SECURITY.md** (logging/redaction/PQ traps)
* **SIX_CONCERNS.MD** (SEC/RES/PERF/GOV/ECON/DX mapping)

---

**Definition of Done**

* Programmatic-only config; **no ambient reads**.
* `validate()` enforces spec constants and strictness (including `b3:` and lowercase hex).
* Env overlay is optional and explicit.
* CI checks for schema/public-api/constants are green.
* `/version` (by host) exposes `proto_version` and ABI fingerprint per OBSERVABILITY.

---

