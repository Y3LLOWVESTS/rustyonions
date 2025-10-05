# Combined Markdown

_Source directory_: `crates/ron-auth/docs`  
_Files combined_: 12  
_Recursive_: 0

---

### Table of Contents

- API.MD
- CONCURRENCY.MD
- CONFIG.MD
- GOVERNANCE.MD
- IDB.md
- INTEROP.MD
- OBSERVABILITY.MD
- PERFORMANCE.MD
- QUANTUM.MD
- RUNBOOK.MD
- SECURITY.MD
- TESTS.MD

---

## API.MD
_File 1 of 12_

````markdown
---
title: API Surface & SemVer Reference — ron-auth
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, auditors, API consumers
---

# API.md — ron-auth

## 0. Purpose

This document captures the **public API surface** of `ron-auth`:

- Snapshot of exported functions, types, traits, modules.
- SemVer discipline: what changes break vs. extend.
- Alignment with `CHANGELOG.md` (behavioral vs. surface changes).
- CI-enforceable via `cargo public-api` and `cargo semver-checks`.
- Acts as the **spec** for external consumers (services, SDKs).

> **Crate role:** Pure library for **capability attenuation & verification** (macaroon-style), **offline** decisioning, keyed **BLAKE3** MAC chain.  
> **No I/O**; keys via **opaque handle** trait.

---

## 1. Public API Surface

> Generated with:
>
> ```bash
> cargo public-api -p ron-auth --simplified --deny-changes
> ```
>
> The block below is the **intended** surface for v0.1.x. Treat it as the contract;
> CI must keep an up-to-date snapshot in `docs/api-history/ron-auth/<version>.txt`.

### 1.1 Current Surface (intended v0.1.x)

```text
# Re-exports at crate root
pub use config::{VerifierConfig, VerifierConfigBuilder, ContextDefaults, CaveatPolicy, UnknownCustom, ConfigError};
pub use token::{Capability, CapabilityBuilder, Scope, Caveat, RequestCtx};
pub use traits::{MacKeyProvider, MacHandle};
pub use verify::{verify_token, Decision, DenyReason, AuthError};

#[cfg(feature = "pq-hybrid")]
pub use pq_hybrid::SigAdapter;

# Modules

pub mod config {
    #[non_exhaustive]
    pub struct VerifierConfig { /* fields stable as documented */ }
    pub struct VerifierConfigBuilder { /* builder setters */ }
    pub struct ContextDefaults { pub amnesia: bool, pub policy_digest_hex: Option<String>, pub redaction_digest_prefix_bytes: u8 }
    pub struct CaveatPolicy { pub allow_custom_namespaces: Vec<String>, pub unknown_custom_behavior: UnknownCustom }
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum UnknownCustom { Deny, Ignore }
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum ConfigError {
        MaxTokenBytes(u32),
        MaxCaveats(u16),
        ClockSkew(u32),
        PolicyDigestHex,
        RedactionPrefix(u8),
    }

    #[cfg(feature="config-env")]
    pub mod env_helper {
        pub fn from_env_with_defaults(defaults: Option<super::VerifierConfig>) -> Result<super::VerifierConfig, super::ConfigError>;
    }
}

pub mod token {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Capability { /* stable fields: v, tid, kid, r, c, s */ }
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Scope {
        pub prefix: Option<String>,
        pub methods: Vec<String>,
        pub max_bytes: Option<u64>,
    }
    #[non_exhaustive]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Caveat {
        Exp(u64),
        Nbf(u64),
        Aud(String),
        Method(Vec<String>),
        PathPrefix(String),
        IpCidr(String),
        BytesLe(u64),
        Rate { per_s: u32, burst: u32 },
        Tenant(String),
        Amnesia(bool),
        GovPolicyDigest(String),
        Custom { ns: String, name: String, cbor: serde_cbor::Value },
    }
    pub struct CapabilityBuilder; // attenuation-only (add caveats)
    impl CapabilityBuilder {
        pub fn new(scope: Scope, tid: impl Into<String>, kid: impl Into<String>) -> Self;
        pub fn caveat(self, c: Caveat) -> Self;
        pub fn build(self) -> Capability;
        // Note: root minting is not public; see feature "mint-internal" (doc(hidden))
    }

    pub struct RequestCtx<'a> {
        pub now_unix_s: u64,
        pub method: &'a str,
        pub path: &'a str,
        pub peer_ip: Option<std::net::IpAddr>,
        pub object_addr: Option<&'a str>,
        pub tenant: &'a str,
        pub amnesia: bool,
        pub policy_digest_hex: Option<&'a str>,
        pub extras: serde_cbor::Value,
    }

    // Encoding helpers (pure, no I/O)
    pub fn encode_b64url(token: &Capability) -> String;
    pub fn decode_b64url(b64: &str) -> Result<Capability, crate::verify::AuthError>;
}

pub mod traits {
    pub trait MacKeyProvider {
        fn mac_handle(&self, tenant: &str, kid: &str) -> Result<Box<dyn MacHandle>, crate::verify::AuthError>;
    }
    pub trait MacHandle: Send + Sync {
        fn mac(&self, msg: &[u8]) -> [u8; 32]; // keyed BLAKE3, domain-separated by caller
    }
}

pub mod verify {
    pub fn verify_token(
        cfg: &crate::config::VerifierConfig,
        token_b64: &str,
        ctx: &crate::token::RequestCtx<'_>,
        keyp: &impl crate::traits::MacKeyProvider,
    ) -> Result<Decision, AuthError>;

    #[non_exhaustive]
    pub enum Decision {
        Allow { scope: crate::token::Scope },
        Deny  { reasons: Vec<DenyReason> },
    }

    #[non_exhaustive]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum DenyReason {
        ParseBase64, ParseCbor, ParseBounds, SchemaUnknownField,
        MacMismatch, UnknownKid, TenantMismatch,
        CaveatExp, CaveatNbf, CaveatAud, CaveatMethod,
        CaveatPath, CaveatIp, CaveatBytes, CaveatRate,
        CaveatTenant, CaveatAmnesia, CaveatPolicyDigest,
        CaveatCustomUnknown, CaveatCustomFailed,
    }
    impl DenyReason { pub fn as_str(&self) -> &'static str; }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum AuthError {
        ParseBase64,
        ParseCbor,
        Bounds,
        SchemaUnknownField,
        UnknownKid { tenant: String, kid: String },
        MacMismatch,
        TenantMismatch,
        CaveatFailed(DenyReason),
        // future variants reserved
    }
}

#[cfg(feature="pq-hybrid")]
pub mod pq_hybrid {
    pub trait SigAdapter: Send + Sync {
        fn verify_hybrid(&self, payload: &[u8], sig: &[u8], kid: &str) -> Result<(), crate::verify::AuthError>;
    }
}

pub mod prelude {
    pub use crate::config::*;
    pub use crate::token::*;
    pub use crate::traits::*;
    pub use crate::verify::{verify_token, Decision, DenyReason, AuthError};
}
````

> Notes:
>
> * `#[non_exhaustive]` appears on enums likely to grow (Caveat, DenyReason, AuthError, Decision).
> * The **root-minting** API is **not** public (guarded behind `mint-internal` and `#[doc(hidden)]`).
> * Encoding helpers are stable and deterministic (IDB I-5/I-18).

---

## 2. SemVer Discipline

### Additive (Minor / Non-Breaking)

* Add new **caveat variants** to `Caveat` (enum is `#[non_exhaustive]`).
* Add new **deny reasons** to `DenyReason` (also non-exhaustive).
* Add new **error variants** to `AuthError` (non-exhaustive).
* Add new **helpers** (e.g., additional pure utilities) or **new modules** behind **opt-in features**.
* Add new **builder methods** that do not change existing defaults/semantics.

### Breaking (Major)

* Remove/rename any public item re-exported at root or change signatures/trait bounds.
* Change **wire encoding** invariants: canonical CBOR rules, Base64URL padding, MAC chain domain strings.
* Make previously `#[non_exhaustive]` enums **exhaustive**, or change existing variant shapes.
* Change **default** verification semantics (e.g., switch unknown custom caveats from Deny→Ignore).
* Expose any I/O or global mutable state through the public API.

### Patch-Level

* Doc-only changes.
* Performance improvements that **do not** alter behavior or surface.
* Error messages text changes (while preserving variants).
* Adding `#[inline]`, `#[must_use]`, or equivalent attributes.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0`. Raising MSRV is a **minor** bump (documented in `CHANGELOG.md`).
* **No unsafe:** `#![forbid(unsafe_code)]` — adding unsafe would be a **major** unless strongly justified and well-scoped.
* **Pure library:** No I/O, no global singletons. Introducing I/O would be **major**.
* **Deterministic wire:** The CBOR/Base64URL encoding is **stable** within a major line; any change is **major**.
* **Constant-time MAC compare** is guaranteed; removing constant-time behavior is **major**.
* **Deny reason strings** returned by `DenyReason::as_str()` are part of the **observability contract**; renames are **major** (additive reasons are minor).

---

## 4. Invariants → API Shape

* **Attenuation-only builder:** `CapabilityBuilder` cannot remove/relax prior caveats (compile-time API shape enforces monotonic add).
* **Offline verification:** `verify_token` requires the caller to provide a `(tenant, kid) → MacHandle` resolver; no hidden network calls.
* **Fail-closed:** Unknown KID, unknown custom caveats (by default), parse ambiguity → typed **errors** (not bools), to force explicit handling upstream.
* **No secrets in Debug/Display:** Any type that could carry secrets either does not implement those traits or redacts by design.

---

## 5. Tooling

* **cargo public-api**: CI gate (`--deny-changes`) with snapshot stored at `docs/api-history/ron-auth/<version>.txt`.
* **cargo semver-checks**: warns on accidental breaking changes vs SemVer expectations.
* **cargo doc** + doctests: examples compile and run in CI; all public items documented (`#![deny(missing_docs)]` recommended).

---

## 6. CI & Gates

* **PR pipeline must run:**

  * `cargo public-api -p ron-auth --simplified --deny-changes`
  * `cargo semver-checks -p ron-auth` (advisory but recommended blocking)
* **Failure policy:**

  * If a surface diff exists, PR must include:

    * `docs/api-history/ron-auth/<new-version>.txt` updated
    * `CHANGELOG.md` entry with **Added/Changed/Removed** and SemVer rationale
* **Feature matrix in CI:**

  * Build and doc with `--no-default-features`, `--features verify`, `--features pq-hybrid`, `--features config-env`
  * Ensure `mint-internal` is **OFF** outside `svc-passport` and tests

---

## 7. Acceptance Checklist (DoD)

* [ ] Current public API snapshot generated & checked in.
* [ ] SemVer assessment performed (minor vs. major vs. patch).
* [ ] CI gates green (`public-api`, `semver-checks`, docs).
* [ ] CHANGELOG updated for any surface or behavioral change.
* [ ] `DenyReason::as_str()` map updated + tests adjusted (observability contract).
* [ ] Feature-gated APIs documented (`pq-hybrid`, `config-env`).
* [ ] No I/O or globals introduced.

---

## 8. Appendix

### 8.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 8.2 Behavioral compatibility (non-surface)

* **Timing:** constant-time MAC compare; timing of **parse** can vary with input size but is bounded (I-12).
* **Errors:** human-readable `Display` messages may evolve; **variants** remain stable.
* **Performance SLO:** p95 ≤ `60 + 8×n_caveats` μs (documented in `OBSERVABILITY.md`); regressions without surface change still require CHANGELOG note.

### 8.3 History (to start once 0.1.0 releases)

* v0.1.0 — Initial public surface with `verify_token`, `CapabilityBuilder`, config types, trait-based key provider.

```
```


---

## CONCURRENCY.MD
_File 2 of 12_

````markdown
---
title: Concurrency Model — ron-auth
crate: ron-auth
owner: Stevan White
last-reviewed: 2025-10-04
status: draft
template_version: 1.1
msrv: 1.80.0
tokio: "1.x (pinned at workspace root)"
loom: "0.7+ (dev-only)"
lite_mode: "This is a small pure library. We fully fill §§1,3,4,5,10,11 and mark others N/A."
---

# Concurrency Model — ron-auth

`ron-auth` is a **pure verification library** (no network/disk I/O, no background tasks).
It must be **thread-safe**, **reentrant**, **side-effect-free**, and **deterministic** under
contention. This file makes all concurrency rules explicit and testable.

> **Golden rule:** never hold a lock across `.await`.  
> (This crate is predominantly **sync** and should not `.await` at all.)

---

## 0) Lite Mode (applies)

- We complete **§1 Invariants**, **§3 Channels**, **§4 Locks**, **§5 Timeouts**, **§10 Validation**, **§11 Code Patterns**.
- Sections **§2, §6, §7, §8, §9, §12–§17** are **N/A** to this crate (library has no runtime, tasks, I/O).

---

## 1) Invariants (MUST)

- **[L-1] No `await` in core paths.** All verification/attenuation APIs are synchronous and do not park a runtime.
- **[L-2] Send+Sync everywhere appropriate.** Public verifier types and configs are `Send + Sync`; share via `Arc`.
- **[L-3] No global mutable state.** No `static mut`, no runtime-mutable singletons. If a global registry exists, it is **write-once** before use and read-only thereafter.
- **[L-4] Constant-time compares.** All MAC equality checks use `subtle` and are independent of success/failure (no timing side channels).
- **[L-5] Zeroization on drop.** Any buffer or key material exposed through trait objects is zeroized; no secret-bearing `Debug`/`Display`.
- **[L-6] Lock discipline.** If any lock exists, its critical section is **short** and **never** crosses an `.await`. Nested locks require a documented order (none expected).
- **[L-7] Config snapshots.** Verification uses an **immutable snapshot** of `VerifierConfig` (e.g., `Arc<VerifierConfig>`). Hot-swaps (if a host performs them) are atomic and race-free.
- **[L-8] No task leaks.** The crate does not spawn tasks. If a host supplies async hooks (e.g., key provider), they must own lifecycle.
- **[L-9] Bounded work.** Decoding/evaluation is **O(n_caveats)** with configured caps (`max_token_bytes`, `max_caveats`); no unbounded structures.
- **[L-10] Panic-free.** Concurrency violations, decode errors, and policy failures return typed errors—never `panic!`.
- **[L-11] Reentrancy.** `verify()` may be called concurrently from many threads; results are independent except for externally injected context (time, amnesia flag, policy digest).
- **[L-12] Registrar freeze.** Custom caveat handlers (if used) are **registered before first use** and then considered frozen (no concurrent mutation).

---

## 2) Runtime Topology — **N/A (library)**

No background tasks, listeners, or supervisors are created by this crate.

---

## 3) Channels & Backpressure

**Library inventory:** _none_. `ron-auth` does **not** own channels.

**Host guidance (when integrating):**
- If you propagate verification results via channels, use **bounded** `mpsc` and prefer **rejecting** new work (`try_send` → `Busy`) over buffering.
- For broadcast (e.g., policy/handler updates), prefer **watch** with **write-once/freeze** semantics for this crate (see §11/Registrar).

---

## 4) Locks & Shared State

**Policy:** Prefer **lock-free** reading via `Arc` snapshots and immutable data.

**Allowed**
- `Arc<VerifierConfig>` snapshots (recommended).
- `once_cell::sync::Lazy` or `OnceLock<T>` for **write-once** global registries (see “Registrar” below).
- Tiny, short-lived `Mutex`/`RwLock` only for construction/registration **before** the verifier is used.

**Forbidden**
- Holding any lock across `.await` (should not occur in this crate).
- Mutable global maps that change at runtime after first verification call.
- Secret-bearing data behind `Debug`, `Display`, or `Clone`.

**Recommended patterns**
- **Config hot-swap (host side):** keep `ArcSwap<VerifierConfig>` (or atomically replace an `Arc<VerifierConfig>`) and pass the snapshot into `verify()`; the crate remains oblivious to swaps.
- **Registrar for custom caveats:** provide a **builder** that accepts handler registrations; once `build()` is called, **freeze** the handler map into an `Arc<HashMap<…>>` and do not mutate.

