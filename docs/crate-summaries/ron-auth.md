---

crate: ron-auth
path: crates/ron-auth
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Zero-trust message envelopes for internal IPC: sign and verify structured headers/payloads using HMAC-SHA256 with scopes, nonces, and time windows, backed by pluggable key derivation.

## 2) Primary Responsibilities

* Provide a **self-authenticating envelope** type that canonically encodes header+payload and authenticates them with an HMAC tag.
* **Verify** envelopes at every receiving boundary (time window + origin + scopes + tag), and **sign** envelopes at send time using a KMS-backed key derivation.
* Define the **`KeyDeriver`** trait to plug in key management/rotation (via `ron-kms` or other backends).

## 3) Non-Goals

* No key storage, rotation scheduling, or secret management (that lives in **`ron-kms`**).
* No capability/macaroon logic or policy evaluation (lives in **`ron-policy`/service layer**).
* No transport/TLS, no network or DB I/O, no replay database (receivers enforce replay windows if needed).
* Not a JWT/OIDC library; this is **intra-cluster IPC** authn/z, not end-user auth.

## 4) Public API Surface

* Re-exports: none.
* Key types / functions / traits:

  * `enum Plane { Node, App }` — plane tagging for routing and policy.
  * `struct Envelope<H, P>` — generic over header/payload (`H: Serialize, P: Serialize`), fields:

    * `plane`, `origin_svc: &'static str`, `origin_instance: Uuid`, `tenant_id: Option<String>`,
      `scopes: SmallVec<[String; 4]>`, `nonce: [u8;16]`, `iat: i64`, `exp: i64`, `header: H`, `payload: P`, `tag: [u8;32]`.
  * `trait KeyDeriver { fn derive_origin_key(&self, svc: &str, instance: &Uuid, epoch: u64) -> [u8;32]; }`
  * `fn sign_envelope(kd: &dyn KeyDeriver, svc: &str, instance: &Uuid, epoch: u64, env: Envelope<H,P>) -> Envelope<H,P>`
  * `fn verify_envelope(kd: &dyn KeyDeriver, expected_svc: &str, epoch: u64, required_scopes: &[&str], env: &Envelope<H,P>) -> Result<(), VerifyError>`
  * `fn verify_envelope_from_any(kd: &dyn KeyDeriver, allowed_senders: &[&str], epoch: u64, required_scopes: &[&str], env: &Envelope<H,P>) -> Result<(), VerifyError>`
  * `fn generate_nonce() -> [u8;16]`
  * `enum VerifyError { Expired, MissingScope(String), WrongOrigin, BadTag, Crypto }`
* Events / HTTP / CLI: none.

## 5) Dependencies & Coupling

* Internal crates → why / stability / replaceable?

  * **`ron-kms` (indirect)**: typical implementer of `KeyDeriver` (derive per-origin keys; sealing & rotation policy). Stability **loose** (trait-level). Replaceable **yes** (any KMS can implement).
  * **`ron-proto` (recommended, not required)**: carries DTOs used as `header`/`payload`; coupling is **loose** (generic `Serialize`). Replaceable **yes**.
* External crates (top 5) → why / risk:

  * `serde`, `rmp-serde` — canonical messagepack encoding for MAC input; mature, low risk.
  * `hmac`, `sha2` — HMAC-SHA256; mature, constant-time verify.
  * `time` — iat/exp handling; mature.
  * `uuid` — per-instance identity; mature.
  * `smallvec` — efficient `scopes`; low risk.
  * (also `rand` for nonce; `thiserror` for error types).
* Runtime services: none (no network/storage/OS calls; pure CPU + time source).

## 6) Config & Feature Flags

* Env vars / config structs: none in this crate.
* Cargo features: none today; future candidates:

  * `pq` (switch/mix to SHA3/KMAC or PQ signatures if design evolves),
  * `replay-cache` (optional in-mem Bloom/ring buffer helpers, though better as a separate crate).

## 7) Observability

* The crate itself logs nothing (pure functions).
* **Recommended in receivers:** increment `auth_success_total{svc}` / `auth_fail_total{svc,reason}` and log minimal context (never secrets). Expose latency buckets for verification if it becomes hot.

## 8) Concurrency Model

* None — all APIs are synchronous and thread-safe (stateless functions over inputs).
* `KeyDeriver` is `Send + Sync` and may perform internal synchronization/IO in the implementer (but recommended to be in-memory and cheap).

## 9) Persistence & Data Model

* None. No on-disk state or schemas; envelopes are transient.

## 10) Errors & Security

