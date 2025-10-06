

````markdown
# 🔗 INTEROP.md — ron-naming

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---
## 0) Purpose

Define the **interop surface** of `ron-naming`:

* Canonical **DTOs & schemas** for names and governance artifacts.  
* **Normalization** and **address hygiene** rules all crates/SDKs must follow.  
* **Canonical encodings** (JSON/CBOR) and **test vectors**.  
* CLI (`tldctl`) **file/stdio contract** for offline authoring.  

> `ron-naming` is **wire-neutral** (pure library + optional offline CLI). Network protocols (HTTP/OAP/QUIC), TLS, and readiness belong to host services (e.g., `svc-index`) that embed this crate.

---

## 1) Protocols & Endpoints

**Ingress protocols:** N/A (library).  
**Exposed endpoints:** N/A (library).

**Transport invariants (context for hosts):**
- **OAP/1** `max_frame = 1 MiB`.  
- **Storage** streaming chunk ≈ **64 KiB** (storage detail; unrelated to naming).  
- **TLS** type at hosts: `tokio_rustls::rustls::ServerConfig`.

> Host crates using `ron-naming` (e.g., `svc-index`) must enforce these on *their* ingress surfaces.

---

## 2) DTOs / Schemas

### 2.1 Core types (Rust)

```rust
pub struct CanonicalName(String);          // NFC + IDNA processed, lowercase, policy-checked
pub struct Label(String);                  // single normalized label

pub enum NameRef {
  Human   { name: CanonicalName },
  Address { b3: String },                  // "b3:<64-lower-hex>"
}

pub struct TldEntry {
  pub tld: Label,
  pub owner_key_id: String,                // logical key identifier (not a secret)
  pub rules_version: u32,                  // policy bundle / table version
}

pub struct TldMap {
  pub version: u64,                        // monotonic map version
  pub entries: Vec<TldEntry>,
}

pub struct SignedTldMap<Sig> {
  pub body: TldMap,                        // canonicalized body
  pub signatures: Vec<Sig>,                // detached; multi-sig supported
}
````

**Normalization & hygiene (library functions)**

```rust
pub struct NormalizationOptions { /* policy bundle + strict/mixed-script knobs */ }

pub fn normalize_name(
  input: &str,
  opts: &NormalizationOptions
) -> Result<CanonicalName, ValidationError>;

pub fn is_b3_addr(s: &str) -> bool;       // accepts only "b3:<64-lower-hex>"
```

**Canonical encodings**

```rust
pub enum WireFormat { Json, Cbor }

pub fn to_canonical_bytes<T: Serialize>(
  t: &T,
  fmt: WireFormat
) -> Result<Vec<u8>, WireError>;

pub fn from_bytes<T: DeserializeOwned>(
  buf: &[u8],
  fmt: WireFormat
) -> Result<T, WireError>;
```

> All public DTOs are `serde` types and must use `#[serde(deny_unknown_fields)]` in their definitions (strict schema).

**Verifier/Signer (feature `verify`)**

```rust
pub trait Verifier<Sig> {
  fn verify(&self, canonical_body: &[u8], sig: &Sig) -> bool;
}

pub trait Signer<Sig> {
  fn sign(&self, canonical_body: &[u8]) -> Result<Sig, SignError>;
}

pub fn verify_signed_map<V, S>(v: &V, m: &SignedTldMap<S>) -> bool
where
  V: Verifier<S>,
  S: Clone;
```

Backends (Ed25519, PQ, HSM/KMS) plug in behind these traits.

### 2.5 Signature Envelope (Interop Format)

To keep polyglot compatibility and PQ-hybrid clarity, signatures are explicit, detached, and ordered:

```json
{
  "body": { /* TldMap */ },
  "signatures": [
    {
      "alg": "Ed25519",         // or "Dilithium2", "Falcon512"
      "kid": "k:ops-2025",      // logical key id; no secret material present
      "policy": { "required": true },  // optional; host m-of-n policy lives outside
      "sig": "base64url…"       // opaque bytes; canonical over CBOR(body)
    },
    {
      "alg": "Dilithium2",
      "kid": "k:pq-2025",
      "sig": "base64url…"
    }
  ]
}
```

