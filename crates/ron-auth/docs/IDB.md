

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