---

## 5) Timeouts, Retries, Deadlines

- **No I/O → no runtime timeouts** inside this crate.
- Time appears only via `RequestCtx::now_unix_s` and **bounded skew** logic. There is **no sleeping**, no retries, no timers.
- If the host calls key providers that potentially block, they must **perform those calls** before calling `verify()` (or adapt with their own timeouts).

---

## 6) Cancellation & Shutdown — **N/A (library)**

No tasks to cancel. Hosts should cancel their own async operations before calling into `ron-auth`.

---

## 7) I/O & Framing — **N/A (library)**

The crate only decodes **in-memory** Base64URL → CBOR and evaluates caveats.

---

## 8) Error Taxonomy (Concurrency-Relevant) — **N/A (library-local queueing)**

All errors are **pure** (decode/semantic/config) and not related to runtime contention.

---

## 9) Metrics (Concurrency Health) — **N/A (library emits none)**

Any metrics are emitted by hosts. Library offers **hooks** to return reason codes so hosts can increment counters.

---

## 10) Validation Strategy

**Unit / Property**
- **Determinism under threads:** run `verify()` from multiple threads with identical inputs; assert **bit-stable** results.
- **Bounds respected:** property tests randomly generate tokens up to bounds; oversize tokens are rejected without allocation explosions.
- **Registrar freeze:** once built, attempts to mutate handler tables must fail at type level (no API) or return a typed error.

**Loom (dev-only)**
- **Model:** two threads calling `verify()` while a third attempts to replace an `Arc<VerifierConfig>` snapshot (host-style). Assert: no data races, decisions use either **old or new** snapshot but never a torn state.
- **No deadlocks:** if a minimal lock is used for *build time only*, loom should never find a lock cycle during verification.

**Fuzz**
- **CBOR fuzzing:** malformed/hostile inputs must not hang, OOM, or panic; total work ≤ O(n).
- **Caveat sequences:** reordering or duplication must not bypass conjunctive semantics.

**Chaos**
- Simulate host hot-swap of config while running verifications at high concurrency; measure no increase in error rate beyond the expected boundary (tokens incompatible with new policy).

---

## 11) Code Patterns (Copy-Paste)

### 11.1 Config snapshots (lock-free read)
```rust
use std::sync::Arc;
use ron_auth::{verify_token, RequestCtx};
use ron_auth::config::VerifierConfig;

let cfg: Arc<VerifierConfig> = Arc::new(VerifierConfig::default());
// in request handlers:
let snapshot = cfg.clone(); // cheap; lock-free
let decision = verify_token(&snapshot, token_b64, &ctx)?;
````

### 11.2 Registrar (write-once, then read-only)

```rust
use std::{collections::HashMap, sync::Arc};
type Handler = Arc<dyn Fn(&serde_cbor::Value, &RequestCtx) -> Result<(), AuthError> + Send + Sync>;

pub struct Registrar {
    table: HashMap<(String, String), Handler>,
}

impl Registrar {
    pub fn new() -> Self { Self { table: HashMap::new() } }
    pub fn register(mut self, ns: &str, name: &str, h: Handler) -> Self {
        self.table.insert((ns.to_owned(), name.to_owned()), h);
        self
    }
    pub fn freeze(self) -> Arc<HashMap<(String,String), Handler>> {
        Arc::new(self.table) // After this, do not mutate; share Arc clones freely.
    }
}
```

### 11.3 No lock across `.await` (defensive pattern for hosts)

```rust
// Acquire data under a lock, drop guard, then await.
let value = { let g = state.lock().unwrap(); g.derive_value() }; // guard dropped here
do_async(value).await;
```

### 11.4 Constant-time equality

```rust
use subtle::ConstantTimeEq;
fn eq_mac(a: &[u8;32], b: &[u8;32]) -> bool {
    a.ct_eq(b).into() // time independent
}
```

### 11.5 ArcSwap (optional host hot-swap)

```rust
// in host crate, not inside ron-auth:
use arc_swap::ArcSwap;
let cfg = ArcSwap::from_pointee(VerifierConfig::default());
// reader:
let snapshot = cfg.load(); // lock-free snapshot
// writer (reload):
cfg.store(Arc::new(new_cfg));
```

---

## 12) Configuration Hooks — **N/A (library concurrency)**

See `docs/CONFIG.md` for schema; concurrency-relevant knobs are host-side (e.g., queue sizes).

---

## 13) Known Trade-offs / Nonstrict Areas

* **Registrar mutability:** We intentionally select a **freeze-then-share** model instead of supporting dynamic handler churn; this removes a whole class of data races.
* **No internal caches:** We avoid caches to keep semantics deterministic and code path short (hosts may add caches outside with their own concurrency controls).

---

## 14) Mermaid Diagrams — **N/A (no tasks/queues)**

Optional diagrams are provided in IDB/CONFIG. None are required here due to lack of runtime.

---

## 15) CI & Lints (Enforcement)

* **Clippy:** `-D warnings`, `-W clippy::await_holding_lock` (defensive), `-W clippy::mutex_atomic`.
* **Forbid:** `#![forbid(unsafe_code)]`.
* **cargo-deny / geiger:** ensure no accidental async executors or I/O deps creep in.
* **Tests:** gate a `loom` job (ignored by default) to model Arc snapshot behavior (dev-only).

---

## 16) Schema Generation — **N/A**

No channels/locks registry needed; the library should remain lock-free at runtime.

---

## 17) Review & Maintenance

* Re-review on any change that introduces interior mutability, global state, or async.
* Keep this file in PR scope whenever `verify()` signatures or config snapshotting changes.

```
```


---

## CONFIG.MD
_File 3 of 12_



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


---

## GOVERNANCE.MD
_File 4 of 12_


````markdown
# 🏛 GOVERNANCE.md — `ron-auth`

---
title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, ops, auditors, stakeholders
crate-type: policy
---

## 0. Purpose

`ron-auth` defines the **verification policy** for capability tokens (macaroon-style), including stable **deny reason taxonomy**, **caveat semantics**, and **key-lookup contracts**.  
This document sets the **rules of engagement** for change control, auditability, and bounded authority so that upgrades cannot silently weaken authentication or break interop.

It ensures:
- Transparent, auditable decision-making for policy changes (reason strings, caveat registry).
- Enforcement of **auth-policy invariants** (deny-by-default for unknowns, bounded inputs).
- Clear **authority boundaries** between verifier, issuers (svc-passport), and custodians (ron-kms).
- SLA-backed commitments to external consumers (stable surface across v1.x).

Ties into:
- **Economic Integrity Blueprint** (no doubles, bounded issuance) — adapted here as **no “free” authority escalation** and **no unbounded attenuation bypass**.
- **Hardening Blueprint** (bounded authority, key custody, size caps).
- **Perfection Gates A–O** (esp. **Gate I**: bounded invariants; **Gate M**: appeal paths; **Gate F/L**: perf + scaling).

---

## 1. Invariants (MUST)

Non-negotiable rules for this crate’s policy surface:

- **[I-A1] Stable Reason Strings:**  
  All deny reasons are stable **string constants** (e.g., `parse.b64`, `parse.cbor`, `parse.bounds`, `schema.unknown_field`, `mac.mismatch`, `kid.unknown`, `tenant.mismatch`, `caveat.*`, `caveat.custom.*`).  
  • Adding new reasons = **minor**; renaming existing = **major** (v2).  
  • Tests MUST assert exact reason strings on golden vectors.

- **[I-A2] Deny by Default:**  
  Unknown caveat namespaces, unknown schema fields (unless explicitly permitted), invalid MACs, or out-of-bounds sizes MUST result in **Deny** (never “best effort”).

- **[I-A3] Bounded Inputs:**  
  `max_token_bytes` default 4096 (validated range [512, 16384]); `max_caveats` default 64 (validated range [1, 1024]). Tokens beyond caps MUST be denied with `parse.bounds`.

- **[I-A4] Key Custody Separation:**  
  `ron-auth` never holds raw secret keys. It receives **opaque handles** via `MacKeyProvider`. All key generation, storage, and rotation belong to **ron-kms**; minting belongs to **svc-passport**.

- **[I-A5] Rotation Window Safety:**  
  Verifier MUST accept **current + N previous** KIDs (window determined by ops). Dropping previous KID from the window MUST deterministically yield `kid.unknown`.

- **[I-A6] No Hidden Overrides:**  
  No “debug” or environment toggle may suppress **deny-by-default** or enlarge authority without explicit config + audit log in host.

- **[I-A7] Auditability:**  
  All policy-relevant outcomes are observable via **golden metrics and stable reasons** exported by hosts; changes to taxonomy/semantics require entry in CHANGELOG + runbook notes.

---

## 2. Roles & Authority

### Roles
- **Verifier Owner (this crate):** maintains reason taxonomy, caveat semantics, and config schema; ships golden vectors.  
- **Issuer (`svc-passport`):** mints capabilities; enforces mint-time constraints; publishes rotation and policy digest.  
- **Custodian (`ron-kms`):** stores/rotates keys; enforces rotation window guarantees.  
- **Host Owners (e.g., `svc-gateway`):** embed `ron-auth`, export metrics/logs, enforce size caps early, wire amnesia policy.  
- **Auditor:** independent read-only; validates changelogs, vectors, and dashboards.

### Authority Boundaries
- `ron-auth` may **define** taxonomy & semantics, but **cannot mint** tokens or control keys.  
- `svc-passport` may define mint policies but **cannot override** verifier deny-by-default rules.  
- `ron-kms` may rotate keys but **cannot alter** decision semantics.  
- Host wrappers may **label/observe** decisions but **cannot change** reason strings or bypass deny-by-default.

---

## 3. Rules & SLAs

- **Semantic Stability SLA (v1.x):**  
  - No breaking changes to reason strings, CBOR field names, or MAC domain strings.  
  - Deprecations require: announcement + 90-day transition + dual-accept w/ warnings.

- **Verification Observability SLA:**  
  - For every `verify()` call, host MUST record: `verify_total`, `deny_total{reason}`, `parse_error_total{kind}`, `unknown_kid_total`, `duration_us`, `token_size_bytes`.  
  - Audit dashboards MUST render deny taxonomy distribution and rotation health.

- **Rotation Health SLA:**  
  - `unknown_kid_total` steady-state ≈ 0.  
  - Any sustained increase (>0 for 10m) triggers rotation drift alert & runbook.

- **Appeals/Overrides SLA:**  
  - Disputed denies are marked via host “dispute” pathway (no silent rollback).  
  - Overrides (e.g., temporary allow for a namespace) require a **multi-sig governance** change with expiry and audit entry.

---

## 4. Governance Process

### Proposal Lifecycle (policy-affecting changes)
1. **Draft** (issue with rationale, examples, vectors).  
2. **Review** (CODEOWNERS: verifier + security + host owners + issuer).  
3. **Approve** (N-of-M maintainers; quorum defined below).  
4. **Execute** (merge; publish new vectors; update RUNBOOK & OBSERVABILITY dashboards).

**Quorum / Signers:**  
- Minimum quorum **3 of 5**: {Verifier maintainer, Security lead, Host rep, KMS rep, Issuer rep}.  
- **Default reject** if quorum not reached in **72h**.

### Emergency Powers (bounded)
- **Freeze Custom Namespaces:** temporary deny for `caveat.custom.*` not on allowlist if abused at scale.  
- **Rotation Window Extend:** temporarily accept **N+1** previous KID to avoid accidental outages.  
- Both require **majority multi-sig**, expiry ≤ 7 days, and audit entry within **24h**.

### Parameter Changes
- Bounds (`max_token_bytes`, `max_caveats`), namespace allowlist, amnesia semantics, or PQ feature toggles must follow the proposal lifecycle (no “drive-by” merges).

---

## 5. Audit & Observability

- **Audit Log (host-exported):**  
  - JSON lines with `event=auth.verify`, `result`, `reason`, `tenant`, `kid`, `corr_id`, `token_digest8` (redacted).  
  - No secrets; no raw tokens; retention matches org policy.

- **Metrics (host-exported; stable series):**  
  - `ron_auth_verify_total`  
  - `ron_auth_deny_total{reason}`  
  - `ron_auth_parse_error_total{kind}`  
  - `ron_auth_unknown_kid_total`  
  - `ron_auth_duration_us_bucket`  
  - `ron_auth_token_size_bytes`

- **Verifiability:**  
  - Golden vectors (allow/deny with exact reasons) published per release; CI enforces parity.  
  - Rotation drills: alert on `rate(unknown_kid_total)` > 0 for 10m; attach RUNBOOK link.

- **Red-team Drills:**  
  - Simulate rogue admin attempting to introduce permissive custom caveat; ensure deny-by-default holds and governance prevents merge without quorum.

---

## 6. Config & Custody

- **Config (host-provided to verifier):**  
  - Bounds: `max_token_bytes`, `max_caveats` (validated).  
  - Registrar: custom caveat allowlist (minimal by default).  
  - Amnesia binding: whether host operates in **amnesia mode** (RAM-only/no persistent logs).  
  - PQ toggles: `pq-hybrid` envelope verification (optional).

- **Custody (keys):**  
  - Private keys live in **ron-kms**/HSM; verifier receives opaque `MacHandle`.  
  - No raw keys in env/files.  
  - **Rotation Policy:** at least **every 90 days** or immediately after suspected compromise; window accepts current + N previous until fleet convergence.

---

## 7. Appeal Path

- **When a deny is disputed:**  
  1) Host attaches `disputed=true` metadata (no rollback).  
  2) Open governance issue with token digest/time, reason, and minimal reproduction.  
  3) If change is warranted, file a **proposal** (Section 4) to adjust registrar or policy.

- **Escalation:**  
  - Step 1: governance bus topic or Git issue labelled `auth-policy`.  
  - Step 2: multi-sig override proposal with expiry.  
  - Step 3: auditor review & public disclosure note in release.

---

## 8. Acceptance Checklist (DoD)

- [ ] Invariants (I-A1…A7) enforced by tests (unit/integration/property).  
- [ ] Reason taxonomy & caveat semantics documented; **no rename** without major bump.  
- [ ] Proposal lifecycle + CODEOWNERS/quorum in place.  
- [ ] Metrics & audit formats stable; dashboards wired.  
- [ ] Rotation drills + alert thresholds defined and tested.  
- [ ] Appeal path validated via a dry-run (dispute → proposal → resolution).  

---

## 9. Appendix

### 9.1 Blueprints
- **Economic Integrity:** no silent authority inflation; no “free” attenuation.  
- **Hardening:** bounded authority; key custody separation; size caps.  
- **Perfection Gates:**  
  - **Gate I** — bounded invariants (deny-by-default, stable reasons).  
  - **Gate M** — appeal paths operational.  
  - **Gate F/L** — perf/scaling guardrails intact (cross-ref PERF/RUNBOOK).

### 9.2 Mermaid — Policy Change Flow
```mermaid
flowchart LR
  D[Draft] --> R[Review]
  R -->|Quorum met (N-of-M)| A[Approve]
  R -->|No quorum in 72h| X[Reject]
  A --> E[Execute: merge + vectors + runbook]
  E --> O[Observe: dashboards + alerts]
  style A fill:#166534,stroke:#052e16,color:#fff
  style X fill:#7f1d1d,stroke:#450a0a,color:#fff
````

### 9.3 References

* `API.md` — traits (`MacKeyProvider`, `SigAdapter`), `Decision`, deny reasons.
* `CONFIG.md` — bounds, registrar, amnesia binding, env mapping.
* `OBSERVABILITY.md` — metrics taxonomy, dashboards, alerts.
* `SECURITY.md` — zeroization/log redaction; trust boundaries.
* `IDB.md` — invariants & canonical vectors; interop guarantees.

### 9.4 History

* Keep a short ledger of notable governance events (reason additions, namespace freezes, rotation incidents, emergency powers usage).

```



