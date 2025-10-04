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