**Verification rule:** `verify_signed_map` operates over **canonical CBOR of `body`**. Hybrid ≠ concat—each signature is independent; host policy decides m-of-n.

---

## 3) CLI (`tldctl`) — Offline Interop Contract

**Subcommands (stable):**

* `lint <in>` — validate & normalize; exit non-zero on error.
* `pack <in> [--out <out>]` — emit canonical bytes (JSON or CBOR).
* `sign <in> [--out <out>]` — produce `SignedTldMap` using external signer.
* `verify <in>` — verify detached signatures.
* `show <in>` — pretty print parsed/normalized view.

**I/O model:**

* Inputs via explicit path or `-` (stdin).
* Outputs default to stdout or `--out <path>`.
* No network or DB; one-shot execution; machine-friendly exit codes.

**Flags (subset most relevant for interop):**

* `--format {json|cbor}`, `--pretty`
* Normalization policy: `--strict`, `--allow-mixed-scripts`,
  `--allowed-scripts ...`, `--policy-bundle <dir>`
* Signing/verify: `--sign-profile <id>`, `--key-id <id>`, `--pq-mode {off|hybrid}`
* Logging: `--log-format json|text`, `--log-level info|debug|trace`

**Config precedence:** `flags > env (TLDCTL_* > RON_NAMING_*) > file > defaults`.

### 3.5 CLI Examples (Offline, Deterministic)

```bash
# Normalize + lint (stdin → stdout), strict policy
tldctl lint - --strict < ./fixtures/name.json

# Pack canonical CBOR and write to file
tldctl pack ./fixtures/tldmap.json --format cbor --out ./out/tldmap.cbor

# Sign with PQ hybrid (detached multi-sig), then verify
tldctl sign ./out/tldmap.cbor --sign-profile ops --pq-mode hybrid --out ./out/signed.cbor
tldctl verify ./out/signed.cbor
```

Exit codes: `0=ok, 2=usage, 3=validation, 4=signature_invalid, 64+=unexpected`.

---

## 4) Canonical Test Vectors

> Store under `tests/vectors/ron-naming/` and mirror in `/docs/api-history/ron-naming/` on releases.

### 4.1 Normalization

* **Input (UTF-8):** `"Exämple.COM"`
* **Policy:** strict, NFC, IDNA UTS-46
* **Output `CanonicalName`:** `"exämple.com"` (lowercase, NFC)
* **JSON (pretty):**

  ```json
  { "name": "exämple.com" }
  ```

### 4.2 Address hygiene

* **Input:** `"b3:4f0c...<64 hex total>..."` → `is_b3_addr == true`
* **Near-miss rejects:** `"B3:..."` (case), `"b3:...63hex"`, `"b3:...65hex"`, `"b2:..."`.

### 4.3 TldMap (JSON canonical)

```json
{
  "version": 7,
  "entries": [
    { "tld": "example", "owner_key_id": "k:ops-2025", "rules_version": 3 }
  ]
}
```

**CBOR canonical (hex dump excerpt):**

```
a2                                      # map(2)
  67                                    # text(7) "version"
  07                                    # 7
  67                                    # text(7) "entries"
  81                                    # array(1)
    a3                                  # map(3)
      63                                # text(3) "tld"
      67 65 78 61 6d 70 6c 65           # "example"
      6b                                # text(11) "owner_key_id"
      6a 6b 3a 6f 70 73 2d 32 30 32 35  # "k:ops-2025"
      6d                                # text(12) "rules_version"
      03
```

### 4.4 SignedTldMap

* **Body canonical bytes:** from `to_canonical_bytes(body, Cbor)`
* **Signatures:** detached; example:

  ```json
  {
    "body": { "...": "TldMap as above ..." },
    "signatures": [
      { "alg": "Ed25519", "kid": "k:ops-2025", "sig": "base64…" },
      { "alg": "Dilithium2", "kid": "k:pq-2025", "sig": "base64…" }
    ]
  }
  ```
* **Verification:** `verify_signed_map` → `true` only if required policy passes (e.g., m-of-n).

### 4.5 Vector Coverage & CI Gates