---

## IDB.md
_File 5 of 12_



````markdown
---
title: ron-auth — Invariant-Driven Blueprint (IDB)
version: 0.1.0
status: draft
last-updated: 2025-10-04
audience: contributors, ops, auditors
crate: ron-auth
crate-type: lib
concerns: [SEC, GOV]
owners: [Stevan White]
msrv: 1.80.0
---

# ron-auth — IDB

> **Role:** Pure library for **capabilities** (macaroon-style attenuation) and **offline verification**.  
> **Issuance & rotation live in:** `svc-passport`.  
> **Key custody lives in:** `ron-kms`.  
> **Typical consumers:** `svc-gateway`, `svc-overlay`, `svc-index`, `svc-storage`, `svc-mailbox`, etc.

---

## 1) Invariants (MUST)

- **[I-1: Library-only purity]** `ron-auth` performs **no network or disk I/O**. All inputs (time, keys, request context) are **injected** via traits. Deterministic, side-effect-free verification.
- **[I-2: Capabilities-only]** Authorization is expressed as **attenuable capabilities**, not roles or ambient ACLs. Every allow is justified by at least one satisfied caveat.
- **[I-3: Offline verification]** Tokens must verify **without** calling a central introspection service. All material required to decide is present in the token + request context + injected keys.
- **[I-4: MAC construction]** Capability integrity uses **BLAKE3 (keyed mode)** as the MAC primitive. No SHA-2/MD5 anywhere.
- **[I-5: Canonical encoding]** The token envelope is **CBOR (deterministic)** and exported as **Base64URL (no padding)**. Parsing is strict; ambiguous encodings are rejected.
- **[I-6: Key boundaries]** Secret keys never leave the `ron-kms` boundary. `ron-auth` only receives **opaque key handles** (get-by-tenant + KID) and uses them to compute/verify MACs. **Zeroize** any transient material.
- **[I-7: Rotation & KID]** Every token binds a **KID** and **tenant**. Verifiers MUST support **multi-version rotation** (active + N previous), and MUST fail-closed if KID is unknown.
- **[I-8: Attenuation order]** The signature chain is **order-sensitive** and built as `sig_{i+1} = BLAKE3(key, sig_i || encode(caveat_{i+1}))` with domain separation at init. Reordering caveats invalidates the token.
- **[I-9: Conjunction semantics]** Caveats are **ANDed** (conjunctive). Missing or unsatisfied caveats **deny**. No implicit OR.
- **[I-10: Time handling]** Time caveats (`exp`, `nbf`) are evaluated against an **injected clock** with a bounded skew window (default ±300s). No direct use of system time.
- **[I-11: Constant-time decisions]** MAC verification and sensitive compares are **constant-time**; no early-exit timing leaks on token validity.
- **[I-12: Bounded size/complexity]** A token is ≤ **4096 bytes** and ≤ **64 caveats**. Verification is **O(caveats)** with a per-caveat upper bound. Oversized/over-complex tokens are rejected.
- **[I-13: Multi-tenant safety]** Keys and decisions are **namespaced by tenant**. Cross-tenant acceptance is impossible without explicit cross-trust configuration.
- **[I-14: Least privilege]** Default-deny. Capabilities **must** encode explicit resource scope (e.g., object prefix, method set, max bytes, rate).
- **[I-15: No secret leakage]** Tokens and secrets are **never logged**. Observability uses **redacted** digests and counters only.
- **[I-16: Error taxonomy]** No `panic!`, `unwrap`, or secret-bearing errors. All errors map to a stable, non-leaky **AuthError** taxonomy.
- **[I-17: PQ readiness]** The MAC scheme is PQ-agnostic, but **public-key adjuncts** (for cross-org delegation) must support **hybrid** verification (e.g., Ed25519+Dilithium2) behind a feature-gated adapter. MAC-only flows remain default.
- **[I-18: Determinism & reproducibility]** Given the same inputs, token mint and verification are **bit-stable**. Canonical test vectors are part of the repo.
- **[I-19: API stability]** Public API follows **SemVer**. Breaking changes require deprecation spans and updated vectors.
- **[I-20: Safety]** `#![forbid(unsafe_code)]`; secret-bearing types implement `Drop` with **zeroization** and **do not** implement `Debug`/`Display` for raw bytes.
- **[I-21: Amnesia binding]** If a token carries `Amnesia(true)`, verification MUST deny unless the host asserts `amnesia_mode = ON` (RAM-only caches, no persistent logs, aggressive zeroization). Services MUST propagate their amnesia state into `RequestCtx`.
- **[I-22: Governance policy binding]** The library MUST support an optional caveat that binds decisions to a **governance policy digest** (e.g., `GovPolicyDigest(b3::<hex>)`). Verification MUST deny if the current policy digest (injected by the caller) does not match. `ron-auth` does not interpret policy content—only digest equality.
- **[I-23: Mint isolation]** Any minting functionality MUST be hidden behind a `mint-internal` feature, `#[doc(hidden)]`, and excluded from default features. Default builds MUST NOT export mint APIs.

---

## 2) Design Principles (SHOULD)

- **[P-1: Attenuation-first UX]** Make it trivial to **narrow** a capability (fluent builder) and non-ergonomic to broaden it.
- **[P-2: Stateless verifiers]** Avoid caches for correctness; if optional caches are used, they must be **opt-in**, bounded, and never hold raw secrets.
- **[P-3: Extensible caveats]** Provide a **registry** of first-class caveats (time/aud/method/path/ipnet/bytes/rate/tenant/amnesia/policy-digest), plus a **namespaced custom** caveat escape hatch.
- **[P-4: Context minimalism]** Keep `RequestCtx` small and composable: `(now, method, path, peer_ip, object_addr, tenant, amnesia, policy_digest, extras)`.
- **[P-5: Fail-closed]** Any parse error, unknown caveat (without a registered handler), unknown KID, or clock failure ⇒ **deny**.
- **[P-6: Observability hooks]** Expose counters/histograms (success, denied_by_caveat, parse_error, unknown_kid, duration_us) but never token contents.
- **[P-7: Test vectors as contract]** Every canonical caveat has **golden vectors** and property tests; SDKs can validate interop without running the full stack.
- **[P-8: Minimal deps]** Prefer `blake3`, `zeroize`, `subtle`, `serde` + `serde_cbor`, `base64` (URL-safe). Avoid heavy crypto stacks in the lib.
- **[P-9: DX consistency]** Builder/Verifier APIs mirror across crates. Errors, metrics, and naming align with the Six Concerns canon.
- **[P-10: PQ bridge path]** For cross-org delegation where sharing MAC keys is undesirable, support a **signature envelope** verified by a pluggable adapter (hybrid Ed25519+Dilithium2); issuance stays in `svc-passport`.

---

## 3) Implementation (HOW)

### [C-1] Core data model (compact sketch)

```rust
/// Deterministic CBOR-serializable capability token
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Capability {
    v: u8,                // version = 1
    tid: String,          // tenant id
    kid: String,          // key id (rotation support)
    r: Scope,             // root scope (resource prefix, methods, limits)
    c: Vec<Caveat>,       // ordered caveats (conjunctive)
    s: [u8; 32],          // MAC = keyed BLAKE3 (final chain mac)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Scope {
    prefix: Option<String>,      // object addr/prefix
    methods: Vec<String>,        // e.g., ["GET","PUT"]
    max_bytes: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum Caveat {
    Exp(u64),                 // unix seconds
    Nbf(u64),                 // not before
    Aud(String),              // audience/service name
    Method(Vec<String>),      // subset of verbs/ops
    PathPrefix(String),       // e.g., /o/b3:abcd...
    IpCidr(String),           // CIDR-notation allow-list
    BytesLe(u64),             // max bytes per request
    Rate { per_s: u32, burst: u32 },
    Tenant(String),           // must equal tid
    Amnesia(bool),            // require amnesia mode host
    GovPolicyDigest(String),  // hex of BLAKE3 digest
    Custom { ns: String, name: String, cbor: serde_cbor::Value },
}

pub struct RequestCtx<'a> {
    pub now_unix_s: u64,
    pub method: &'a str,
    pub path: &'a str,
    pub peer_ip: Option<std::net::IpAddr>,
    pub object_addr: Option<&'a str>, // b3:<hex> or similar
    pub tenant: &'a str,
    pub amnesia: bool,
    pub policy_digest_hex: Option<&'a str>, // for I-22
    pub extras: serde_cbor::Value,          // optional context
}

pub trait MacKeyProvider {
    fn mac_handle(&self, tenant: &str, kid: &str) -> Result<Box<dyn MacHandle>, AuthError>;
}

pub trait MacHandle {
    /// Domain-separated keyed BLAKE3; input is canonical bytes.
    fn mac(&self, msg: &[u8]) -> [u8; 32];
    // keys must be zeroized on drop
}
````

### [C-2] MAC chain & domain separation

* `sig_0 = BLAKE3_key("ron-auth/v1\0init" || tid || kid || canonical(r) || nonce_128)`
* For each caveat `c_i`: `sig_{i+1} = BLAKE3_key("ron-auth/v1\0caveat" || sig_i || canonical(c_i))`
* Final `s = sig_n`. Verification recomputes the chain and compares in constant time.

### [C-3] Encoding & wire shape

* Token bytes = deterministic **CBOR** of `Capability`, then **Base64URL** (no padding).
* **Deterministic CBOR** (canonical ordering, shortest ints, definite lengths) is mandatory.

### [C-4] Verification pipeline (stateless)

1. **Decode** Base64URL → CBOR → `Capability`. Reject if size > 4096 B or caveats > 64.
2. **Lookup key** via `(tid, kid)` using `MacKeyProvider`. Unknown KID ⇒ **deny**.
3. **Recompute chain** over `r` and each `c_i`. Constant-time compare with `s`.
4. **Evaluate caveats** against `RequestCtx`:

   * `Exp/Nbf`: check with injected clock ± skew.
   * `Aud/Tenant`: exact match.
   * `Method/PathPrefix/IpCidr/BytesLe/Rate`: enforce as defined.
   * `Amnesia(true)`: require `ctx.amnesia == true`.
   * `GovPolicyDigest(d)`: require `ctx.policy_digest_hex == Some(d)`.
   * `Custom`: dispatch to registered handler by `(ns, name)`. Unknown ⇒ **deny** unless a namespace explicitly marks “ignore-unknown” (discouraged).
5. **Decision**: `Decision::Allow{ scope }` or `Decision::Deny{ reasons }`.

#### [C-4.1] Verification flow (Mermaid)

```mermaid
sequenceDiagram
  autonumber
  participant S as Service (gateway/overlay/…)
  participant A as ron-auth::verify()
  participant K as MacKeyProvider
  Note over S: Build RequestCtx (now, method, path, peer_ip, tenant, amnesia, policy_digest)
  S->>A: verify(base64url_token, RequestCtx)
  A->>A: Decode (Base64URL→CBOR); bounds check
  A->>K: mac_handle(tenant, kid)
  K-->>A: MacHandle (opaque)
  A->>A: Recompute MAC chain (domain-separated; constant-time compare)
  A->>A: Evaluate caveats (AND): exp/nbf, aud, method/path/ip, bytes/rate, tenant
  A->>A: Check Amnesia(true) ⇒ require ctx.amnesia=ON
  A->>A: Check GovPolicyDigest(d) ⇒ require ctx.policy_digest==d
  A-->>S: Decision::Allow{scope}/Deny{reasons}
  Note over S: Emit redacted metrics (no secrets)
```

### [C-5] Attenuation & minting boundary

* **Attenuate** by adding caveats (builder); **never** remove a prior caveat.
* **Minting root capabilities is performed only by `svc-passport`.**
  The library exposes the algorithm and vectors; any mint API is hidden behind **`mint-internal`** (doc(hidden), non-default) for `svc-passport` and tests.

### [C-6] PQ adapter (optional)

```rust
pub trait SigAdapter {
    fn verify_hybrid(&self, payload: &[u8], sig: &[u8], kid: &str) -> Result<(), AuthError>;
}
```

* Feature `pq-hybrid` adds this adapter for cross-org signature envelopes (e.g., Ed25519+Dilithium2). Default build excludes it.

### [C-7] Observability (no secrets)

* Prometheus counters: `auth_verify_total{result,reason}`, histogram `auth_verify_duration_us`.
* Redaction: only `b3(token_bytes)` first 8 bytes for correlation; never log token fields.

### [C-8] Bounds & defaults (config struct)

* `max_token_bytes = 4096`, `max_caveats = 64`, `clock_skew = ±300s`, `default_ttl = 900s`.
* Tunables are constructor parameters; **no global statics**.

### [C-9] Feature flags

* Default: `features = ["verify"]` — verification + attenuation builders only.
* Optional: `pq-hybrid` — adds `SigAdapter` for hybrid envelopes.
* Internal: `mint-internal` (**doc(hidden)**) — used solely by `svc-passport` and tests to construct root capabilities. Must be **off** in production builds.

---

## 4) Acceptance Gates (PROOF)

> Every MUST maps to one or more gates. **Fail any gate ⇒ no release.**

* **[G-1: Purity & no I/O]** Static scan/deny list ensures no `std::fs`/`reqwest`/`tokio` in the lib.
  *Proves:* **I-1**.
* **[G-2: No unsafe, no panic]** `#![forbid(unsafe_code)]`, clippy `-D panic`, cargo-geiger = 0, `RUSTFLAGS="-D unsafe_code"`.
  *Proves:* **I-16**, **I-20**.
* **[G-3: Canonical encoding]** Golden vectors round-trip; property tests ensure deterministic CBOR; ambiguous encodings rejected.
  *Proves:* **I-5**, **I-18**.
* **[G-4: MAC correctness]** Known-answer tests for chain derivation; constant-time equality via `subtle`.
  *Proves:* **I-4**, **I-11**.
* **[G-5: Rotation]** Accept on current/previous KIDs, deny on unknown KID.
  *Proves:* **I-7**.
* **[G-6: Caveat semantics]** Per-caveat unit + property tests (time skew bounds, CIDR matching, path prefix normalization). Conjunction enforced; any failing caveat denies.
  *Proves:* **I-9**, **I-10**, **I-14**.
* **[G-7: Bounds]** Fuzz + negative tests reject tokens > 4096 B or > 64 caveats; perf test shows **O(caveats)**.
  *Proves:* **I-12**.
* **[G-8: Multi-tenant isolation]** `(tid)` mismatch denies even with valid MAC; cross-tenant keys never accepted.
  *Proves:* **I-13**.
* **[G-9: Zeroization]** Unit tests with `cargo miri` confirm zeroize on drop for key handles and secret buffers (as observable).
  *Proves:* **I-6**, **I-20**.
* **[G-10: PQ adapter]** (feature-gated) Signature adapter vectors for Ed25519 and Dilithium2 (hybrid); disabling the feature removes all code paths.
  *Proves:* **I-17**.
* **[G-11: API stability]** `cargo public-api` + `cargo semver-checks` gate on changes; CHANGELOG updated.
  *Proves:* **I-19**.
* **[G-12: Performance SLO]** Criterion benches: p95 verify latency ≤ **60µs + 8µs × caveats** on baseline dev machine; allocations ≤ 2 per verification.
  *Supports:* **I-12** and perf discipline.
* **[G-13: CI quality bars]** `cargo clippy -D warnings`, `cargo deny` (licenses/dupes/advisories), `cargo fmt --check`.
  *Global quality*.
* **[G-14: Amnesia matrix]** Parameterized tests run verification with `Amnesia(true)` under both host states (`amnesia=ON`, `amnesia=OFF`). Expect allow in ON, deny in OFF.
  *Proves:* **I-21**.
* **[G-15: Governance digest]** Golden vectors include `GovPolicyDigest` caveats. Tests inject a matching and a non-matching digest; expect allow/deny accordingly.
  *Proves:* **I-22**.