* **Error taxonomy (VerifyError):**

  * `Expired` — iat/exp outside window (retryable only if client clock skew or short expiration; otherwise terminal).
  * `MissingScope(String)` — terminal; caller lacks permission.
  * `WrongOrigin` — terminal; sender spoof/misroute.
  * `BadTag` — terminal; integrity/auth failure (or wrong key/epoch).
  * `Crypto` — terminal; library misuse/internal error.
* **Security properties:**

  * **Integrity & authenticity** via HMAC over a **canonical rmp-serde encoding** of all non-tag fields (plane, origin, tenant, scopes, nonce, iat/exp, header, payload).
  * **Least privilege**: `required_scopes` enforced per endpoint.
  * **Time-box**: iat/exp required; mitigates token reuse over time.
* **Known gaps (to be handled by call sites or future work):**

  * **Replay protection**: no built-in nonce cache; receivers should maintain a sliding window store keyed by `(origin_svc, origin_instance, nonce)` when threat model requires it.
  * **Key lifecycle**: rotation and epoch selection policy live in `ron-kms`/ops; misconfiguration (wrong epoch/key id) yields `BadTag`.
  * **Algorithm agility/PQ**: fixed to HMAC-SHA256 for now; PQ readiness deferred.
  * **Canonicalization caveat**: map-like payloads may reorder fields; prefer struct DTOs from `ron-proto` for deterministic encoding.

## 11) Performance Notes

* Cost per verify/sign is one rmp-serde encode + HMAC-SHA256 over small buffers (usually < 1 KiB).
* Throughput: comfortably tens to hundreds of thousands of verifications/sec on commodity CPUs.
* Allocation: `SmallVec<[String;4]>` keeps common scope sets allocation-free; payload/header encode allocates a `Vec<u8>` for MAC input.
* Hot path guidance: reuse `KeyDeriver` and precompute **epoch** outside tight loops; avoid creating large strings in `scopes`.

## 12) Tests

* Current unit tests (baseline): **sign → verify** happy path; scope and origin enforcement.
* Recommended additions:

  * **Tamper tests**: flip any single field (scope/tenant/header byte) ⇒ `BadTag`.
  * **Clock skew**: +/- skew tolerance tests (may add helper to clamp).
  * **Negative scope**: missing/extra scope fails.
  * **Epoch rotation**: verify fails with previous/next epoch; verify succeeds after re-sign with new epoch.
  * **Property tests**: for arbitrary small payloads ensure `verify(sign(env)) == Ok(())` and that `verify` fails if any byte in the mac’d view is toggled.
  * **Fuzz**: rmp-serde decoder fuzz on `Envelope<Header,Payload>` if you ever support deserializing raw untrusted bytes in this crate (currently not the case).

## 13) Improvement Opportunities

* **Replay window helper**: optional `ReplayGuard` (ring buffer/Bloom) crate with `seen(nonce, origin)` API; integrate at service boundaries.
* **Key hinting**: include `kid`/epoch field explicitly in envelope to enable fast key selection when multiple epochs overlap; document epoch derivation (e.g., days since epoch).
* **Algorithm agility**: version the envelope (`ver: u8`) and abstract MAC algorithm; gate SHA-3/KMAC or BLAKE3-MAC with a feature.
* **Typed scopes**: newtype `Scope(&'static str)` or enum for compile-time safety; keep wire repr as string.
* **Skew handling**: add optional verifier parameter `max_clock_skew_secs` and check `iat <= now + skew && now - skew <= exp`.
* **Zeroization**: explicitly zeroize ephemeral derived keys if you ever store them beyond stack duration (currently derived in KMS impl; here only MAC state).
* **Observability hooks**: optional callback to emit structured auth failure reasons without coupling to a metrics crate.
* **Docs**: example snippets per service (Gateway/Overlay/Omnigate) and threat-model notes.

## 14) Change Log (recent)

* 2025-09-14 — Initial draft: `Envelope`, `KeyDeriver`, HMAC-SHA256 sign/verify, scopes, nonce, iat/exp; added helper `verify_envelope_from_any` and `generate_nonce()`.

## 15) Readiness Score (0–5 each)

* API clarity: **4** (clean surface; add envelope versioning & typed scopes to reach 5)
* Test coverage: **3** (good basics; add tamper/property tests)
* Observability: **2** (none built-in; easy to add hooks)
* Config hygiene: **5** (no env/features; generic trait for KMS)
* Security posture: **4** (strong integrity/auth; replay & algorithm agility deferred)
* Performance confidence: **5** (HMAC + small encodes, very fast)
* Coupling (lower is better): **1** (pure lib; only trait ties to KMS)