* **Coverage targets:** at least one vector per class:

  * **ASCII**, **Latin w/ diacritics**, **CJK**, **RTL**, **Emoji-in-name (rejected)**,
    **Mixed-script (rejected)**, **Confusable (rejected)**.
* **CI job:** regenerate vectors → compare canonical bytes → fail on diff unless PR includes:

  1. table/version bump rationale,
  2. updated vectors,
  3. SemVer note (minor/major) if canonical bytes changed.

---

## 5) Error Taxonomy (library/CLI)

**Validation/normalization (library):**
`unknown_field`, `oversize`, `unicode_confusable`, `mixed_script`, `bad_address_format`,
`canonical_mismatch`, `rules_unsupported_version`.

**CLI (`tldctl`) exit codes:**
`0` success; `2` usage error; `3` validation failure; `4` signature invalid; `64+` unexpected/internal.

> Hosts should map library errors to structured HTTP/OAP errors in their own interop specs (e.g., `oversize → 413`).

---

## 6) Bus Topics & Observability Tie-In

`ron-naming` publishes **no** bus events itself.
Hosts SHOULD emit:

* `<svc>.naming.policy_loaded` (Gauge bool) — becomes `naming_policy_loaded` metric.
* `<svc>.naming.parse_error{reason}` (Counter) — mirrors `naming_manifest_parse_errors_total{reason}`.
* `<svc>.naming.selftest_ok` (Gauge bool) — set after boot round-trip.

This aligns events ↔ metrics to make dashboards and alerts trivial.

---

## 7) Interop Guarantees

* **Canonical encodings** are stable across minor versions; any change that alters canonical bytes requires a **major** version.
* **DTO hygiene:** unknown fields are **rejected** on read (strict schema).
* **No network/DB side effects** in either lib or CLI.
* **Verifier/Signer traits** provide a stable boundary for Ed25519 and PQ backends; adding new algorithms behind features is **additive**.
* **Vectors are normative**; release CI re-generates and compares canonical outputs.
* **Amnesia embedding:** When hosts run with amnesia ON, policy tables and vectors are loaded from read-only bundles at boot; no persistent caches are assumed.

---

## 8) SDK Notes (polyglot)

When porting to TS/Python/Swift:

* Mirror DTOs exactly; enforce lowercasing + IDNA/NFC in **one** place.
* Deny unknown fields; match the address regex `^b3:[0-9a-f]{64}$`.
* Use CBOR canonical form with deterministic map ordering for signatures.
* Provide a single `normalizeName(input, options)` entrypoint to avoid divergence.

**TypeScript sketch:**

```ts
export type CanonicalName = string;

export function isB3Addr(s: string): boolean {
  return /^b3:[0-9a-f]{64}$/.test(s);
}

export function normalizeName(input: string, opts: {
  strict?: boolean;
  allowedScripts?: string[];
}): CanonicalName {
  // delegate to platform IDNA/NFC lib; ensure lowercase + policy
  // exact behavior must match Rust vectors (round-trip tests in CI)
  throw new Error("impl in SDK; verify against vectors");
}
```

---

## 9) Interop Gates (CI-Enforceable)

* **API & schema drift:** `cargo public-api --deny-changes` and schema compile tests.
* **Vectors:** `regen-vectors && compare-vectors` must pass; diffs require explicit approval + SemVer note.
* **Determinism:** Two runs of `pack` over the same input (same tables) must byte-match.
* **No-network guard:** `tldctl` executes under egress sandbox; any socket attempt fails CI.

---

## 10) References

* **GMI-1.6 Omni-Gate** (host interop baseline)
* **OBSERVABILITY.md** — metrics/spans a host should emit when calling the library
* **API.md** — surface & SemVer gates
* **SECURITY.md** — confusable/mixed-script policy, signing/verification model

---

## 11) Diagram

```mermaid
flowchart LR
  subgraph Dev/Ops
    A[tldctl (offline)] -->|pack/sign/verify| B{Canonical bytes}
  end
  B --> C[Vectors (JSON/CBOR)]
  C --> D[SDK ports (TS/Py/Swift)]
  D --> E[Hosts (svc-index)]
  E -->|serve OAP/HTTP| Users
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

```