* **[G-16: Mint isolation in CI]** Workspace CI asserts `mint-internal` is disabled for all crates except `svc-passport` and tests. `cargo check --tests --no-default-features` passes.
  *Proves:* **I-23** and preserves **I-1**.

### Reviewer checklist (expanded)

* Purity/no I/O ✔ | No unsafe/panic ✔ | Canonical CBOR ✔ | MAC chain KATs ✔
* Constant-time compare ✔ | Rotation/KID ✔ | Conjunction semantics ✔ | Skew bounds ✔
* Bounds (size/caveats) ✔ | Multi-tenant isolation ✔ | Zeroization ✔ | PQ adapter gate ✔
* Perf SLO ✔ | SemVer & public API gates ✔ | No secret logs ✔
* **Amnesia binding** ✔ | **Governance policy digest** ✔ | **Mint isolation** ✔

---

## 5) Anti-Scope (Forbidden)

* **No network or disk I/O**, no TLS, no database calls from `ron-auth`.
* **No SHA-2/HMAC-SHA-256/JWT/OAuth-introspection patterns.** (This library is capabilities + MAC; central introspection is an anti-pattern.)
* **No ambient roles/ACLs** or “allow by default” fallbacks.
* **No global singletons** for keys or config; no env-sourced secrets in the lib.
* **No logging of tokens/secrets**; no debug printing of sensitive structs.
* **No panics/unwrap/expect**; no leaking error messages that reveal caveat details.
* **No mutable global time source**; time must be injected.
* **No secret-bearing `Clone`/`Copy`** on key material types; no `Debug`/`Display` for raw secrets.
* **No ZK/attestation frameworks here** (keep governance/policy binding minimal via digest; do not expand scope into policy engines).

---

## 6) References

* **Project Blueprints:** Hardening Blueprint, Interop Blueprint, App Integration Blueprint, 12 Pillars, Six Concerns, Scaling Blueprint, Concurrency & Aliasing Blueprint.
* **Crates:** `ron-kms` (key custody traits), `svc-passport` (issuance/rotation), consumer services (`svc-gateway`, `svc-overlay`, `svc-index`, `svc-storage`, `svc-mailbox`).
* **Concepts:** Capabilities & attenuation (macaroons), BLAKE3 (keyed mode), deterministic CBOR, Base64URL, constant-time crypto.
* **PQ Direction:** Hybrid signatures (Ed25519+Dilithium2) adapter for cross-org envelopes (feature-gated).

---

## 7) Traceability (MUST → PROOF)

* **I-1** → **G-1, G-16**
* **I-2** → **G-6**
* **I-3** → **G-1, G-6**
* **I-4/11** → **G-4**
* **I-5/18** → **G-3**
* **I-6/20** → **G-9**
* **I-7** → **G-5**
* **I-8/9/14** → **G-6**
* **I-10** → **G-6**
* **I-12** → **G-7, G-12**
* **I-13** → **G-8**
* **I-17** → **G-10**
* **I-19** → **G-11**
* **I-21** → **G-14**
* **I-22** → **G-15**
* **I-23** → **G-16**

---

```



---

## INTEROP.MD
_File 6 of 12_

```markdown
---
title: 🔗 INTEROP — ron-auth
audience: developers, auditors, external SDK authors
msrv: 1.80.0
status: draft
last-updated: 2025-10-04
---

# INTEROP.md — ron-auth

## 0) Purpose

Define the **interop surface** of `ron-auth`, the capability **verification/attenuation** library:

- **Wire format** for capability tokens (deterministic CBOR → Base64URL).
- **Algorithm contract** for MAC chaining (keyed BLAKE3).
- **DTOs & schemas** (Capability, Scope, Caveat, RequestCtx).
- **Header/field conventions** for HTTP/gRPC/OAP1 carriers.
- **Canonical test vector** layout and rules.
- **Trait contracts** (`MacKeyProvider`, optional `SigAdapter`) for cross-crate integration.

This document ensures consistent behavior across services (`svc-gateway`, `svc-overlay`, `svc-index`, `svc-storage`, `svc-mailbox`), `svc-passport` (issuance), `ron-kms` (key custody), and any external SDKs—aligned with Omni-Gate principles.

---

## 1) Protocols & Endpoints

`ron-auth` itself exposes **no network endpoints**. Tokens travel **within** other protocols as bearer credentials.

### 1.1 Carrier Conventions

- **HTTP/1.1+ / HTTP/2 / gRPC**  
  Prefer `Authorization` header with a registered auth-scheme:

```

Authorization: Capability <base64url-token>

````

Fallback (when intermediaries interfere with `Authorization`):  
`X-RON-Capability: <base64url-token>`

- **OAP/1 (framed)**  
Include capability as a metadata field in the envelope payload:
```cbor
{ "cap": <bstr base64url bytes as text or raw bstr>, ... }
````

(See §2 for the token itself.)

* **Message size**
  The **token** (decoded) must be ≤ **4096 bytes** (§Invariants). Carriers must apply their own frame limits separately.

### 1.2 Transport Invariants (host services)

* TLS termination and readiness gates belong to **host services** (not `ron-auth`).
* Services **must not** rely on central introspection for token validity—verification is **offline** using this library.

---

## 2) DTOs / Schemas (Wire-Level)

### 2.1 Capability (CBOR → Base64URL)

**Media type (recommended):** `application/ron-cap+cbOR;v=1`
**On the wire:** Base64URL **without padding** of the canonical CBOR serialization of the structure below.

CBOR logical model (Rust-ish for clarity):

```rust
struct Capability {
  v:   u8,              // version = 1
  tid: String,          // tenant identifier (ASCII, [-._a-zA-Z0-9], 1..64)
  kid: String,          // key id (ASCII, [-._a-zA-Z0-9], 1..64)
  r:   Scope,           // root scope
  c:   Vec<Caveat>,     // ordered, conjunctive caveats
  s:   [u8; 32],        // MAC tag (final chain)
}
```

**Deterministic CBOR rules (canonical):**

* **Maps** use text keys exactly `"v","tid","kid","r","c","s"`.
* **Definite** lengths only; **shortest** integer encoding; **no** floating-point.
* Byte strings (`s`) are length-32 `bstr`.
* Strings are UTF-8; no alternate forms.
* No additional/unknown top-level fields.

> **Compatibility:** Unknown **caveat** variants are handled via `Custom` (see below), not by adding top-level fields.

### 2.2 Scope (CBOR map)

```rust
struct Scope {
  prefix:    Option<String>,   // resource prefix (e.g., "/o/b3:…")
  methods:   Vec<String>,      // e.g., ["GET","PUT"]
  max_bytes: Option<u64>,      // per-request cap
}
```

### 2.3 Caveats (CBOR tagged union as map: `{"t": <str>, "v": <any>}`)

Canonical tag strings and value shapes:

| `t`                   | Value `v`                                   | Notes                            |
| --------------------- | ------------------------------------------- | -------------------------------- |
| `"exp"`               | `u64` (unix seconds)                        | deny if now > exp (± skew)       |
| `"nbf"`               | `u64`                                       | deny if now < nbf (± skew)       |
| `"aud"`               | `tstr`                                      | exact match against audience     |
| `"method"`            | `[tstr]`                                    | subset of verbs/ops              |
| `"path_prefix"`       | `tstr`                                      | normalized path prefix           |
| `"ip_cidr"`           | `tstr`                                      | CIDR in text form                |
| `"bytes_le"`          | `u64`                                       | max allowed body size            |
| `"rate"`              | `{ "per_s": u32, "burst": u32 }`            | host-enforced rate               |
| `"tenant"`            | `tstr`                                      | must == `tid`                    |
| `"amnesia"`           | `bool`                                      | require host amnesia mode        |
| `"gov_policy_digest"` | `tstr` (hex, 64 chars)                      | bind to governance policy digest |
| `"custom"`            | `{ "ns": tstr, "name": tstr, "cbor": any }` | namespaced extension             |

> **Extensibility:** New standard caveats MAY be added in v1.x without breaking wire compatibility. Unknown **standard** tags should cause **deny** unless specifically allowed by config (default is deny).

### 2.4 Request Context (host → library, not on the wire)

```rust
struct RequestCtx<'a> {
  now_unix_s: u64,
  method: &'a str,
  path: &'a str,
  peer_ip: Option<IpAddr>,
  object_addr: Option<&'a str>,
  tenant: &'a str,
  amnesia: bool,
  policy_digest_hex: Option<&'a str>,
  extras: serde_cbor::Value,
}
```

`RequestCtx` is **not** serialized; it is an input to verification.

---

## 3) Algorithms (Normative)

### 3.1 MAC Primitive

* **BLAKE3** keyed mode (`KDF`/MAC) with the symmetric key referenced by `kid` (per `tid`).
* **Constant-time** equality compare for final tag.

### 3.2 Domain Separation & Chain

Let `DS_INIT = "ron-auth/v1\0init"` and `DS_CAV = "ron-auth/v1\0caveat"` (ASCII bytes).

1. **Initial link:**

```
sig_0 = BLAKE3_key( key, DS_INIT || tid || kid || canonical_cbor(r) )
```

2. **For each caveat `c_i` in order (i=0..n-1):**

```
sig_{i+1} = BLAKE3_key( key, DS_CAV || sig_i || canonical_cbor(c_i) )
```

3. **Final tag:** `s = sig_n` (32 bytes).

> **Notes:**
>
> * No random nonce is required in v1 (the chain is bound to content and order).
> * Reordering caveats **invalidates** the token.
> * Any change to `r` or any `c_i` produces a different `s`.

### 3.3 Verification

* Decode Base64URL → CBOR (canonical, strict).
* Enforce **bounds**: size ≤ 4096 B, caveats ≤ 64.
* Resolve `(tid,kid) → key handle` via `MacKeyProvider`.
* Recompute chain; constant-time compare with `s`.
* Evaluate caveats with **conjunctive** semantics (all must pass).
* Fail-closed on: parse errors, bounds, unknown KID, unknown caveat where not explicitly allowed, context failure (amnesia/policy digest).

---

## 4) Traits (Integration Contracts)

### 4.1 `MacKeyProvider` (required)

```rust
trait MacKeyProvider {
  fn mac_handle(&self, tenant: &str, kid: &str) -> Result<Box<dyn MacHandle>, AuthError>;
}

trait MacHandle: Send + Sync {
  fn mac(&self, msg: &[u8]) -> [u8; 32]; // keyed BLAKE3
}
```

* Implemented by **`ron-kms`** or service-local adapters.
* **Never** expose raw key material to `ron-auth`.

### 4.2 `SigAdapter` (optional, `pq-hybrid`)

For cross-org **signature envelopes** (avoid MAC key sharing):

```rust
trait SigAdapter: Send + Sync {
  fn verify_hybrid(&self, payload: &[u8], sig: &[u8], kid: &str) -> Result<(), AuthError>;
}
```

This verifies an outer **envelope** over `payload = canonical_cbor(Capability without 's')`.
Default build excludes this.

---

## 5) Canonical Test Vectors

Vectors live under: `crates/ron-auth/tests/vectors/` and are versioned by **wire version**.

### 5.1 File Layout

```
tests/vectors/
  v1/
    readme.md                # provenance and generation instructions
    capability_roundtrip.json
    mac_chain.json
    deny_cases.json
    interop_suite.csv        # tabular summary for SDKs
```

### 5.2 Vector Schema (examples)

* **`capability_roundtrip.json`**

  ```json
  {
    "name": "allow_get_prefix",
    "cap_cbor_hex": "a66576... (canonical CBOR hex)",
    "cap_b64url": "p2V2... (no padding)",
    "tid": "tenant-1",
    "kid": "kid-2025-10",
    "scope": { "prefix": "/o/b3:abcd", "methods": ["GET"], "max_bytes": 1048576 },
    "caveats": [
      { "t": "exp", "v": 1767225600 },
      { "t": "method", "v": ["GET"] },
      { "t": "path_prefix", "v": "/o/b3:abcd" }
    ],
    "mac_hex": "b3b3...32bytes",
    "ctx": {
      "now_unix_s": 1767225599,
      "method": "GET",
      "path": "/o/b3:abcd/some",
      "tenant": "tenant-1"
    },
    "expect": "allow"
  }
  ```
* **`deny_cases.json`**: table of `{name, reason, token, ctx}` using canonical **deny reason strings** (`"parse.b64"`, `"kid.unknown"`, `"caveat.exp"`, …).
* **`mac_chain.json`**: lists intermediate `sig_i` values for SDK implementers.

### 5.3 Generation & Validation

* Vectors are generated by rust tests and **re-validated** in CI.
* External SDKs must reproduce `s` and decisions exactly for the same inputs.
* Any change to vectors → **major** if it alters v1 semantics.

---

## 6) Error / Deny Taxonomy (Interop-Stable)

The library exposes a **non-exhaustive** `DenyReason` enum plus **stable** strings via `as_str()` that SDKs/log pipelines must use:

```
parse.b64 | parse.cbor | parse.bounds | schema.unknown_field
mac.mismatch | kid.unknown | tenant.mismatch
caveat.exp | caveat.nbf | caveat.aud | caveat.method
caveat.path | caveat.ip | caveat.bytes | caveat.rate
caveat.tenant | caveat.amnesia | caveat.policy_digest
caveat.custom.unknown | caveat.custom.failed
```

> These strings are part of the **observability contract** (see `OBSERVABILITY.md`).
> Adding **new** reason strings is **minor**; renaming existing ones is **major**.

---

## 7) Interop Guarantees

* **Wire stability:** CBOR map keys (`"v","tid","kid","r","c","s"`), canonical encoding rules, and MAC chain domain strings are **stable within v1**.
* **Extensibility:** `Caveat` is open via `"custom"` and new standard tags; unknown standard tags → **deny** unless explicitly allowed.
* **Offline verification:** No network calls; `MacKeyProvider` is the only dependency.
* **Tenant/KID binding:** Decisions are **namespaced** by `(tid,kid)`; cross-tenant acceptance is impossible unless a host implements explicit cross-trust.
* **SemVer discipline:** Any change that affects wire encoding, domain constants, or evaluation semantics is **major**.
* **Auditability:** Vectors and their generator source are checked in; CI enforces determinism.

---

## 8) Cross-Crate Touchpoints

* **`svc-passport` (issuance/rotation):**

  * Mints tokens conforming to §2 and computes `s` using §3.
  * Publishes active `KID` windows; removal immediately invalidates tokens with that KID.
* **`ron-kms` (keys):**

  * Implements `MacKeyProvider` returning opaque `MacHandle`s.
  * Enforces rotation policy and tenant isolation.
* **Consumers (`svc-gateway`, `svc-*`):**

  * Extract token from header (HTTP) or metadata (OAP/1/gRPC).
  * Construct `RequestCtx` from the live request and call `verify()`.

---

## 9) References

* `docs/IDB.md` — invariants & proofs
* `docs/CONFIG.md` — verifier config & bounds
* `docs/SECURITY.md` — threat model & handling
* `docs/OBSERVABILITY.md` — deny reason strings & metrics
* Omni-Gate Interop principles (GMI-1.6)

---

✅ With this spec, `ron-auth` remains **portable and exact**: any SDK can implement the same CBOR + MAC chain + caveat evaluation and achieve bit-for-bit compatibility with Rust services—without calling back to a central authority.

```
```


---

## OBSERVABILITY.MD
_File 7 of 12_

```markdown
# 📈 OBSERVABILITY.md — ron-auth

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible — though this crate is sync/pure)*

---

## 0) Purpose

Define **what is observable**, **how it is exposed**, and **how it’s used** for:

- Metrics (Prometheus/OTEL via host integration)
- Health/readiness semantics (N/A for this **library**; host services own them)
- Logs (JSON schema & redaction guidance)
- Tracing spans & correlation
- Alerts & SLOs (for the **verify path**)

> `ron-auth` is a **pure library** (no I/O, no HTTP endpoints). It **does not** bind `/metrics` or emit logs by itself.  
> Instead, it exposes **typed outcomes** and **lightweight hooks** so **host services** export metrics/logs/traces consistently.

---

## 1) Metrics (Prometheus-style)

### 1.1 Library “golden” metrics (to be emitted by hosts)

| Metric name | Type | Labels | Meaning |
|---|---|---|---|
| `ron_auth_verify_total` | Counter | `result="allow|deny"` | Total verifications and outcome |
| `ron_auth_deny_total` | Counter | `reason` | Denies by **normalized reason** (see 1.4) |
| `ron_auth_parse_error_total` | Counter | `kind="b64|cbor|bounds|schema"` | Decode/shape failures (pre-MAC) |
| `ron_auth_unknown_kid_total` | Counter | `tenant`,`kid` | KID not found during verification |
| `ron_auth_duration_us` | Histogram | `caveats_bucket` (optional, e.g., `0-4`,`5-16`,`17+`) | End-to-end verification time (μs) |
| `ron_auth_token_size_bytes` | Histogram | — | Size distribution of tokens (decoded) |

> **Why these:** they directly support IDB invariants and Acceptance Gates (G-3/G-4/G-5/G-6/G-12), and are minimal enough to avoid PII/secret leakage.

#### 1.2 Minimal label discipline
- **Do not** attach raw `tenant` except where operationally necessary. If used, ensure low cardinality and **no secrets**.
- Never label with raw token material. If correlation is required, use **redacted digest** (see §3.2).

#### 1.3 Registration discipline (host side)
- Metrics are registered **once** in the host’s `Metrics::new()`.  
- Keep handles (`Counter`, `Histogram`) and pass to verifier wrapper or use a global `metrics` facade (feature-gated).

#### 1.4 Canonical deny reasons (stable strings)
These strings are **part of the contract** for dashboards/alerts:

```

"parse.b64" | "parse.cbor" | "parse.bounds" | "schema.unknown_field"
"mac.mismatch" | "kid.unknown" | "tenant.mismatch"
"caveat.exp" | "caveat.nbf" | "caveat.aud" | "caveat.method"
"caveat.path" | "caveat.ip" | "caveat.bytes" | "caveat.rate"
"caveat.tenant" | "caveat.amnesia" | "caveat.policy_digest"
"caveat.custom.unknown" | "caveat.custom.failed"

````

> The crate exposes a `DenyReason` enum → **these exact strings** via `as_str()`; hosts should log/metric using these, unchanged.

---

## 2) Health & Readiness

**N/A for this library.**  
- Liveness/readiness endpoints (`/healthz`, `/readyz`) belong to **host services**.  
- Readiness should **not** depend on `ron-auth`; however, a host may include a “policy/KID cache warmed” indicator if it wraps lookups or policy digests.

---

## 3) Logs

### 3.1 Format (host guidance)
- JSON lines. Required fields:
  - `ts` (RFC3339), `level`, `service`, `event` (e.g., `auth.verify`)
  - `result` (`allow|deny`)
  - `reason` (from `DenyReason::as_str()`; omit on allow)
  - `tenant` (optional, low-cardinality)
  - `corr_id` (propagated)
  - `latency_us` (int)
  - `token_digest8` (optional; **redacted**, see below)

### 3.2 Redaction & secrets
- **Never** log raw tokens/caveats/keys.
- If correlation is needed, compute `token_digest8 = hex(b3(token_bytes))[0..8]`.  
- Config diffs/logs must redact secrets/keys and omit token bodies.

---

## 4) Tracing & Correlation

- **Feature `trace` (optional)**: `ron-auth` can annotate `verify()` with `tracing` spans via `#[instrument(skip(token_b64, key_provider))]`.
  - Span name: `lib.ron_auth.verify`
  - Span fields: `tenant`, `kid`, `caveats_count`, `token_size`
  - Events: `deny` (with normalized `reason`), `allow`
- **OpenTelemetry**: exporting is a **host** concern. If hosts enable OTEL, spans propagate naturally.

---

## 5) Alerts & SLOs (verify path)

### 5.1 SLOs (library-level, enforced by host benches/alerts)

- **Latency SLO (dev HW)**: p95 `ron_auth_duration_us` ≤ **60 + 8×caveats** μs  
  (e.g., 10 caveats → p95 ≤ 140 μs)
- **Correctness SLO**: `ron_auth_parse_error_total` very low under normal traffic; spikes indicate bad clients or regressions.
- **Reliability SLO**: `ron_auth_unknown_kid_total` near-zero in steady state; spikes indicate rotation issues.

### 5.2 Alerts (examples)

- `rate(ron_auth_unknown_kid_total[5m]) > 0` for 10m → **KID rotation misconfig**
- `histogram_quantile(0.95, sum(rate(ron_auth_duration_us_bucket[5m])) by (le)) > 2x SLO` → **perf regression**
- `increase(ron_auth_deny_total{reason=~"mac.*|parse.*"}[10m]) > N` → **attack or bug**
- `increase(ron_auth_deny_total{reason="caveat.custom.unknown"}[10m]) > 0` → **namespace drift**

> Each alert should link to a **RUNBOOK**: check rotation window, policy digest propagation, clock skew, and host amnesia mode.

---

## 6) CI / Enforcement

- **Unit tests** ensure `DenyReason::as_str()` is **stable** (golden map).
- **Criterion benches** publish `ron_auth_duration_us` locally and compare to SLOs (fail on large regressions).
- **No-log guarantee**: tests assert that the crate does not emit logs on success/failure (library purity).
- **Metrics contract test** (host example): verify counters/histograms increment as expected when wrapping `verify()`.

---

## 7) Integration patterns (copy-paste)

### 7.1 Thin wrapper that instruments verification (host side)

```rust
// host/src/auth_obs.rs
use std::time::Instant;
use prometheus::{CounterVec, Histogram, IntCounterVec};
use ron_auth::{verify_token, DenyReason, RequestCtx};
use ron_auth::config::VerifierConfig;

pub struct AuthMetrics {
    pub verify_total: CounterVec,      // labels: result
    pub deny_total: IntCounterVec,     // labels: reason
    pub parse_error_total: IntCounterVec, // labels: kind
    pub unknown_kid_total: IntCounterVec, // labels: tenant,kid (optional)
    pub duration_us: Histogram,
    pub token_size_bytes: Histogram,
}

pub fn verify_with_metrics(
    m: &AuthMetrics,
    cfg: &VerifierConfig,
    token_b64: &str,
    ctx: &RequestCtx<'_>,
    keyp: &impl ron_auth::MacKeyProvider,
) -> Result<ron_auth::Decision, ron_auth::AuthError> {
    let start = Instant::now();

    // Optional size metric (decoded)
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(token_b64) {
        m.token_size_bytes.observe(bytes.len() as f64);
    }

    match verify_token(cfg, token_b64, ctx, keyp) {
        Ok(decision) => {
            m.verify_total.with_label_values(&["allow"]).inc();
            m.duration_us.observe(start.elapsed().as_micros() as f64);
            Ok(decision)
        }
        Err(err) => {
            use ron_auth::AuthError::*;
            match &err {
                ParseBase64 => { m.parse_error_total.with_label_values(&["b64"]).inc(); }
                ParseCbor   => { m.parse_error_total.with_label_values(&["cbor"]).inc(); }
                Bounds      => { m.parse_error_total.with_label_values(&["bounds"]).inc(); }
                SchemaUnknownField => { m.parse_error_total.with_label_values(&["schema"]).inc(); }
                UnknownKid { tenant, kid } => {
                    m.unknown_kid_total.with_label_values(&[tenant, kid]).inc();
                }
                _ => { /* fall through to deny reason below */ }
            }
            m.verify_total.with_label_values(&["deny"]).inc();
            if let Some(reason) = err.deny_reason() {
                m.deny_total
                    .with_label_values(&[reason.as_str()])
                    .inc();
            }
            m.duration_us.observe(start.elapsed().as_micros() as f64);
            Err(err)
        }
    }
}
````

> This wrapper demonstrates label normalization, separation of parse vs. decision failures, and latency observation.

### 7.2 Tracing instrumentation (optional)

```rust
#[cfg(feature = "trace")]
#[tracing::instrument(
    name = "lib.ron_auth.verify",
    skip_all,
    fields(tenant = ctx.tenant, caveats = ?token.caveats_len())
)]
fn verify_instrumented(...) -> Result<Decision, AuthError> { /* call into core */ }
```

---

## 8) Log event cheatsheet (host)

| Event                  | Level  | Fields                                                         | When                                                              |
| ---------------------- | ------ | -------------------------------------------------------------- | ----------------------------------------------------------------- |
| `auth.verify`          | `INFO` | `result`, `latency_us`, `tenant?`, `token_digest8?`, `corr_id` | On every verification (sample in prod if high QPS)                |
| `auth.deny`            | `WARN` | `reason`, `tenant?`, `corr_id`, `latency_us`                   | On deny; consider rate-limiting noisy reasons (e.g., `parse.b64`) |
| `auth.rotation.window` | `INFO` | `active_kid`, `prev_kids`, `window_days`                       | On rotation rollouts (host-side)                                  |
| `auth.policy.digest`   | `INFO` | `digest`, `source`                                             | When governance digest changes (host-side)                        |

---

## 9) Dashboards (suggested panels)

* **Outcomes:** stacked `ron_auth_verify_total{result}` over time
* **Denies by reason:** top-N table of `ron_auth_deny_total` deltas
* **Latency:** p50/p95/p99 from `ron_auth_duration_us`
* **Unknown KID:** time series, broken down by tenant
* **Token size distribution:** histogram heatmap (sanity check against bounds)

---

## 10) Stability & versioning

* Metric names and **deny reason strings** are **SemVer-stable API** for this crate.
* Any rename/add/remove is a **breaking change** (major version) unless purely additive with clear defaults (new reason strings start as **off-path** until documented).

---

## 11) Security notes (observability)

* Never include token plaintext or caveat bodies in metrics, logs, or traces.
* Redaction rule: **digest-then-truncate**, never substring of raw token.
* Correlate with `corr_id` and `tenant` (if safe), not with secrets.

---

## 12) N/A items (library context)

* `/metrics`, `/healthz`, `/readyz` endpoints — **host-only**.
* Bus lag & supervised restarts — **host-only**.
* I/O timeouts & framing metrics — **host-only**.

---

## 13) Review cadence

* Re-validate SLOs and alert thresholds **every 90 days** or after perf-affecting changes.
* Keep this file in scope for any change to error taxonomy or `DenyReason` strings.

---

```
```


---

## PERFORMANCE.MD
_File 8 of 12_


---

```markdown
# ⚡ PERFORMANCE.md — `ron-auth`

---
title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-04
audience: contributors, ops, perf testers
---

## 0) Purpose

Defines the **performance profile** for `ron-auth`, a **pure verification library** (no I/O, no background tasks):

- Lib-level SLOs (latency/allocations) + host-parallel throughput guidance.
- Benchmarks & workloads it must sustain.
- Perf harness & profiling tools.
- Scaling knobs, known bottlenecks, and a triage runbook.
- CI regression gates to prevent silent drift.

Ties into **Scaling Blueprint v1.3.1**, **Omnigate Bronze→Gold**, and **Perfection Gates** (Gate **F** = perf regressions barred; Gate **L** = scaling under chaos validated).

> **Amnesia Mode:** The library is stateless by design and holds no keys. Hosts are responsible for key zeroization and any amnesia toggles on the request context.

---

## 1) SLOs / Targets (Library)

### Latency (p95 per verification)
- **Conservative baseline (portable across HW):**  
  p95 ≤ **60 µs + (8 µs × caveats)** for 4 KiB tokens.  
  (E.g., 10 caveats → p95 ≤ 140 µs.)

- **Typical modern x86 (informative note):**  
  Expect **10–20 µs base + ~1–2 µs/caveat** on high-end desktops; we still enforce the conservative SLO above to avoid over-optimism across fleets.

### Allocations / op
- ≤ **2 allocations** steady (decode + minimal workspace), verified by heaptrack.

### Throughput (host-parallel guidance)
- On an **8-core x86_64 workstation** (3.5 GHz class), `--release`, 4 KiB tokens, 10 caveats, saturating request-level parallelism:  
  **≥ 1.0M verifies/sec** sustained.  
- On **CI (ubuntu-latest, 2 vCPU)**: track **relative deltas**, not absolutes; regressions >10% fail.

### Reliability / budgets (host-observed)
- Parse errors ≈ 0 in steady state (spikes indicate client or parser drift).
- Unknown KID ≈ 0 in steady state (spikes indicate rotation drift).

### Resource ceilings / bounds
- `max_token_bytes` default **4096**; validated range **[512, 16384]**.
- `max_caveats` default **64**; validated range **[1, 1024]**.  
These are **MUST** bounds to keep perf predictable and abuse-resistant.

### PQ-hybrid envelope (feature `pq-hybrid`)
- No hard SLO yet; **measure & publish** overhead vs. classical signatures. Target keeping overheads within deployment budgets as the QUANTUM plan matures.

---

## 2) Benchmarks & Harness

**Micro-bench (Criterion):**
- `verify_small_token` (1–3 caveats; cold/warm).
- `verify_many_caveats` (16/32/64 caveats; enforces slope).
- `verify_skewed` (`exp/nbf`, realistic `RequestCtx`).
- `verify_unknown_kid` (constant-time deny path).
- `verify_custom_caveats` (registrar present; unknowns deny).

**PQ benches (`pq-hybrid`):**
- `sig_adapter_ed25519` vs `sig_adapter_dilithium2` — report the delta (not a hard gate…yet).

**Integration load (host wrapper):**
- Wrap `verify()` with Prometheus histograms/counters; drive with `bombardier`/`wrk`. Collect latency distributions and error taxonomy.

**Profiling tools:**
- `cargo flamegraph` for CPU hotspots (decode/parse/MAC chain).
- `tokio-console` (if wrapper is async) to catch stalls around the call boundary.
- `heaptrack` to verify ≤2 allocations/op.
- `hyperfine` for quick end-to-end CLI latency of the wrapper, if any.

_Run locally:_
```

cargo bench -p ron-auth
cargo flamegraph -p ron-auth --bench verify_many_caveats
hyperfine 'cargo bench -p ron-auth --bench verify_small_token'

````

---

## 3) Scaling Knobs

- **Bounds:** tune `max_caveats` and `max_token_bytes` within validated ranges to cap per-op cost.
- **Key provider:** implement `MacKeyProvider` as an **O(1)** KID→handle map; during rotations, atomically swap (current + N previous) to avoid `unknown_kid` spikes.
- **Zero-copy:** decode once; avoid buffer churn in host wrappers (retain ≤2 alloc/op).
- **Parallelism:** scale throughput via host-level concurrency; verifier is `Send + Sync` and re-entrant.
- **PQ toggles:** enable only where needed; always record deltas per algorithm.

---

## 4) Bottlenecks & Known Limits

- **Decode + CBOR parse** (~size-sensitive, but bounded by `max_token_bytes`).
- **Keyed BLAKE3 MAC chain** (O(n_caveats); latency scales linearly with caveats).
- **KID lookup** (must be constant-time hashmap or similar; spikes appear as `unknown_kid`).
- **PQ envelope verify** (optional) — higher CPU cost; track and report.

> **Deliberate choice:** no internal caches in the lib; determinism and short codepaths beat micro-caching. Host may cache at higher layers.

---

## 5) Regression Gates (CI)

CI fails if any hold:

- p95 latency ↑ **>10%** at any tested caveat count, or slope deviates materially from the linear target.
- Host-parallel throughput (on the same runner type) ↓ **>10%**.
- Allocations/op > **2** steady or become unstable.
- Error counters (`parse_error_total`, `unknown_kid_total`) exceed steady thresholds under load.

**Baselines:** commit JSON/CSV snapshots under `testing/performance/baselines/ron-auth/`.  
**Waivers:** allowed only when traced to an upstream dependency with a filed issue and documented CHANGELOG note.

> **Minimal CI guard (example)**  
> Add `.github/workflows/perf.yml`:
>
> ```yaml
> name: perf
> on:
>   workflow_dispatch: {}
>   schedule: [{cron: "0 8 * * 1"}]  # weekly
> jobs:
>   bench:
>     runs-on: ubuntu-latest
>     steps:
>       - uses: actions/checkout@v4
>       - uses: dtolnay/rust-toolchain@stable
>       - run: cargo bench -p ron-auth
>       - name: Compare against baselines
>         run: bash scripts/perf_guard.sh target/criterion testing/performance/baselines/ron-auth || exit 1
> ```
>
> And `scripts/perf_guard.sh` compares current Criterion estimates to baselines and exits non-zero on >10% regressions.

---

## 6) Perf Runbook (Triage)

1. **Bounds sanity:** Ensure `max_token_bytes` / `max_caveats` match intended tier. Tighten if abuse indicated.
2. **Metrics first:** Check latency histogram p95, `parse_error_total`, `unknown_kid_total`, `deny_total{reason=…}`.
3. **Flamegraph:** Attribute time among decode/parse, MAC chain, KID lookup, and (if enabled) `SigAdapter` verify.
4. **Key provider audit:** Confirm O(1) lookups and rotation window (current + N prev). Fix drift if `unknown_kid` spikes.
5. **Allocator churn:** Run heaptrack; keep ≤2 alloc/op. Hunt stray copies in wrappers.
6. **PQ toggle pivot:** If recently enabled and deltas breach budget, disable or reduce surface area; file QUANTUM follow-up.
7. **Chaos replay:** Re-run with compression/jitter disabled; verify slope stays linear with caveats.

---

## 7) Acceptance Checklist (DoD)

- [ ] SLOs captured (p95 ≤ 60 µs + 8 µs×caveats; ≤2 alloc/op; host-parallel ≥ 1.0M ops/s on 8-core).  
- [ ] Criterion benches run locally + CI; baselines stored and compared.  
- [ ] Flamegraph and heaptrack traces collected at least once per minor.  
- [ ] Scaling knobs documented (bounds, KID map, PQ features).  
- [ ] Regression gates wired into CI with failure thresholds.  
- [ ] Runbook updated and linked from alerts.

---

## 8) Appendix

### A. Hardware Baselines (fingerprint & expectations)

| Tier | CPU example | Cores | Note | Enforcement |
|----|----|----:|----|----|
| CI | `ubuntu-latest` (2 vCPU) | 2 | Relative deltas only | ✔ |
| Dev | Ryzen 7 / i7 (3.5 GHz) | 8 | ≥ 1.0M ops/s host-parallel | ✔ |
| Edge | Apple M-class / ARM | 4 | Track & report (no hard SLO) | ✔ |

Capture runner fingerprint (CPU model, rustc version, flags) alongside baseline artifacts.

### B. Hot-Path Sketch (Mermaid)

```mermaid
flowchart LR
  A[Base64URL decode] --> B[CBOR parse]
  B --> C[MAC chain (keyed BLAKE3) per caveat]
  C --> D[Evaluate caveats (exp/nbf/path/...)]
  D --> E[Decision: Allow / Deny{reason}]
````

### C. PQ posture

* Features gated (`pq-hybrid`); interop intact; report per-algorithm overheads and keep within deployment budgets as QUANTUM milestones progress.

### D. History

* Record notable regressions + fixes; include perf notes in CHANGELOG even when the public API is unchanged.

```

---


---

## QUANTUM.MD
_File 9 of 12_


---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing — ron-auth
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, security auditors, ops
crate: ron-auth
crate-type: lib | policy
pillar: 3  # Identity & Keys
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose
`ron-auth` verifies **capability tokens** (macaroon-style). Its correctness underpins the entire identity plane: if verification can be forged, upper-layer economics collapse.  
This document explains how `ron-auth` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto while keeping interop and policy stability.

Scope: algorithms in use, custody boundaries, runtime knobs, telemetry, tests, rollout plan, and “Harvest-Now, Decrypt-Later” (HNDL) exposure.

---

## 1) Exposure Assessment (What’s at risk?)

| Class | Algorithm / Usage | Quantum Risk | Notes |
|---|---|---:|---|
| Public-key usage (Shor) | Ed25519 (signatures via optional `SigAdapter` for cross-org envelopes) | High **if used alone** | Optional only; disabled unless feature-gated. |
| Key exchange | N/A in this crate | – | Transport handled by hosts (ron-transport). |
| Symmetric/Hash (Grover) | Keyed **BLAKE3-256** (MAC chain) | Low | 256-bit → ~128-bit effective post-Grover; keep 256-bit outputs. |
| Data at rest | None (no persistence) | – | Tokens are transient in RAM only. |
| Token TTL / HNDL | Default TTL **≤ 5 minutes** (policy) | **Low HNDL** | Short lifetime makes harvested bytes useless later. |
| Blast radius | Forged envelope sigs could spoof cross-tenant tokens for affected tenants if classical-only signatures were broken. |

> **HNDL risk:** Low. `ron-auth` stores no ciphertext or long-lived secrets; only transient token bytes.

---

## 2) Current Crypto Profile (Today)

| Function | Algorithm | Library | Notes |
|---|---|---|---|
| MAC chain | **Keyed BLAKE3-256** | `blake3` | Deterministic O(n_caveats) chain. |
| Hashing | BLAKE3-256 | `blake3` | Keep 256-bit outputs. |
| Envelope signatures (optional) | **Ed25519** | `ed25519-dalek` v2 | Only via `SigAdapter` when feature-enabled. |
| Key custody | **ron-kms / HSM** | – | Verifier sees opaque handles only. |
| Rotation | ≥ every **90 days** | – | Accept current + N previous. |

---

## 3) Target PQ Posture (Where we’re going)

| Plane | Goal Algorithm | Adoption | Comment |
|---|---|---|---|
| MAC / Hash | Keep **BLAKE3-256** | Stable | Grover-aware sizing already adequate. |
| Envelope Signatures | **Hybrid** Ed25519 + **ML-DSA (Dilithium2)** | M2 optional → M3 recommended default | Maintain classical interop; add PQ assurance. |
| KMS / KEX (outside) | **Hybrid** X25519 + **ML-KEM (Kyber-768)** | In `ron-kms`/transport | Out of scope for this crate’s code; tracked for completeness. |

**Back-compat:** Classical Ed25519 remains supported through M3; hybrid becomes the default thereafter (policy-controlled per tenant).

---

## 4) Feature Flags & Config (How to turn it on)

```toml
[features]
pq = []                 # base PQ plumbing/types
pq-hybrid = ["pq"]      # enable hybrid envelope verify (classical + PQ)
pq-sign = ["pq"]        # enable PQ signature verification paths
pq-only = []            # compile-time guard to refuse classical signatures
````

```ini
# VerifierConfig (host-provided)
pq_hybrid = true              # default off until M3
pq_sign_algo = "ml-dsa"       # "ml-dsa" | "slh-dsa"
pq_only = false               # if true, deny classical-signature envelopes
key_rotation_days = 90
```

* **Interoperability:** If peer lacks PQ, `pq_hybrid=true` still verifies classical; with `pq_only=true`, verification **denies** with a clear reason.
* **Metrics:** always emit PQ labels (even if 0) for easy adoption.

---

## 5) Migration Plan (Milestones)

| Milestone       | Deliverables                                                                                                                                                    |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **M1 (Bronze)** | Add `pq` features; compile stubs; document exposure; baseline classical perf.                                                                                   |
| **M2 (Silver)** | Implement **hybrid** envelope verify (Ed25519 + Dilithium2); interop tests (classical↔classical, hybrid↔hybrid, hybrid↔classical); record perf deltas per arch. |
| **M3 (Gold)**   | Default `pq_hybrid=true` for identity plane; publish dual vectors; update RUNBOOK/SECURITY; dashboards show PQ adoption.                                        |
| **Post-M3**     | Optional `pq_only` environments; begin sunsetting pure-classical on external edges when partners ready.                                                         |

---

## 6) Invariants (MUST)

* **[PQ-I1]** Any signature path MUST provide ≥ **128-bit** PQ security (e.g., ML-DSA level 2+).
* **[PQ-I2]** `pq_only=true` ⇒ classical-only envelopes **deny** with `deny.reason="pq.unsupported"`.
* **[PQ-I3]** MAC keys & hashes remain **256-bit**.
* **[PQ-I4]** Key rotations must migrate handles atomically; no silent downgrade to weaker algos.
* **[PQ-I5]** Golden vectors cover classical **and** hybrid cases; CI enforces parity.
* **[PQ-I6]** PQ feature builds pass the matrix; decisions are identical to classical except for algo labels.
* **[PQ-E1] (ECON tie)** PQ upgrades MUST **not** introduce any “free escalation” of authority or broaden allow-sets. Tests assert that hybrid mode is **at most** as permissive as classical (i.e., Allow_hybrid ⊆ Allow_classical unless policy explicitly widens).

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with algo labels:

* `ron_auth_verify_total{pq="off|hybrid|pq-only"}`
* `ron_auth_signature_total{algo="ed25519|ml-dsa|hybrid"}`
* `ron_auth_signature_fail_total{algo,reason}`
* `crypto_latency_seconds{op="verify",algo}` (histogram)

**Readiness (host):** if policy mandates PQ and peer/stack can’t negotiate it, `/readyz` fails.
**Logs:** structured JSON includes `pq_state`, `algo`, `peer_mode`, and stable `reason`.

---

## 8) Testing & Verification

* **Unit / property:**

  * Hybrid verify is **monotonic** (never accepts a token that classical would deny unless policy explicitly allows).
  * Corrupt PQ signatures ⇒ `pq.sig.invalid`; never panic.

* **Interop suite:**

  * classical↔classical, hybrid↔hybrid, hybrid↔classical (if `pq_only=false`).
  * Deny with `pq.unsupported` when `pq_only=true` and peer is classical.

* **Fuzz:** PQ envelope decoders, negotiation paths, error taxonomy.

* **Perf:**

  * Record verify costs per algo & arch (x86/ARM).
  * **Expectation:** Dilithium verify can be **5–50×** Ed25519 depending on libs/arch; our target is **“within SLO budget”** (see PERFORMANCE.md) rather than a fixed % cap.

* **Security drill:**

  * Flip `pq_only=true`; confirm classical envelopes deny cleanly with auditable reasons.
  * Rotation drills unchanged.

---

## 9) Risks & Mitigations

| Risk             | Impact               | Mitigation                                                                             |
| ---------------- | -------------------- | -------------------------------------------------------------------------------------- |
| PQ perf overhead | Higher CPU/latency   | Parallelize verify; cache per-request results; keep MAC chain fast; track perf labels. |
| Library churn    | Interop breaks       | Wrap via `SigAdapter`; pin versions; golden vector parity in CI.                       |
| Downgrade abuse  | Loss of PQ guarantee | Enforce `pq_only` when mandated; alert on downgraded sessions.                         |
| Key size bloat   | Memory/IPC overhead  | Opaque handles; registrar partitioning by tenant.                                      |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL risk **Low** with TTLs documented.
* [ ] PQ feature set compiles (`--features pq,pq-hybrid,pq-sign,pq-only`).
* [ ] Interop matrix passes; `pq_only` denies classical with stable reason.
* [ ] PQ metrics labeled & dashboards updated.
* [ ] Perf numbers recorded per arch; SLO budget respected.
* [ ] RUNBOOK / SECURITY / GOVERNANCE cross-links updated.

---

## 11) Role Preset (Identity/Policy)

| Aspect          | ron-auth Default             | Upgrade Path                                            |
| --------------- | ---------------------------- | ------------------------------------------------------- |
| Feature default | `pq_hybrid=false` (M1–M2)    | → `true` (M3)                                           |
| Signature mode  | Classical Ed25519            | Hybrid Ed25519 + ML-DSA                                 |
| Custody         | `ron-kms` (HSM/KMS)          | Hybrid KEM window (X25519 + Kyber-768) in KMS/transport |
| Perf target     | Fit within verify SLO budget | Monitor per-algo labels; optimize wrappers              |
| Audit scope     | Vectors + metrics + alerts   | Add PQ adoption KPIs                                    |

---

## 12) PQ Ciphersuites Policy (Normative)

* **Preferred signatures (M3):** `hybrid(ed25519, ml-dsa-65)` (Dilithium2 class).
* **Acceptable alternatives:** `hybrid(ed25519, slh-dsa-s128)` if ML-DSA unavailable.
* **KEM (outside):** `hybrid(x25519, ml-kem-768)` for KMS/transport layers.
* **Hash/MAC:** BLAKE3-256 retained; no change required.
* **Deprecation path:** pure classical signatures to be discouraged post-M3 and eventually refused when tenant policy marks `pq_only=true`.

---

## 13) CI Matrix (Minimal, copy-paste)

```yaml
name: pq-matrix
on:
  pull_request: {}
  push:
    branches: [main]
jobs:
  test-pq:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        features:
          - ""
          - "pq"
          - "pq,pq-hybrid"
          - "pq,pq-sign"
          - "pq,pq-hybrid,pq-sign"
          - "pq,pq-hybrid,pq-sign,pq-only"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build & test
        run: |
          FEAT="${{ matrix.features }}"
          if [ -n "$FEAT" ]; then
            cargo test -p ron-auth --features "$FEAT" --all-targets -- --nocapture
          else
            cargo test -p ron-auth --all-targets -- --nocapture
          fi
      - name: Golden vector parity
        run: cargo test -p ron-auth --test allow_deny_vectors -- --nocapture
```

---

## 14) Appendix

**Chosen Algorithms (target M3):**

* Sign: **hybrid** Ed25519 + ML-DSA (Dilithium2).
* KEM (external): **hybrid** X25519 + ML-KEM (Kyber-768).
* MAC/Hash: **Keyed BLAKE3-256** (unchanged).

**Libraries (anticipated):** `ed25519-dalek` v2 + `pqcrypto`/`liboqs-rust` adapters behind `SigAdapter`.

**Interop notes:** Hybrid signatures decode via `SigAdapter`; classical fallback allowed unless `pq_only=true`.

**Change log:**

| Date       | Change                                                                        | Owner |
| ---------- | ----------------------------------------------------------------------------- | ----- |
| 2025-10-04 | v2: perf wording clarified; ECON invariant added; PQ policy + CI matrix added | SW    |

```
```


---

## RUNBOOK.MD
_File 10 of 12_


---

````markdown
---
title: RUNBOOK — ron-auth
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-04
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — ron-auth

## 0) Purpose
Operational manual for **ron-auth** (pure verification library): startup (as embedded in hosts), health semantics, diagnostics, failure modes, recovery, scaling notes, and security ops.  
Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview
- **Name:** `ron-auth`
- **Role:** Capability **verification & attenuation** library (macaroon-style) — **no I/O, no endpoints**; hosts instrument and export metrics/logs/traces. See [API.md](./API.md) and [OBSERVABILITY.md](./OBSERVABILITY.md).
- **Criticality Tier:** 1 (critical function embedded in many services; treat as blocking if verification is required at ingress).
- **Dependencies (runtime contracts):**
  - [`MacKeyProvider`](./API.md#mackeyprovider) → `(tenant, kid) -> MacHandle` (opaque keyed-BLAKE3 handle). Keys never reside in this crate.
  - Optional [`SigAdapter`](./API.md#sigadapter) (feature `pq-hybrid`) for cross-org signature **envelopes**.
  - Issuance/rotation via `svc-passport` (caps) and custody via `ron-kms`. See [SECURITY.md](./SECURITY.md#key-custody).
- **Ports Exposed:** **N/A (library)** — `/metrics`, `/readyz` live on hosts that wrap `verify()`. See [OBSERVABILITY.md](./OBSERVABILITY.md#golden-metrics).
- **Data Flows:** `token_b64` + `RequestCtx` + KID→key lookup → **Decision (allow/deny + reason)**; host records metrics/logs. See [API.md](./API.md#decision).
- **Version Constraints:** SemVer; reason strings and bounds are frozen API. See [IDB.md](./IDB.md#invariants) and [CONFIG.md](./CONFIG.md#verifierconfig).

---

## 2) Startup / Shutdown
**This crate is not a process.** It’s configured and used by a host (e.g., `svc-gateway`).

### Startup (host pattern)
```rust
use anyhow::Result;
use std::sync::Arc;
use ron_auth::{Verifier, VerifierConfig, Decision};
use ron_auth::ctx::RequestCtx;
use ron_auth::keys::MacKeyProvider;

fn init_verifier() -> Arc<Verifier> {
    let cfg = VerifierConfig::from_env_with_defaults(None)  // or load from your config system
        .expect("valid VerifierConfig");
    Arc::new(Verifier::new(cfg))
}

fn verify_once(
    token_b64: &str,
    ctx: &RequestCtx,
    keys: &dyn MacKeyProvider,
    v: &Verifier,
) -> Result<bool> {
    match v.verify(token_b64, ctx, keys, None)? {
        Decision::Allow => Ok(true),
        Decision::Deny { reason } => {
            // increment metrics: ron_auth_deny_total{reason}
            // log JSON with redacted digests; never log tokens/keys
            Ok(false)
        }
    }
}
````

See [CONFIG.md](./CONFIG.md#environment-mapping) for env mapping and [OBSERVABILITY.md](./OBSERVABILITY.md#metrics-taxonomy) for the canonical metrics.

### Shutdown

**N/A** at library level. Host shutdown semantics apply (drain requests, stop, unload).

---

## 3) Health & Readiness

* **Library has no endpoints.** Hosts own:

  * **`/healthz`** = process alive.
  * **`/readyz`** = deps connected (e.g., key provider ready, caches warmed if the host adds any).
* Target: host ready within **2–5s** after start; alert if not **ready by 10s**.

---

## 4) Common Failure Modes

| Symptom / Alert              | Likely Cause                                             | Metric / Log Clue                                     | Resolution                                                                                      | Alert Threshold    |           |                                                                       |        |
| ---------------------------- | -------------------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------ | --------- | --------------------------------------------------------------------- | ------ |
| Spike in `unknown_kid`       | Rotation window misconfigured; KID removed too early     | `ron_auth_unknown_kid_total{tenant,kid}` rising       | Provider map must accept **current + N previous**; republish rotation policy; hot-swap provider | any sustained rise |           |                                                                       |        |
| Many `parse.*` denies        | Malformed/oversized tokens; client bug or abuse          | `ron_auth_parse_error_total{kind="b64                 | cbor                                                                                            | bounds             | schema"}` | Enforce bounds (`max_token_bytes`, `max_caveats`) earlier; fix client | > N/5m |
| Latency p95 regressions      | Larger tokens, more caveats, or wrapper copies           | `ron_auth_duration_us` histogram                      | Check token size/caveats; ensure ≤2 alloc/op; remove extra copies in wrapper                    | p95 > SLO for 10m  |           |                                                                       |        |
| `caveat.custom.unknown`      | Namespace drift; host didn’t register custom caveat      | `ron_auth_deny_total{reason="caveat.custom.unknown"}` | Freeze registrar; align names; unknown custom caveats **deny** by default                       | any                |           |                                                                       |        |
| Denies with `caveat.amnesia` | Host set `Amnesia(true)` but process not in amnesia mode | reason string mentions amnesia                        | Either run host in RAM-only/no persistent logs or mint tokens without that caveat               | any                |           |                                                                       |        |
| Frequent `mac.mismatch`      | Wrong key handle or tenant mix-up                        | `ron_auth_deny_total{reason="mac.mismatch"}`          | Audit KID map by tenant; check mint vs verify binding                                           | sustained rise     |           |                                                                       |        |

> Use **stable deny reason strings** exactly (see [API.md](./API.md#deny-reasons) and [OBSERVABILITY.md](./OBSERVABILITY.md#dashboards)).

---

## 5) Diagnostics

**Metrics (host):**

```bash
curl -s http://HOST:PORT/metrics | \
  grep -E 'ron_auth_(verify|deny|parse_error|unknown_kid|duration_us|token_size_bytes)'
```

**Logs (host guidance):**

* JSON lines: `event=auth.verify`, `result=allow|deny`, `reason` (normalized), `corr_id`, and **redacted** `token_digest8`.
* **Never** log raw tokens or keys. See [SECURITY.md](./SECURITY.md#log-redaction).

**Tracing (optional):**

* Span name `lib.ron_auth.verify` with fields: `tenant`, `kid`, `caveats_count`, `token_size`; events `allow|deny`. See [OBSERVABILITY.md](./OBSERVABILITY.md#tracing).

---

## 6) Recovery Procedures

1. **Key Rotation Drift → `unknown_kid`**

   * **Action:** Update provider map to include **current + N previous**; hot-swap atomically (e.g., `ArcSwap<Provider>`); verify drop in `unknown_kid_total`.

2. **Bounds / Parse Errors**

   * **Action:** Confirm `VerifierConfig` within validated ranges (bytes ∈ [512, 16384]; caveats ∈ [1, 1024]) in [CONFIG.md](./CONFIG.md#validated-ranges). Enforce caps earlier at ingress.

3. **Config Drift (host)**

   * **Action:** Build+validate new `VerifierConfig` off-thread; hot-swap via `Arc`/`ArcSwap`. Rollback if deny storms persist.

4. **Amnesia Mismatch**

   * **Action:** Either operate host in amnesia mode (RAM-only, no persistent logs, aggressive zeroization) or remove the `Amnesia(true)` caveat from minted tokens.

5. **Deny Storm from Custom Caveats**

   * **Action:** Freeze registrar; audit namespace; unknown custom caveats **deny by default**. Update [API.md](./API.md#custom-caveats) table.

---

## 7) Backup / Restore

* **Stateless library.** No state to back up. Backups concern host services only (e.g., their sled DBs).

---

## 8) Upgrades

* Respect SemVer: behavior changes to token encoding, reason strings, or caveat semantics → **MAJOR** + new golden vectors ([IDB.md](./IDB.md#test-vectors)).
* Cross-crate surfaces: `svc-passport` (mint/rotation), `ron-kms` (keys). After upgrade, replay **canonical test vectors** before enabling traffic.

---

## 9) Chaos & Validation Drills

**Quarterly (Gate J):**

* **Hot-swap KID window drill:** Remove previous KID, observe clean `kid.unknown` denies, no panics; restore window.
* **Hostile input drill:** Replay malformed Base64URL/CBOR vectors (oversize, truncated, schema-invalid); confirm `parse.*` reasons and no OOM/panic.
* **Amnesia drill:** Toggle host amnesia; mint/verify tokens with `Amnesia(true)`; ensure policy is enforced and logs remain non-persistent.

**Optional CI example (wire to repo when harness is present):**

```yaml
name: runbook-chaos
on:
  workflow_dispatch: {}
  schedule: [{cron: "0 8 1 */3 *"}]  # quarterly
jobs:
  chaos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test -p ron-auth -- --ignored runbook_chaos_hot_swap_kid
      - run: cargo test -p ron-auth -- --ignored runbook_chaos_hostile_inputs
      - run: cargo test -p ron-auth -- --ignored runbook_chaos_amnesia_mode
```

(Implement the `--ignored` drills as integration tests or a small harness under `testing/`.)

---

## 10) Scaling Notes (when embedded)

* **Bounds are your throttle:** `max_token_bytes` (default 4096) and `max_caveats` (default 64) cap per-op cost. See [CONFIG.md](./CONFIG.md#verifierconfig).
* **Key lookups:** keep `(tenant,kid)→handle` in a fast map; rotate atomically (current + N previous).
* **Parallelism:** scale via host request-level concurrency; `verify()` is re-entrant (`Send + Sync`).
* **Observability:** use the golden metrics set and stable deny reason strings for dashboards/alerts. See [OBSERVABILITY.md](./OBSERVABILITY.md#dashboards).

---

## 11) Security Ops

* **No secret logs**; use redacted token digests for correlation. See [SECURITY.md](./SECURITY.md#log-redaction).
* **Amnesia binding:** only set `amnesia=true` in `RequestCtx` if host runs with RAM-only/no persistent logs (and zeroization policies).
* **Key custody:** keys remain in `ron-kms`; `ron-auth` sees opaque handles and zeroizes any transient material.

---

## 12) References

* [CONFIG.md](./CONFIG.md) — verifier config & bounds; env/validation rules.
* [SECURITY.md](./SECURITY.md) — threat model; boundaries; zeroization.
* [OBSERVABILITY.md](./OBSERVABILITY.md) — metrics taxonomy; alerts; tracing.
* [API.md](./API.md) — traits, DTOs, decision & reasons.
* [IDB.md](./IDB.md) — invariants & canonical vectors.

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Golden metrics wired (verify/deny/parse_error/unknown_kid/duration/token_size).
* [ ] Gate J: Chaos drill: hot-swap KID removal passes (clean denies, no panics); hostile inputs pass.
* [ ] Gate K: Continuous vigilance (alerts link to this runbook; logs redacted).
* [ ] Gate N: Edge/ARM spot check recorded (perf note only; no endpoints).
* [ ] Gate O: Security audit clean (zeroize, no secret logs).

---

## Appendix — Visuals

### A. Hot-Path Sketch

```mermaid
flowchart LR
  A[Base64URL decode] --> B[CBOR parse]
  B --> C[MAC chain (keyed BLAKE3) per caveat]
  C --> D[Evaluate caveats (exp/nbf/path/custom)]
  D --> E{Decision}
  E -->|Allow| G[Return 200/continue]
  E -->|Deny + reason| H[Return 401/403; record metrics]
```

### B. Automation Hooks (optional)

* **Rundeck / GH Actions:** expose “Rotate KID Window” and “Amnesia Drill” as `workflow_dispatch` actions that:

  1. Toggle provider map & run the chaos tests above,
  2. Post a short summary (p95 latency, deny counts) back to the PR or ops channel.

```
```


---

## SECURITY.MD
_File 11 of 12_

````markdown
---
title: Security Notes — ron-auth
crate: ron-auth
owner: Stevan White
last-reviewed: 2025-10-04
status: draft
---

# Security Documentation — ron-auth

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-auth`.  
It complements the repo-wide **Hardening Blueprint**, **Interop Blueprint**, and `docs/IDB.md` / `docs/CONFIG.md` for this crate.

> **Posture:** `ron-auth` is a **pure library** for capability **verification and attenuation** (macaroon-style).  
> **No network/disk I/O.** Key custody lives in `ron-kms`; issuance/rotation lives in `svc-passport`.  
> Verification is **offline**, constant-time where relevant, and **fail-closed**.

---

## 1) Threat Model (STRIDE)

| Category | Threats (attacker goals) | Relevant in `ron-auth`? | Mitigations (this crate) | Residual/Host Mitigations |
|---|---|:---:|---|---|
| **S**poofing | Forge valid tokens; spoof tenant/KID; misuse custom caveats | **Y** | Keyed **BLAKE3** MAC chain; **tenant+KID** binding; constant-time compare; **fail-closed** on unknown KID; caveat registry w/ **deny unknown** | Host must use authenticated transport (TLS) and peer auth (service boundary) |
| **T**ampering | Modify token/caveat order; truncate/extend; equivocation of encodings | **Y** | Deterministic **CBOR**, strict decode; **order-sensitive** chaining; strict size/complexity bounds; Base64URL w/o padding | Host should reject tokens outside policy windows |
| **R**epudiation | No audit trail for allow/deny | **Y** (host-level) | Returns **typed reasons** for deny (non-secret); exposes redaction guidance (`b3(token)[..8]`) | Host logs structured JSON with correlation id; do **not** log tokens/secrets |
| **I**nfo Disclosure | Leak keys/tokens via logs, panics, or debug printing | **Y** | No logging; secrets under **zeroize**; no `Debug`/`Display` for secret bytes; stable `AuthError` w/o secrets | Host must scrub logs, enable amnesia mode if required, control crash dumps |
| **D**oS | Oversized/many caveats; adversarial CBOR; pathological custom caveats | **Y** | Hard caps: **≤4096 B**, **≤64 caveats**; O(n) verification; strict parse; deny unknown custom caveats by default | Host applies rate limits/RPS caps upstream; isolates CPU budgets per request |
| **E**oP | Bypass least privilege; broaden scope; cross-tenant key misuse | **Y** | **Conjunctive** caveats; explicit scope; tenant binding; **amnesia** and **policy digest** caveats; **fail-closed** defaults | Host separates tenants; enforces amnesia mode and governance policy inputs |

**Out of scope for this crate:** transport-level TLS, socket exhaustion, storage tampering—handled by services (gateway/overlay/index/storage) and `ron-kms`.

---

## 2) Security Boundaries

- **Inbound surface (public API):**  
  - `verify(token_b64, ctx, key_provider) -> Decision`  
  - Attenuation/builders for capabilities (no broadening, only narrowing)  
  - Optional **custom caveat registrar** (write-once, then frozen)
- **Outbound dependencies:**  
  - `MacKeyProvider` trait (opaque **key handle** for keyed BLAKE3)  
  - Optional `SigAdapter` (feature `pq-hybrid`) for cross-org signature **envelopes**  
- **Trust zone:** in-process; treat **all token bytes and request context as attacker-controlled** inputs.  
- **Assumptions:**  
  - Keys are provisioned and guarded by `ron-kms` (opaque handles, rotation/KID map).  
  - Hosts enforce transport security (TLS), quotas, and authentication of peers.  
  - If **Amnesia(true)** is required, the host accurately reflects `amnesia = ON` and satisfies its operational guarantees (RAM-only, no persistent logs).  
  - Governance policy digest is provided by host when required by tokens.

---

## 3) Key & Credential Handling

- **Key types (seen indirectly):** symmetric MAC keys (keyed BLAKE3); optional public keys for PQ-hybrid envelopes (Ed25519 + Dilithium2).  
- **Ownership & storage:** keys **never** reside in `ron-auth`; only opaque `MacHandle` is used. Any transient key-derived material is **zeroized** on drop.  
- **Rotation:** tokens bind **KID**; verifiers must accept **current + N previous** and **deny unknown**. Host rotates at ≤30-day cadence (policy), revocation by removing KID from provider map.  
- **Amnesia:** when enabled at host, it implies **RAM-only** key handles and no persistence; `Amnesia(true)` caveat enforces binding.  
- **Zeroization:** all secret buffers use `zeroize` (or are kept behind key handles); `Debug/Display` forbidden for secret types.

---

## 4) Hardening Checklist (crate-specific)

- [ ] **Fail-closed** on any parse/caveat/unknown-kid error  
- [ ] **Deterministic CBOR** + Base64URL (no padding)  
- [ ] **Constant-time** MAC equality (`subtle`)  
- [ ] **Order-sensitive** MAC chain with **domain separation** and per-token nonce  
- [ ] **Strict bounds:** token ≤4096 B; ≤64 caveats; O(n) evaluation  
- [ ] **Tenant binding** + explicit scope + conjunctive caveats (no implicit OR)  
- [ ] **No I/O**; no async in hot path; **no global mutable state**  
- [ ] **Zeroize** secrets; no `Debug/Display` for secret bytes  
- [ ] **Unknown custom caveats → Deny** (default)  
- [ ] **PQ-hybrid** envelope support **feature-gated** (`pq-hybrid`), off by default  
- [ ] **Mint isolation:** `mint-internal` feature is **doc(hidden)** and disallowed in prod builds

*(Service-level items like `/readyz`, sockets, RPS caps are N/A here.)*

---

## 5) Observability for Security

`ron-auth` does not emit logs/metrics itself; it returns **typed reasons** so hosts can instrument:

**Suggested host metrics**
- `auth_verify_total{result="allow|deny", reason}`  
- `auth_parse_error_total{kind}` (`b64`, `cbor`, `bounds`)  
- `auth_unknown_kid_total{tenant,kid}`  
- `auth_caveat_fail_total{name}` (`exp`, `nbf`, `aud`, `tenant`, `amnesia`, `policy_digest`, …)  
- `auth_verify_duration_us` (histogram)

**Logging guidance**
- Structured JSON with `corr_id`, `tenant`, **no raw token**.  
- If correlation is required, log only a **redacted digest** (e.g., first 8 bytes of `b3(token_bytes)`).

---

## 6) Dependencies & Supply Chain

- **Cryptography & safety-critical**:  
  - `blake3` (keyed mode) — MAC primitive  
  - `subtle` — constant-time equality  
  - `zeroize` — secret zeroization  
- **Encoding**:  
  - `serde`, `serde_cbor` (deterministic encoding), `base64` (URL-safe)  
- **Erroring & types**:  
  - `thiserror` for non-leaky errors

**Controls**
- Pinned at workspace root; `cargo-deny` (licenses, bans, advisories, sources) in CI.  
- `#![forbid(unsafe_code)]`; `cargo geiger` zero.  
- SBOM generated at release; stored under `docs/sbom/`.

---

## 7) Formal & Destructive Validation

- **Property tests**:  
  - Re-encoding determinism; bounds enforcement; order sensitivity (reordering caveats invalidates).  
  - **Conjunction** semantics: any failing caveat denies.
- **Fuzzing**:  
  - Base64URL/CBOR token parser; malformed/hostile inputs; ensure no panics/OOM.  
- **Loom (dev-only)**:  
  - Model concurrent `verify()` calls against config snapshot replacement (host-style) to ensure no torn reads.  
- **Chaos (host-oriented)**:  
  - Under load, hot-swap config (e.g., KID removal); expect deny with `unknown_kid` and no panics.

---

## 8) Security Contacts

- **Owners:** Stevan White  
- **Escalation:** Open a private security issue in the repo; do not post secrets/tokens.

---

## 9) Migration & Upgrades

- **Breaking changes** (token encoding, MAC chain domain strings, caveat semantics): **MAJOR** SemVer bump + explicit migration notes + new golden vectors.  
- **Additive caveats**: allowed with safe defaults (**deny** unless explicitly handled).  
- **KID policy**: document rotation window; removing a KID immediately invalidates existing tokens signed with it.

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  A[Caller / Host Service] -->|token_b64 + RequestCtx| B(ron-auth::verify)
  B -->|lookup (tenant,KID) via trait| K[MacKeyProvider]
  K -- opaque handle --> B
  B -->|MAC chain (const-time)<br/>decode+evaluate caveats| D{Decision}
  D -->|Allow (attenuated scope)| OK[Proceed]
  D -->|Deny (typed reason)| NO[Return error + host metrics]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

---

## 11) Residual Risks & Notes

* **Side-channels in parsing:** While MAC equality is constant-time, CBOR decoding work is data-dependent. We mitigate via strict bounds and non-leaky errors; hosts should not expose decode timing to untrusted networks without normalizing responses.
* **Harvest-now-decrypt-later:** Symmetric MACs are not broken by Shor; **signature envelopes** (if used) are hybrid (Ed25519+Dilithium2) behind a feature gate.
* **Custom caveats:** Namespaces widen the attack surface; keep **allowlist minimal** and treat unknowns as **deny** unless a strong reason exists.
* **Amnesia binding:** Security depends on truthful host signaling (`amnesia=true` only when operational guarantees hold).
* **Governance policy digest:** Equality binding only; policy content is out-of-scope for this crate.

---

```
```


---

## TESTS.MD
_File 12 of 12_


````markdown
# 🧪 TESTS.md — `ron-auth`

*Audience: developers, auditors, CI maintainers*  
*MSRV: 1.80.0 (loom-capable toolchain; not required for this crate)*

---

## 0) Purpose

Define the **test contract** for `ron-auth`:

- Unit, integration, property, fuzz, chaos drills (host-embedded), and performance.
- Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
- Invocation commands for devs & CI.
- Golden vectors for **allow/deny** decisions and **stable reason strings**.

> `ron-auth` is a **pure verification library** (no I/O, no background tasks). Most chaos/soak is validated **in hosts** (e.g., `svc-gateway`) that wrap `verify()` and export metrics.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, pure)
**Scope:** token decode/parse helpers, caveat evaluation logic, reason-string normalization, decision shaping.  
**Location:** `src/**`, gated by `#[cfg(test)]`.  
**Run:**
```bash
cargo test -p ron-auth --lib -- --nocapture
````

**Must cover (examples):**

* Base64URL decode errors → `parse.b64` reason.
* CBOR parse errors / missing fields → `parse.cbor` / `schema.*`.
* Bounds enforcement: `max_token_bytes`, `max_caveats` → `parse.bounds`.
* Deterministic reason taxonomy (stable strings).
* Caveat evaluation primitives: `exp/nbf/path`, boolean conjunctions, custom caveat dispatch (unknown → deny).

---

### 1.2 Integration Tests (end-to-end)

**Scope:** full crate surface via `verify(token_b64, RequestCtx, MacKeyProvider, SigAdapter?) → Decision`.
**Location:** `tests/*.rs`.
**Run:**

```bash
cargo test -p ron-auth --test '*'
```

**Mandatory suites:**

* **`allow_deny_vectors.rs`** — replay **golden vectors** (see §3.5) across tenants/KIDs; assert `Decision::Allow` vs `Decision::Deny{reason}` and **exact** reason strings.
* **`config_bounds.rs`** — validate ranges & defaults; oversize tokens/too many caveats → `parse.bounds`.
* **`rotation_window.rs`** — provider exposes **current + N previous** KIDs; removing the previous immediately → `kid.unknown`.
* **`custom_caveats.rs`** — with/without registrar: known custom caveat passes when satisfied; unknown custom caveat **denies by default**.
* **`amnesia_mode.rs`** — tokens containing `Amnesia(true)` must **deny** unless host signals amnesia in `RequestCtx`.
* **`pq_envelope.rs`** *(cfg `pq-hybrid`)* — signature envelope verified via `SigAdapter` (classical vs PQ); ensure decision parity and record perf delta (no hard gate here).

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Run:**

```bash
cargo test -p ron-auth --lib --features proptest -- --nocapture
```

**Invariants & strategies:**

* **No panics** for any input up to `max_token_bytes` and `max_caveats` (fuzzable byte vectors constrained by config).
* **Linear slope**: latency scales ~linearly with caveat count (assert within a tolerance window in debug metrics mode, optional).
* **MAC integrity**: flipping any byte of payload or tag ⇒ `mac.mismatch` (never `Allow`).
* **Order sensitivity**: reordering caveats changes MAC chain ⇒ **deny** (`mac.mismatch` or related); order-preserving replay with same inputs ⇒ **allow**.
* **Schema stability**: adding unknown top-level fields ⇒ `schema.unknown_field` (stable reason).

> Generators produce realistic caveat sets (`exp`, `nbf`, `path`, `tenant`, and random custom namespaces). Shrinkers bias toward minimal counterexamples (1-byte flip, 1 unknown field, etc.).

---

### 1.4 Fuzz Tests

**Tooling:** `cargo-fuzz` with ASan/UBSan where available.
**Targets & goals:**

* `token_parser_fuzz` — arbitrary bytes → verify parse path; **no panics/OOM**; must terminate within time budget.
* `b64_decode_fuzz` — malformed Base64URL cases (padding/URL alphabet).
* `cbor_parse_fuzz` — indefinite-length items, nested maps/arrays, truncated items.
* `custom_caveat_name_fuzz` — untrusted namespace bytes → deny without panic.
* *(optional)* `pq_envelope_fuzz` (cfg `pq-hybrid`) — corrupted envelopes ⇒ deny safely.

**Corpus:** seed with golden negatives (oversize, truncated, invalid schema) and real-world samples (redacted).
**Acceptance:** CI quick pass (≤60s) per PR; **nightly** job 1h (Silver) / 4h (Gold) with **0 crashes**.

**Run:**

```bash
cargo fuzz run token_parser_fuzz -- -max_total_time=60
```

---

### 1.5 Chaos / Soak (host-embedded)

`ron-auth` has no processes/FDs to soak; validate **via a tiny host harness** (under `testing/harness/`) that wraps `verify()` and exposes `/metrics`:

Inject in the harness:

* **Key rotation churn:** hot-swap KID windows; assert clean `kid.unknown` denies (no panics).
* **Hostile input replay:** malformed Base64URL/CBOR; confirm `parse.*` reasons.
* **Amnesia toggling:** enable/disable amnesia; tokens with `Amnesia(true)` obey policy.

**Acceptance:** 30–60 min soak ⇒ zero panics, steady memory, stable deny taxonomy.

**Run (example):**

```bash
cargo test -p ron-auth -- --ignored runbook_chaos_hot_swap_kid
cargo test -p ron-auth -- --ignored runbook_chaos_hostile_inputs
cargo test -p ron-auth -- --ignored runbook_chaos_amnesia_mode
```

---

### 1.6 Performance / Load

Benchmarks live under `benches/` (Criterion). They enforce PERF SLOs and detect regression slope with caveat count.

**Benches:**

* `verify_small_token` (1–3 caveats; cold/warm).
* `verify_many_caveats` (16/32/64; check linear slope).
* `sig_adapter_delta` *(cfg `pq-hybrid`)* — record delta vs classical.

**Run:**

```bash
cargo bench -p ron-auth
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Code coverage **≥ 70%** (line).
* Fuzz harness **builds**; at least one corpus seeded.
* Golden vectors present (allow + deny) with reason parity checks.

### 2.2 Silver (Useful Substrate)

* Property tests in place for parse/MAC invariants.
* Fuzz run ≥ **1h** nightly; **0 crashes**.
* Coverage **≥ 85%**.
* Chaos harness scripted; three drills implemented as `--ignored` tests.

### 2.3 Gold (Ops-Ready)

* Fuzz run ≥ **4h** nightly; **0 crashes**.
* Soak/chaos 60–120 min in CI harness; **no leaks**; deny taxonomy stable.
* Coverage **≥ 90%** (line) and **≥ 85%** (branch) on stable rustc.
* Perf regression tracked release-to-release; ≤10% tolerated unless waived.

---

## 3) Invocation Examples

### 3.1 All tests (crate)

```bash
cargo test -p ron-auth --all-targets -- --nocapture
```

### 3.2 Specific integration suite

```bash
cargo test -p ron-auth --test allow_deny_vectors -- --nocapture
```

### 3.3 Fuzz quick pass (local)

```bash
cargo fuzz run token_parser_fuzz -- -max_total_time=60
```

### 3.4 Benchmarks

```bash
cargo bench -p ron-auth
```

### 3.5 Golden vectors replay

Vectors live under:

```
testing/vectors/ron-auth/v1/
  allow_*.json
  deny_*.json
  pq_*.json           # only when feature pq-hybrid is enabled
```

Each vector MUST include:

```json
{
  "tenant": "acme",
  "kid": "ed25519:2025-09-01",
  "key_hex": "… (for test-only determinism) …",
  "token_b64": "eyJ…",
  "ctx": { "amnesia": false, "path": "/o/foo", "now": 1736037000 },
  "expected": { "decision": "allow" }
}
```

or for denies:

```json
{ "expected": { "decision": "deny", "reason": "parse.cbor" } }
```

**Rules:** reason strings are **stable**; tests must match **exact** text.

---

## 4) Observability Hooks (tests)

* Integration & chaos harness must emit **structured JSON** on failures: include `result`, `reason`, `corr_id`, redacted `token_digest8`.
* If using the host harness, export Prometheus counters/histograms named:

  * `ron_auth_verify_total`
  * `ron_auth_deny_total{reason}`
  * `ron_auth_parse_error_total{kind}`
  * `ron_auth_unknown_kid_total`
  * `ron_auth_duration_us_bucket`
  * `ron_auth_token_size_bytes`

Dashboards should show:

* Deny taxonomy distribution over time.
* Unknown KID spikes during rotation drills.
* p95 duration vs caveat count slope.

---

## 5) CI Enforcement

**Minimum:**

* PR: `cargo test -p ron-auth --all-targets`
* Lints/format: `cargo fmt -- --check && cargo clippy --no-deps -D warnings`
* Advisories: `cargo deny check advisories bans licenses sources`

**Coverage (choose one):**

* `cargo llvm-cov --lcov --output-path lcov.info` (recommended)
* `cargo tarpaulin --out Xml`

**Fuzz (nightly):**

* Quick 60s per target on PR label `fuzz-ok`.
* Nightly 1h (Silver) / 4h (Gold) with artifact upload of new crashing inputs (should be none) and corpus growth.

**Perf guard (optional but recommended):**

* Run Criterion; compare to `testing/performance/baselines/ron-auth/` and fail on >10% regression.

**Chaos harness (quarterly):**

* Run `--ignored` drills from §1.5 on a schedule (Gate J/K).

---

## 6) Open Questions / Per-Crate Decisions

* **Loom usage:** None at lib level (no internal concurrency). Loom reserved for host hot-swap tests (e.g., ArcSwap of providers) in host repos.
* **Mandatory fuzz targets:** `token_parser_fuzz`, `cbor_parse_fuzz`, `b64_decode_fuzz` (**required**); `custom_caveat_name_fuzz` (**recommended**); `pq_envelope_fuzz` (**when feature enabled**).
* **Perf SLOs (cross-ref PERF.md):** p95 ≤ **60 µs + (8 µs × caveats)** at 4 KiB tokens (conservative). Track host-parallel throughput (8-core ≥ **1.0M verifies/s**).

---

## 7) Repository Layout (tests & tooling)

```
crates/ron-auth/
├─ src/
│  └─ … (unit tests inside modules with #[cfg(test)])
├─ tests/
│  ├─ allow_deny_vectors.rs
│  ├─ config_bounds.rs
│  ├─ rotation_window.rs
│  ├─ custom_caveats.rs
│  ├─ amnesia_mode.rs
│  └─ pq_envelope.rs            # cfg(feature = "pq-hybrid")
├─ benches/
│  ├─ verify_small_token.rs
│  ├─ verify_many_caveats.rs
│  └─ sig_adapter_delta.rs      # cfg(feature = "pq-hybrid")
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ token_parser_fuzz.rs
│  │  ├─ cbor_parse_fuzz.rs
│  │  ├─ b64_decode_fuzz.rs
│  │  └─ custom_caveat_name_fuzz.rs
│  └─ Cargo.toml
└─ testing/
   ├─ vectors/ron-auth/v1/*.json
   ├─ performance/baselines/ron-auth/*.json
   └─ harness/ (optional mini-host for chaos/soak)
```

---

## 8) Bronze→Silver→Gold Acceptance Gate Checklist

**Bronze**

* [ ] Unit + integration pass; golden vectors implemented.
* [ ] Coverage ≥ 70% (line); fuzz harness compiles.

**Silver**

* [ ] Property tests for parse/MAC invariants.
* [ ] Nightly fuzz 1h (0 crashes); chaos harness scripted.
* [ ] Coverage ≥ 85% (line).
* [ ] Perf guard wired; baselines stored.

**Gold**

* [ ] Nightly fuzz 4h (0 crashes); soak/chaos 60–120 min clean.
* [ ] Coverage ≥ 90% (line), ≥ 85% (branch).
* [ ] Perf tracked per release; ≤10% regression or approved waiver.
* [ ] Quarterly drills (rotation, hostile inputs, amnesia) logged and linked in RUNBOOK.

---

✅ This testing contract makes `ron-auth` **reproducible, falsifiable, and drift-resistant** across contributors and CI. It aligns with your invariants (bounds, stable reasons, pure-lib ethos) and keeps ops hooks (metrics/chaos) with the host wrappers where they belong.

```
```

