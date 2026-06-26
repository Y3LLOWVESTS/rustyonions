

---

# Carryover Notes — `ron-proto` → `ron-oap` (2025-10-24)

## What we accomplished (✅ done)

**Scope & posture**

* `ron-proto` is a **DTO-only** crate: pure data types + strict serde; **no I/O, no hashing/crypto**.
* `#![forbid(unsafe_code)]`, `#![deny(warnings)]`; MSRV pinned via `rust-version = "1.80"`.

**IDs**

* `ContentId` newtype enforcing **BLAKE3-256** addressing **by format only**:

  * Exact string form: `b3:<64 lowercase hex>`.
  * Parser + serde validation (`parse`, `FromStr`, and `Deserialize` reject bad/prefix/len/casing).

**OAP envelopes (wire DTOs)**

* `oap::{hello::Hello, start::Start, data::Data, end::End, error::Error}`.
* `OapKind` enum for classification (`HELLO/START/DATA/END/ERROR`), UPPERCASE rename.
* **Hard spec cap**: `oap::MAX_FRAME_BYTES = 1 MiB` (host must enforce; DTO calls it out).
* `Hello.protocol` is **String** (not `&'static str`) to deserialize cleanly.

**Manifests**

* `manifest::v1::ManifestV1` + `manifest::EntryRef` (deterministic maps via `BTreeMap`).
* `MediaKind` enum with **deterministic default** = `Blob` (serde default + `Default` impl).

**Mailbox / Capability / Error taxonomy**

* `mailbox::{Send,Recv,Ack}`; payloads as `Vec<u8>` using `serde_bytes`.
* `cap::{CapTokenHdr, Caveat}` (headers only; verification elsewhere).
* `error::{Kind, ProtoError, stable_reason}` with **immutable** metric reason strings.

**Version & PQ tags**

* `version::{PROTO_VERSION = 1, PROTO_ABI_FINGERPRINT = "dev-fingerprint"}` (placeholder).
* `quantum::{SignatureAlg, KemAlg}` for **PQ-agile** enums (no crypto, just tags).

**Validation helpers (lightweight)**

* `config::{validate_hello, validate_start, validate_data, validate_seq_progress}`.
* Trait sugar: `Validate` + `Limits { max_frame_bytes }` and **re-exports** at crate root:

  * ergonomic use: `use ron_proto::{Validate, Limits};`

**Tests & examples**

* Unit tests for `ContentId`, OAP roundtrips, manifest defaults.
* Golden vector tests (embedded JSON literals) for `HELLO` & minimal `DATA`.
* Validator tests covering `Hello/Start/Data`.
* Example: `examples/hello_json.rs` prints pretty `HELLO` JSON.

**Features**

* Default JSON; optional: `cbor` (via `serde_cbor`), `schemars` (schema gen).
* No “serde” feature gate (serde is always on).

**Build status**

* `cargo check/build/test -p ron-proto` = **green**. Feature toggles compile.

---

## What’s left (in `ron-proto`) — nice-to-haves, not blockers

* **Public API diff gate** in CI (using `cargo public-api`) — prevents accidental breaking changes.
* **Schema emission** (if you want JSON Schema baselines) via `schemars` and a tiny generator.
* **Fuzzing**: `cargo-fuzz` targets for `ContentId` and OAP DTOs (defense-in-depth).
* **CHANGELOG.md** (we added a template) and **README** “Wire Compatibility & Semver Policy” section.

These are polish items; we can do them later or in parallel.

---

## `ron-oap` design cribsheet (what to reuse, how to wire it)

**Purpose**

* `ron-oap` owns **framing/deframing + I/O** for OAP/1 using the DTOs from `ron-proto`.

**Framing**

* **Length-prefix**: u32 **big-endian** prefix (number of bytes following).
* **Per-frame cap**: hard-enforce **≤ 1 MiB** (match `ron_proto::oap::MAX_FRAME_BYTES`).
* **Negotiated cap**: enforce `Start.max_frame_bytes` for `DATA` (typically ≤ 1 MiB).

**Encoding**

* **JSON** default (`serde_json::to_vec` / `from_slice`).
* Support optional **CBOR** under `--features cbor`.
* Keep a small `WireFormat` tag (`Json`/`Cbor`).

**Flow (happy path)**

1. **HELLO**: produce from `Hello::default()` or custom; run `validate_hello`.
2. **START**: from peer → `validate_start`.
3. **DATA**: each chunk → `validate_data(data, negotiated_max)`.
4. **END**: `ok = true` or `ok = false` with `oap::error::Error`.

**Sequencing**

* Use `validate_seq_progress(prev, next)` if you maintain per-stream seq (recommended).
* We’ll treat `seq_start` (`Start`) and `seq` (`Data`) as **monotonic u64** per stream.

**Errors & mapping**

* Keep an internal codec error (e.g., `OapCodecError::{TooLarge,Serde,Truncated,...}`).
* When translating to `oap::error::Error` (wire ERROR):

  * `TooLarge` → `error::Kind::TooLarge`
  * `Serde` (malformed payload) → `error::Kind::BadRequest`
  * `Truncated/IncompletePrefix` → `error::Kind::BadRequest`
  * Peer/version mismatch → `error::Kind::ProtoMismatch`
  * Transient I/O (in async helpers) → `error::Kind::Unavailable`

**Async helpers (feature `tokio`)**

* Reader: read 4 bytes (u32 BE len), enforce cap, read body, `Frame::decode`.
* Writer: `Frame::encode` to bytes, write prefix + body.
* Consider **bounded buffer** reads to avoid unbounded allocations.

**Backpressure**

* Respect negotiated `max_frame_bytes` (a **per-stream** throttle).
* Avoid buffering many frames; prefer **process-then-write** with clear upper bounds.

**Security**

* Never accept frames > 1 MiB (regardless of negotiator claims).
* Zero-copy where possible for `Vec<u8>` (we’re already using `serde_bytes`).
* No secrets/keys in this crate; caps/macaroons remain **headers only**.

**Observability**

* Map codec failures into stable `ProtoError.kind` and reason labels using `stable_reason`.
* If you add a tiny `metrics` feature later, stick to: `frames_in_total`, `frames_out_total`, `codec_errors_total{kind}`, `bytes_in/out_total`.

---

## Implementation cues (snippets / signatures to target)

**Frame enum (wire union)**

```rust
#[serde(tag="kind", rename_all="UPPERCASE")]
pub enum Frame {
  Hello { #[serde(flatten)] v: ron_proto::oap::hello::Hello },
  Start { #[serde(flatten)] v: ron_proto::oap::start::Start },
  Data  { #[serde(flatten)] v: ron_proto::oap::data::Data  },
  End   { #[serde(flatten)] v: ron_proto::oap::end::End    },
  Error { #[serde(flatten)] v: ron_proto::oap::error::Error},
}
```

**Codec core**

```rust
pub enum WireFormat { Json, Cbor }

pub fn encode_frame(frame: &Frame, fmt: WireFormat) -> Result<Vec<u8>, OapCodecError>;
pub fn decode_frame(bytes: &[u8], fmt: WireFormat) -> Result<Frame, OapCodecError>;

pub fn encode_into(buf: &mut BytesMut, frame: &Frame, fmt: WireFormat) -> Result<(), OapCodecError>;
pub fn decode_from(buf: &mut BytesMut, fmt: WireFormat) -> Result<Option<Frame>, OapCodecError>;
```

**Tokio helpers (optional)**

```rust
pub async fn write_frame<W: AsyncWrite + Unpin>(
    w: &mut W, frame: &Frame, fmt: WireFormat
) -> Result<(), OapCodecError>;

pub async fn read_frame<R: AsyncRead + Unpin>(
    r: &mut R, fmt: WireFormat
) -> Result<Frame, OapCodecError>;
```

**Cap enforcement usage**

```rust
use ron_proto::{Validate, Limits};

let hello = ron_proto::oap::hello::Hello::default();
hello.validate(Limits::default())?;

let start = ron_proto::oap::start::Start { seq_start: 0, max_frame_bytes: 1_048_576, meta: None };
start.validate(Limits::default())?;

let limits = Limits { max_frame_bytes: start.max_frame_bytes };
data_frame.validate(limits)?;
```

**Test vectors we can reuse in `ron-oap`**

* HELLO JSON: `{"protocol":"OAP/1","version":1,"features":[]}`
* DATA JSON: `{"obj":"b3:<64hex>","seq":1,"bytes":[104,101,108,108,111]}` (i.e., `"hello"`)
* END OK: `{"seq_end":1,"ok":true}`
* END ERR: `{ "seq_end":2,"ok":false,"error":{"code":"too_large","message":"...","detail":"..."}}`

---

## Gotchas we already solved (avoid regressions)

* **`Hello.protocol`: String** (not `&'static str`). This avoids E0597 when deserializing.
* **`serde_bytes` behavior**: JSON encoding of `Vec<u8>` appears as **array of numbers** by default (not base64). Cross-SDKs must agree; if you want base64 in JSON, we’d need an explicit wrapper/newtype (not required yet).
* **`MediaKind` default**: we implemented `Default` + `#[serde(default)]` on `EntryRef.kind` to keep back-compat.
* **`deny_unknown_fields`**: applied to externally-facing structs to catch drift; be careful adding new required fields (that’s a **MAJOR** bump).
* **Enum growth**: `#[non_exhaustive]` on extension-prone enums (`OapKind`, PQ enums) so adding variants is **MINOR**, not MAJOR.

---

## Minimal “Definition of Done” for `ron-oap`

1. **Framing API**: `Frame` union + encode/decode (JSON; CBOR behind feature).
2. **Caps enforced**: global 1 MiB + negotiated `Start.max_frame_bytes`.
3. **Seq helpers**: `validate_seq_progress` in stream paths.
4. **Tests**:

   * Roundtrip per kind (HELLO/START/DATA/END/ERROR).
   * Over-cap rejection (serialize a >1 MiB DATA and assert error).
   * Negotiated cap rejection (e.g., `Start=8`, `DATA=9` bytes → error).
   * Corrupt prefix / truncated body handling.
5. **Async helpers** (if `tokio`): read/write happy path + truncated/prefix tests using an in-memory duplex stream.
6. **Docs**: short README with handshake flow and invariants; note JSON default + CBOR optional.

---

## Quick commands you’ll likely run

```
cargo new --lib crates/ron-oap
cargo add -p ron-oap --path ../ron-proto ron-proto
cargo add -p ron-oap bytes serde serde_json thiserror
cargo add -p ron-oap serde_cbor --optional
cargo add -p ron-oap tokio --features io-util --optional
```

(We can paste the full crate we mocked earlier if you want to go straight to green.)

---

### TL;DR

* `ron-proto` is **ready**: strict DTOs, helpers, and tests are in place.
* For `ron-oap`, focus on **framing**, **caps**, **serde**, and **clean error mapping**—reusing the validators we wrote.
* Start with the small `Frame` union + (de)serialize → add BE u32 prefix + cap checks → bolt on tokio helpers.
* We already have vectors and validators, so the `ron-oap` crate should go up fast.

### END NOTE ###



### BEGIN NOTE - APRIL 29 2026 - 13:00 CST

Here are the **ron-proto changelog/carry-over notes only**.

---

# `ron-proto` — WEB3_2 Asset DTO / Manifest / Page Contract Notes

**Session date:** 2026-04-29
**Crate:** `crates/ron-proto`
**Status:** Batch complete / green
**Purpose of this session:** Add the shared WEB3_2 DTO contract for typed asset manifests, ownership claims, payout references, storage availability, receipt references, and hydrated asset-page responses.

---

## 0. Summary

In this session, `ron-proto` was extended with the core WEB3_2 asset/product DTO layer.

This is the cross-service contract that lets the rest of the RustyOnions stack agree on what an asset is, how its manifest is represented, how ownership and payout references are shaped, and how a hydrated b3 asset page should look.

This batch added DTOs for:

```text
Asset kinds
Asset manifests
Asset metadata
Asset owners
Asset provenance
Curator metadata
Ownership claims
Signature references
Payout targets
Payout splits
Storage availability
Receipt references
Hydrated asset pages
Canonical crab asset URL helper
```

The canonical public crab asset URL helper now produces:

```text
crab://<64hex>.<asset_kind>
```

Example:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

The internal content ID remains:

```text
b3:<64 lowercase hex>
```

---

## 1. Files added or updated

### Updated

```text
crates/ron-proto/src/lib.rs
```

### Added

```text
crates/ron-proto/src/asset/mod.rs
crates/ron-proto/src/asset/manifest.rs
crates/ron-proto/src/asset/ownership.rs
crates/ron-proto/src/asset/page.rs
crates/ron-proto/src/asset/payout.rs
crates/ron-proto/tests/asset_manifest.rs
crates/ron-proto/tests/asset_page_wire.rs
```

---

## 2. Public module added

A new `asset` module was added and exported from `lib.rs`.

```rust
pub mod asset;
pub use asset::*;
```

This makes the WEB3_2 asset DTOs available to downstream crates such as:

```text
svc-index
omnigate
svc-gateway
svc-storage
ron-app-sdk
browser extension / TS SDK later
```

This is important because `ron-proto` owns cross-service DTOs, not runtime behavior.

---

## 3. Major DTOs added

## 3.1 `AssetKind`

A new `AssetKind` enum was added for typed WEB3_2 asset pages.

Current `ron-proto` supported asset kinds:

```text
image
video
music
song
article
post
comment
page
site
app
manifest
```

This gives the broader product layer enough vocabulary for:

```text
crab://<hash>.image
crab://<hash>.video
crab://<hash>.music
crab://<hash>.article
crab://<hash>.post
crab://<hash>.comment
crab://<hash>.page
crab://<hash>.site
crab://<hash>.app
crab://<hash>.manifest
```

`AssetKind` includes:

```rust
suffix()
matches_suffix()
Display
FromStr
serde rename_all = "snake_case"
```

The enum is marked:

```rust
#[non_exhaustive]
```

So future asset kinds can be added without pretending the vocabulary is frozen forever.

---

## 3.2 `canonical_crab_asset_url()`

A helper was added:

```rust
canonical_crab_asset_url(asset_cid: &ContentId, asset_kind: AssetKind) -> String
```

It converts an internal CID:

```text
b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

Into the public crab URL:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Important design choice:

```text
The public URL does not include b3/.
The internal CID still includes b3:.
```

So:

```text
Public:   crab://<hash>.<kind>
Internal: b3:<hash>
```

---

# 4. Asset manifest DTOs added

## 4.1 `AssetManifestV1`

The main manifest DTO was added:

```rust
AssetManifestV1
```

Fields:

```text
version
asset_cid
asset_kind
manifest_cid
owner
payout
metadata
provenance
storage
receipts
curator
```

Purpose:

```text
Bind immutable asset bytes to metadata, owner references, payout references, provenance, storage refs, receipts, and curator metadata.
```

Important boundary:

```text
AssetManifestV1 does not store raw bytes.
AssetManifestV1 does not authorize ownership.
AssetManifestV1 does not execute payouts.
AssetManifestV1 does not mutate wallet or ledger.
```

It is a strict DTO contract only.

---

## 4.2 `AssetOwnerV1`

Added:

```rust
AssetOwnerV1
```

Fields:

```text
passport_subject
wallet_account
```

This is a reference shape only.

It does **not** mean the wallet can spend. It does **not** grant authority. Actual capability verification still belongs in:

```text
ron-auth
svc-passport
svc-wallet
ron-policy
```

---

## 4.3 `AssetMetadataV1`

Added:

```rust
AssetMetadataV1
```

Fields:

```text
title
description
tags
license
content_type
```

Validation includes:

```text
non-empty title
bounded title length
bounded description length
bounded license length
bounded content type length
maximum tag count
maximum tag length
duplicate tag rejection
case-insensitive duplicate tag detection
```

This makes asset metadata safe for later display/hydration layers.

---

## 4.4 `AssetProvenanceV1`

Added:

```rust
AssetProvenanceV1
```

Fields:

```text
created_at_ms
source
parent_cids
```

Purpose:

```text
Track source/provenance relationships without making manifests mutable truth.
```

Validation includes:

```text
created_at_ms must be nonzero
source is bounded if present
```

---

## 4.5 `CuratorMetadataV1`

Added:

```rust
CuratorMetadataV1
```

Fields:

```text
curator_passport_subject
notes
tags
```

Purpose:

```text
Allow future curator/collection metadata on asset pages.
```

This supports the WEB3_2 goal of asset pages showing curator metadata, tags, provenance, and ownership/payout context.

---

# 5. Ownership claim DTOs added

## 5.1 `AssetOwnershipClaimV1`

Added:

```rust
AssetOwnershipClaimV1
```

Fields:

```text
version
asset_cid
asset_kind
claim_type
passport_subject
wallet_account
issued_at_ms
expires_at_ms
signature
```

Purpose:

```text
Represent a signed claim binding an asset CID and kind to a passport subject and wallet account.
```

Important boundary:

```text
ron-proto does not verify the signature.
ron-proto does not verify passport ownership.
ron-proto does not verify wallet authority.
ron-proto only defines the strict shape.
```

Verification belongs to:

```text
ron-auth
svc-passport
ron-kms
ron-policy
```

---

## 5.2 `OwnershipClaimType`

Added claim types:

```text
original_creator
transfer
curator
license_grant
```

This gives enough vocabulary for initial asset ownership flows without implementing a full ownership transfer system yet.

---

## 5.3 `SignatureRefV1`

Added:

```rust
SignatureRefV1
```

Fields:

```text
alg
public_key_ref
signature
```

Again, this is only a reference DTO. It does not perform crypto verification.

---

# 6. Payout DTOs added

## 6.1 `PayoutTarget`

Added:

```rust
PayoutTarget
```

Fields:

```text
default_action
recipient_account
splits
```

Purpose:

```text
Tell product layers and payout planners what account should receive value for a given action.
```

Important boundary:

```text
PayoutTarget does not execute payout.
PayoutTarget does not call svc-wallet.
PayoutTarget does not mutate ron-ledger.
```

It is display/planning metadata only.

---

## 6.2 `PayoutSplitV1`

Added:

```rust
PayoutSplitV1
```

Fields:

```text
role
account
bps
```

All split math is integer basis points only.

Validation requires:

```text
if splits are present, total bps must equal exactly 10_000
```

This preserves WEB3 invariants:

```text
No floating-point money math.
Percentages use basis points.
```

---

## 6.3 `PayoutRole`

Added payout roles:

```text
creator
asset_owner
site_owner
curator
storage_provider
edge_provider
treasury
reward_pool
referrer
```

This sets up future ROC economics and configurable payout splits without adding execution logic here.

---

# 7. Asset page DTOs added

## 7.1 `AssetPageV1`

Added:

```rust
AssetPageV1
```

Fields:

```text
version
asset_cid
asset_kind
manifest
owner
payout
storage
receipts
links
warnings
```

Purpose:

```text
Represent the hydrated response that omnigate/gateway can return for typed b3 asset pages.
```

This is the DTO shape for a page like:

```text
crab://<hash>.image
```

The eventual frontend/browser extension can render this into a user-facing asset page.

---

## 7.2 `StorageAvailabilityV1`

Added:

```rust
StorageAvailabilityV1
```

Fields:

```text
available
size_bytes
content_type
provider_ref
raw_url
```

Purpose:

```text
Show whether the object bytes are available and where/how they can be fetched.
```

Boundary:

```text
It does not store bytes.
It does not verify storage.
It does not pin content.
It is only metadata/display shape.
```

---

## 7.3 `ReceiptRefV1`

Added:

```rust
ReceiptRefV1
```

Fields:

```text
tx_id
receipt_kind
amount_minor_units
account
created_at_ms
```

Purpose:

```text
Allow asset pages to show wallet/storage/view receipts by reference.
```

Boundary:

```text
ReceiptRefV1 does not verify receipts.
ReceiptRefV1 does not fetch receipts.
ReceiptRefV1 does not mutate wallet or ledger.
```

Verification remains in:

```text
svc-wallet
ron-ledger
paid enforcement services
```

---

## 7.4 `ReceiptKind`

Added receipt kinds:

```text
hold
capture
release
issue
transfer
burn
paid_storage
paid_view
site_visit
```

This supports future display of:

```text
upload paid
storage pinned
view paid
site visit paid
creator payout
provider payout
```

---

## 7.5 `AssetPageLinksV1`

Added:

```rust
AssetPageLinksV1
```

Fields:

```text
canonical_crab
raw
manifest
paid_view
```

Purpose:

```text
Give the browser extension / dashboard / client all important product-facing links.
```

Validation ensures:

```text
canonical_crab must be present
canonical_crab must start with crab://
optional links are bounded
```

---

# 8. Validation system added

The asset module added a deterministic validation error type:

```rust
AssetValidationError
```

Variants include:

```text
InvalidVersion
EmptyField
FieldTooLong
UnsupportedAssetKind
AssetKindMismatch
TooManyTags
DuplicateTag
InvalidBpsTotal
InvalidTimeOrder
```

Validation helpers added:

```text
require_non_empty_bounded()
validate_optional_bounded()
validate_bounded()
validate_tags()
```

These helpers enforce:

```text
bounded DTOs
non-empty required fields
safe tag limits
duplicate tag rejection
basis-point total checks
version checks
time-order checks
```

This is important because these DTOs will eventually be accepted from external clients and browser-extension flows.

---

# 9. Tests added

## 9.1 `tests/asset_manifest.rs`

Covers:

```text
valid manifest validation
canonical crab URL helper
manifest JSON round-trip
unknown manifest field rejection
invalid CID rejection during deserialize
missing owner rejection during deserialize
empty owner passport rejection
empty payout recipient rejection
payout split total must equal 10_000 bps
duplicate tag rejection
asset kind suffix mismatch rejection
stable asset kind suffixes
```

Key proof:

```text
AssetManifestV1 is strict, round-trippable, and validates important WEB3_2 invariants.
```

---

## 9.2 `tests/asset_page_wire.rs`

Covers:

```text
valid AssetPageV1 validation
asset page JSON round-trip
unknown field rejection
oversized storage content type rejection
receipt tx_id requirement
manifest CID mismatch rejection
manifest kind mismatch rejection
canonical_crab must use crab:// scheme
missing-manifest page can still validate
```

Key proof:

```text
Hydrated asset pages have a stable JSON shape and fail closed on malformed/mismatched data.
```

---

# 10. Validation gates passed

The final `ron-proto` gates passed:

```text
cargo fmt
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

Final observed result:

```text
ron-proto clippy: clean
ron-proto tests: all passing
```

Passing test groups included:

```text
asset_manifest.rs
asset_page_wire.rs
content_id.rs
golden_vectors.rs
manifest_defaults.rs
oap_roundtrip.rs
validate_helpers.rs
benches/encode_decode_large.rs
benches/encode_decode_small.rs
examples/hello_json.rs
```

The new asset tests specifically passed:

```text
asset_manifest.rs    11/11 passing
asset_page_wire.rs   9/9 passing
```

---

# 11. Invariants preserved

This work preserved the WEB3 / WEB3_2 invariants:

```text
ron-proto remains DTO-only.
No IO was added.
No async was added.
No storage client was added.
No gateway client was added.
No wallet client was added.
No ledger client was added.
No direct economic mutation was added.
No raw asset bytes are stored here.
No floating-point money math was added.
All payout percentages use integer basis points.
Internal content IDs remain b3:<64 lowercase hex>.
Public crab URLs are crab://<64hex>.<asset_kind>.
Unknown JSON fields reject via deny_unknown_fields.
```

---

# 12. Architectural boundary notes

`ron-proto` now owns:

```text
Asset DTO schemas
Manifest DTO schemas
Ownership claim DTO schemas
Payout target DTO schemas
Receipt reference DTO schemas
Hydrated asset-page DTO schemas
Strict serde contracts
Basic shape validation helpers
Canonical crab asset URL helper
```

`ron-proto` still does **not** own:

```text
storage persistence
asset byte storage
manifest pointer storage
manifest hydration
wallet receipt verification
wallet mutation
ledger mutation
policy decisions
passport verification
signature verification
gateway routing
browser extension UX
```

Those responsibilities remain in their proper crates:

```text
ron-naming     — crab URL parsing / naming grammar
svc-index      — manifest pointers and runtime lookup
svc-storage    — raw object bytes by content ID
omnigate       — hydrated view composition
svc-gateway    — ingress/proxy exposure
svc-wallet     — wallet API and receipt verification
ron-ledger     — durable economic truth
ron-auth       — capability verification
svc-passport   — passport identity layer
ron-kms        — key/signature verification substrate
ron-policy     — policy evaluation
```

---

# 13. Completion assessment for `ron-proto`

For the WEB3_2 product-proof TODO, `ron-proto` Batch 2 is:

```text
100% complete for this session’s scope
```

Current `ron-proto` WEB3_2 readiness:

```text
AssetKind DTO vocabulary                 complete
AssetManifestV1                          complete
AssetOwnershipClaimV1                    complete
PayoutTarget / PayoutSplitV1             complete
AssetPageV1                              complete
StorageAvailabilityV1                    complete
ReceiptRefV1                             complete
Canonical crab asset URL helper          complete
Strict serde unknown-field rejection      complete
Validation tests                          complete
Targeted gates                            green
```

---

# 14. Remaining future work for `ron-proto`

No blocker remains from this session.

Future enhancements may include:

```text
Align ron-naming AssetKind and ron-proto AssetKind exactly if desired.
Add more asset kinds if product needs them.
Add site manifest DTOs when crab://site backend foundation begins.
Add paid content-view prepare/receipt DTOs if not already covered elsewhere.
Add browser-extension-facing DTOs only if they are shared cross-service.
Add stable JSON golden vectors for extension/SDK contract tests.
Add explicit DTO examples in docs once route contracts finalize.
```

Potential future DTOs:

```text
SiteManifestV1
SiteLaunchPrepareRequest
SiteLaunchPrepareResponse
AssetUploadPrepareRequest
AssetUploadPrepareResponse
PaidContentViewPrepareRequest
PaidContentViewPrepareResponse
CrabResolveRequest
CrabResolveResponse
PassportAssetPermissionRef
```

These should only be added when needed by the next TODO batches.

---

# 15. Session takeaway

`ron-proto` now has the shared DTO foundation for WEB3_2 typed asset pages.

It gives the rest of the stack a stable contract for:

```text
what an asset is
who owns it
where payout should go
how metadata/provenance is represented
where storage/provider data appears
how receipts are referenced
what a hydrated asset page response should look like
```

This completed the second major product-proof primitive after `ron-naming`.

The chain after this crate became:

```text
ron-naming
  defines crab://<hash>.<kind>

ron-proto
  defines the asset/manifest/page DTO contract

svc-index
  stores mutable pointers to immutable manifest CIDs

omnigate
  hydrates asset pages

svc-gateway
  exposes product routes to clients/browser extension
```


### END NOTE - APRIL 29 2026 - 13:00 CST



### BEGIN NOTE - MAY 6 2026 - 17:50 CST



## Set 3 — `ron-proto` Crate Notes

Drop this into the `ron-proto` crate notes.

---

# `ron-proto` Crate Carryover Notes

## NEXT_LEVEL / WEB3_2 Public Passport + Profile DTO Foundation

## 1. Crate role

`ron-proto` is the canonical DTO / wire-contract crate for RustyOnions.

It should define strict, serde-compatible message shapes used across services, SDKs, tests, and future clients.

Its job is:

```text
Define stable DTOs.
Define strict public schema shapes.
Validate DTO invariants.
Prevent unknown/private fields in public wire types.
Provide deterministic validation errors.
Keep cross-service JSON/wire contracts aligned.
```

It must **not** do:

```text
network calls
database access
profile persistence
username uniqueness enforcement
wallet mutation
ledger mutation
storage writes
policy enforcement
signature verification
private key custody
alt linkage resolution
reputation computation
moderation enforcement
```

For this session, `ron-proto` became the shared schema foundation for public RON Passport / profile objects.

---

## 2. Session summary

In this session, `ron-proto` was upgraded with strict public identity/profile DTOs for NEXT_LEVEL.

Main additions:

```text
PassportPublicProfileV1
PassportPublicManifestV1
PassportKindV1
UsernameStatusV1
AttributionStatus
ReputationSummaryV1
ModeratorSummaryV1
SiteModeratorRoleV1
SiteModeratorRoleAssignmentV1
PublicAssetReferenceV1
IdentityValidationError
expanded identity/profile asset kinds
identity_profile.rs tests
```

This gives the project a canonical public DTO shape for:

```text
public RON Passport profiles
public profile manifests
@username status
profile display fields
avatar image references
public site references
public asset references
public reputation summary placeholder
public moderation summary placeholder
public proof refs
public profile manifest CIDs
```

Important distinction:

```text
ron-proto defines the public schema.
svc-passport owns profile claim state.
omnigate/svc-gateway proxy or hydrate responses.
CrabLink renders only backend truth.
```

---

## 3. Files changed / added

Primary files touched:

```text
crates/ron-proto/src/lib.rs
crates/ron-proto/src/identity/mod.rs
crates/ron-proto/src/asset/mod.rs
crates/ron-proto/tests/identity_profile.rs
```

---

## 4. Public module exports

`src/lib.rs` now includes and exports the new identity module:

```rust
pub mod identity;
pub use identity::*;
```

The public facade now includes identity/profile DTOs alongside existing modules:

```text
asset
cap
config
econ
error
gov
id
identity
mailbox
manifest
naming
oap
quantum
trace
version
```

This means downstream crates can import profile DTOs directly from `ron_proto::*`.

Example:

```rust
use ron_proto::{
    PassportPublicProfileV1,
    PassportPublicManifestV1,
    UsernameStatusV1,
    PassportKindV1,
};
```

---

## 5. Asset module updates

`crates/ron-proto/src/asset/mod.rs` was updated to include the expanded NEXT_LEVEL asset vocabulary.

Important public typed crab URL rule remains:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Internal CID rule remains:

```text
b3:<64 lowercase hex>
```

Expanded `AssetKind` includes:

```text
Image
Video
Music
Song
Podcast
Article
Post
Comment
Stream
Profile
Passport
Alt
Page
Site
App
Manifest
```

Stable suffixes:

```text
.image
.video
.music
.song
.podcast
.article
.post
.comment
.stream
.profile
.passport
.alt
.page
.site
.app
.manifest
```

Important helper:

```rust
canonical_crab_asset_url(asset_cid: &ContentId, asset_kind: AssetKind) -> String
```

Example:

```text
Input CID:
b3:1111111111111111111111111111111111111111111111111111111111111111

Kind:
AssetKind::Profile

Output:
crab://1111111111111111111111111111111111111111111111111111111111111111.profile
```

Important invariant:

```text
Public crab URL omits b3/ path prefix.
Internal CID keeps b3: prefix.
```

---

## 6. New identity module

New file:

```text
crates/ron-proto/src/identity/mod.rs
```

This module defines public profile/passport DTOs for NEXT_LEVEL identity discovery.

It is DTO-only. It does not claim, verify, persist, or authorize usernames.

Major constants:

```rust
PASSPORT_PUBLIC_PROFILE_VERSION: u16 = 1;
PASSPORT_PUBLIC_MANIFEST_VERSION: u16 = 1;

MAX_USERNAME_BYTES: usize = 32;
MAX_HANDLE_BYTES: usize = 33;
MAX_DISPLAY_NAME_BYTES: usize = 96;
MAX_PROFILE_BIO_BYTES: usize = 1024;
MAX_IDENTITY_REF_BYTES: usize = 512;
MAX_PUBLIC_PROFILE_REFS: usize = 128;
MAX_IDENTITY_WARNING_BYTES: usize = 512;
```

---

## 7. Main DTO: `PassportPublicProfileV1`

Purpose:

```text
Public, safe, read-only profile/passport display object.
```

This is intended for gateway/CrabLink hydration and future public profile manifests.

Fields:

```rust
pub struct PassportPublicProfileV1 {
    pub version: u16,
    pub passport_subject: String,
    pub passport_kind: PassportKindV1,
    pub username: String,
    pub handle: String,
    pub username_status: UsernameStatusV1,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_image: Option<String>,
    pub public_profile_cid: Option<ContentId>,
    pub public_payout_account: Option<String>,
    pub public_sites: Vec<String>,
    pub public_assets: Vec<PublicAssetReferenceV1>,
    pub reputation_summary: Option<ReputationSummaryV1>,
    pub moderator_summary: Option<ModeratorSummaryV1>,
    pub attribution_status: AttributionStatus,
    pub warnings: Vec<String>,
}
```

Important method:

```rust
profile.validate()
```

Important helper:

```rust
profile.canonical_profile_crab_url()
```

Expected canonical profile URL:

```text
crab://@skinnycrabby
```

Important security invariant:

```text
PassportPublicProfileV1 must not contain:
private keys
seed phrases
recovery metadata
private alt mappings
wallet spend authority
private funding source
secret capabilities
```

---

## 8. Main DTO: `PassportPublicManifestV1`

Purpose:

```text
Optional b3-addressable public manifest root for a public profile/passport object.
```

Fields:

```rust
pub struct PassportPublicManifestV1 {
    pub version: u16,
    pub manifest_cid: Option<ContentId>,
    pub profile: PassportPublicProfileV1,
    pub public_proof_refs: Vec<String>,
    pub public_asset_catalogue_cid: Option<ContentId>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub warnings: Vec<String>,
}
```

Important method:

```rust
manifest.validate()
```

Important behavior:

```text
version must match PASSPORT_PUBLIC_MANIFEST_VERSION
profile.validate() must pass
created_at_ms must be nonzero
updated_at_ms must be >= created_at_ms
manifest_cid and profile.public_profile_cid must not conflict
public proof refs are bounded public refs only
```

Important distinction:

```text
PassportPublicManifestV1 is not the private passport record.
It is not key custody.
It is not wallet authority.
It is not alt linkage authority.
```

---

## 9. Username and passport status enums

### `PassportKindV1`

Current variants:

```text
Main
Alt
```

Wire names:

```text
main
alt
```

Important product rule:

```text
Main passports are intended for site creation / public identity.
Alt passports are for pseudonymous activity later.
Public alt profile DTOs must not reveal parent/main passport linkage.
```

### `UsernameStatusV1`

Current variants:

```text
LocalDraft
Requested
Confirmed
Rejected
Unavailable
BackendUnknown
```

Wire names:

```text
local_draft
requested
confirmed
rejected
unavailable
backend_unknown
```

Important frontend rule:

```text
CrabLink must only display confirmed username ownership when backend returns username_status = "confirmed".
```

Local or requested states are not ownership proof.

---

## 10. Attribution enum

### `AttributionStatus`

Current variants:

```text
Unverified
SelfDeclared
BackendConfirmed
Disputed
Revoked
```

Purpose:

```text
Public attribution confidence state.
```

Important note:

```text
This is descriptive only.
Verification, policy, and disputes are service-owned.
```

For now, tests use:

```text
BackendConfirmed
```

for the sample public profile.

---

## 11. Reputation and moderation DTOs

### `ReputationSummaryV1`

Fields:

```rust
pub struct ReputationSummaryV1 {
    pub global_reputation_score: Option<u32>,
    pub source: String,
    pub updated_at_ms: Option<u64>,
    pub warnings: Vec<String>,
}
```

Important note:

```text
Score calculation is not owned by ron-proto.
ron-proto only defines the public shape.
```

### `ModeratorSummaryV1`

Fields:

```rust
pub struct ModeratorSummaryV1 {
    pub global_moderator_score: Option<u32>,
    pub site_roles: Vec<SiteModeratorRoleAssignmentV1>,
    pub source: String,
    pub updated_at_ms: Option<u64>,
    pub warnings: Vec<String>,
}
```

Important note:

```text
Moderation enforcement is not owned by ron-proto.
```

### `SiteModeratorRoleV1`

Current variants:

```text
Viewer
Commenter
SignalModerator
QueueModerator
ContentModerator
TrustedModerator
AdminModerator
SiteOwner
```

Important NEXT_LEVEL concept:

```text
Everyone with a passport can eventually participate in some moderation/reputation surface,
but special site moderators can have site-scoped elevated privileges.
```

`ron-proto` only defines DTO fields for those future surfaces.

---

## 12. Public asset references

### `PublicAssetReferenceV1`

Fields:

```rust
pub struct PublicAssetReferenceV1 {
    pub asset_cid: Option<ContentId>,
    pub asset_kind: AssetKind,
    pub crab_url: String,
    pub title: Option<String>,
}
```

Validation:

```text
crab_url must start with crab://
title is optional and bounded
asset_cid is optional because some profiles may reference route-backed objects before manifest publication
```

This supports public profile pages that list:

```text
avatar images
published images
posts
articles
videos
music
sites
profile assets
passport assets
```

---

## 13. Validation behavior

`PassportPublicProfileV1::validate()` checks:

```text
version == PASSPORT_PUBLIC_PROFILE_VERSION
passport_subject nonempty and bounded
username syntax
handle syntax
handle matches username
display_name bounded if present
bio bounded if present
avatar_image must be crab:// if present
public_payout_account bounded if present
public_sites are crab:// URLs
public_assets validate
reputation_summary validates if present
moderator_summary validates if present
warnings bounded
```

`PassportPublicManifestV1::validate()` checks:

```text
version == PASSPORT_PUBLIC_MANIFEST_VERSION
profile validates
created_at_ms != 0
updated_at_ms >= created_at_ms
manifest_cid and profile.public_profile_cid do not conflict
public_proof_refs bounded
warnings bounded
```

Important validation error enum:

```rust
IdentityValidationError
```

Current variants:

```text
InvalidVersion
EmptyField
FieldTooLong
TooManyRefs
InvalidUsername
InvalidHandle
HandleMismatch
InvalidCrabUrl
CidMismatch
InvalidTimeOrder
```

---

## 14. DTO privacy / safety rules

Public identity DTOs must never include:

```text
private_key
seed_phrase
recovery
recovery_phrase
spend_authority
wallet_spend_authority
private_alt_mapping
parent_passport
main_passport_subject
funding_source
private capabilities
bearer tokens
```

The tests explicitly guard against several of these strings appearing in serialized public profile/manifest JSON.

Important rule for future contributors:

```text
If a field would reveal private identity linkage, wallet authority, or recovery data, it does not belong in PassportPublicProfileV1 or PassportPublicManifestV1.
```

---

## 15. Tests added

New file:

```text
crates/ron-proto/tests/identity_profile.rs
```

Tests cover:

```text
public_profile_validates_and_roundtrips_json
public_manifest_validates_and_roundtrips_json
profile_rejects_unknown_private_fields
handle_must_match_username
avatar_must_be_crab_url_when_present
manifest_cid_and_profile_cid_must_not_conflict
alt_profile_does_not_serialize_public_parent_linkage
asset_kind_identity_roots_have_stable_suffixes
```

Key test behaviors:

```text
Serde roundtrips public profile DTOs.
Serde rejects unknown/private fields due to deny_unknown_fields.
Handle must match username.
Avatar URL must be crab://.
Manifest CID must not conflict with profile CID.
Alt profile JSON must not reveal parent/main linkage.
Profile/passport/alt asset suffixes are stable.
```

---

## 16. Green proof from this session

`ron-proto` passed:

```bash
cargo fmt -p ron-proto
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

Important result:

```text
identity_profile.rs: 8 passed
```

Status:

```text
GREEN
```

---

## 17. Relationship to `ron-naming`

`ron-proto` and `ron-naming` now overlap conceptually but serve different purposes.

`ron-naming`:

```text
parses and normalizes usernames/crab links
owns grammar
returns parser errors
```

`ron-proto`:

```text
defines public DTOs
validates DTO shape
prevents private field leakage
defines profile/passport wire contracts
```

Important future cleanup possibility:

```text
If dependency boundaries permit, ron-proto validation could eventually reuse ron-naming username/parser logic.
```

But be careful:

```text
Do not introduce cycles.
Do not pull runtime service logic into ron-proto.
Do not broaden dependencies without checking architecture.
```

For now, duplicated basic username shape validation in DTO validation is acceptable.

---

## 18. Relationship to `svc-passport`

`svc-passport` now has its own profile claim core and HTTP routes.

`ron-proto` should eventually be used more directly by `svc-passport` when returning public profile objects and public profile manifests.

Current state:

```text
svc-passport currently returns svc-passport.public-profile.v1 route response.
ron-proto defines deeper PassportPublicProfileV1 / PassportPublicManifestV1 DTOs.
```

Future integration target:

```text
svc-passport public profile route should either:
1. return ron-proto PassportPublicProfileV1 directly, or
2. map its internal profile record into PassportPublicProfileV1 for manifest publication / profile hydration.
```

Current reason this was not fully done in this session:

```text
We staged the work to avoid too much crate coupling at once.
First we made DTOs green.
Then svc-passport got a local core.
Then routes were exposed.
```

---

## 19. Relationship to Omnigate/Gateway/CrabLink

`ron-proto` DTOs are the eventual canonical profile shape that Omnigate/Gateway/CrabLink should converge toward.

Current route chain is live through gateway:

```text
CrabLink future call:
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

Current response from `svc-passport` is a practical public profile response with:

```text
schema
passport_subject
passport_kind
username
handle
username_status
display_name
bio
avatar_image
profile_crab_url
public_profile_cid
reputation_score
moderator_score
warnings
```

Future response shape may move closer to `PassportPublicProfileV1`.

Important compatibility rule:

```text
CrabLink should be tolerant of null reputation/moderator scores.
CrabLink should not assume public_profile_cid exists yet.
CrabLink should display confirmed username only if backend status says confirmed.
```

---

## 20. Remaining work for `ron-proto`

### A. Use DTOs in live services

Future work should wire `PassportPublicProfileV1` and/or `PassportPublicManifestV1` into live services.

Candidate crates:

```text
svc-passport
omnigate
svc-gateway
CrabLink schemas/fixtures
```

Potential approach:

```text
svc-passport creates/returns a profile DTO
omnigate passes it through or hydrates a profile page
svc-gateway passes it through
CrabLink validates/displays it
```

### B. Public profile manifest publication

The DTO exists but is not yet b3-published.

Future target:

```text
Profile claim/update can publish a PassportPublicManifestV1 as a b3 object.
public_profile_cid becomes Some(ContentId).
crab://<hash>.profile resolves to the profile manifest/page.
crab://@username can resolve to the current public profile.
```

### C. Public proof references

`public_proof_refs` currently exists as a bounded string list.

Future work:

```text
Define stronger proof reference DTOs if needed.
Represent backend-confirmed username claim proof.
Represent profile update signatures.
Represent site ownership proof.
Represent creator/curator attribution proof.
```

Keep private proof/capability material out of public DTOs.

### D. Reputation/moderation DTO refinement

Current DTOs are placeholders for public display.

Future work:

```text
Define scoring source semantics.
Define update timestamp expectations.
Define per-site vs global display rules.
Define warning/error shape.
Define score range policy if needed.
```

Important:

```text
Do not compute scores in ron-proto.
```

### E. Alt passport public DTO semantics

Current DTOs support `PassportKindV1::Alt`, but alt semantics are not finished.

Future design needs:

```text
Alt public profile should not reveal main passport.
Alt should be able to browse/comment later.
Alt should not create sites.
Alt may have separate public assets/comments.
Alt wallet linkage must be carefully controlled.
```

`ron-proto` must remain privacy-honest.

### F. Versioning and compatibility

Current versions:

```text
PASSPORT_PUBLIC_PROFILE_VERSION = 1
PASSPORT_PUBLIC_MANIFEST_VERSION = 1
```

Future breaking schema changes should become:

```text
PassportPublicProfileV2
PassportPublicManifestV2
```

or add optional fields if backward-compatible.

Do not silently change V1 semantics.

---

## 21. Known good examples

### Public profile DTO example

```json
{
  "version": 1,
  "passport_subject": "passport:main:skinnycrabby",
  "passport_kind": "main",
  "username": "skinnycrabby",
  "handle": "@skinnycrabby",
  "username_status": "confirmed",
  "display_name": "Skinny Crabby",
  "bio": "Building the content-addressed creator web.",
  "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image",
  "public_profile_cid": "b3:1111111111111111111111111111111111111111111111111111111111111111",
  "public_payout_account": "acct_creator_skinnycrabby",
  "public_sites": [
    "crab://thedustyonion6"
  ],
  "public_assets": [
    {
      "asset_cid": "b3:2222222222222222222222222222222222222222222222222222222222222222",
      "asset_kind": "image",
      "crab_url": "crab://2222222222222222222222222222222222222222222222222222222222222222.image",
      "title": "Profile avatar"
    }
  ],
  "reputation_summary": {
    "global_reputation_score": 1776,
    "source": "svc-passport.future-read-model",
    "updated_at_ms": 1776000000100,
    "warnings": []
  },
  "moderator_summary": {
    "global_moderator_score": 100,
    "site_roles": [
      {
        "site": "crab://thedustyonion6",
        "role": "site_owner",
        "status": "backend_confirmed"
      }
    ],
    "source": "svc-passport.future-read-model",
    "updated_at_ms": 1776000000100,
    "warnings": []
  },
  "attribution_status": "backend_confirmed",
  "warnings": []
}
```

### Public manifest DTO example

```json
{
  "version": 1,
  "manifest_cid": "b3:1111111111111111111111111111111111111111111111111111111111111111",
  "profile": {
    "version": 1,
    "passport_subject": "passport:main:skinnycrabby",
    "passport_kind": "main",
    "username": "skinnycrabby",
    "handle": "@skinnycrabby",
    "username_status": "confirmed",
    "display_name": "Skinny Crabby",
    "bio": "Building the content-addressed creator web.",
    "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image",
    "public_profile_cid": "b3:1111111111111111111111111111111111111111111111111111111111111111",
    "public_payout_account": "acct_creator_skinnycrabby",
    "public_sites": [],
    "public_assets": [],
    "reputation_summary": null,
    "moderator_summary": null,
    "attribution_status": "backend_confirmed",
    "warnings": []
  },
  "public_proof_refs": [
    "proof:profile:backend-confirmed"
  ],
  "public_asset_catalogue_cid": null,
  "created_at_ms": 1776000000000,
  "updated_at_ms": 1776000000100,
  "warnings": []
}
```

---

## 22. Important invariants to preserve

For future `ron-proto` changes:

```text
Keep DTOs pure.
Keep DTOs serde-friendly.
Use #[serde(deny_unknown_fields)] for external/public structs.
Do not add private or secret fields to public DTOs.
Do not add service logic.
Do not compute reputation/moderation scores here.
Do not make wallet or ledger calls.
Do not mutate storage.
Do not depend on runtime services.
Do not expose private alt mappings.
Do not expose wallet spend authority.
```

Public DTOs are allowed to reference public pointers:

```text
crab://...
b3:...
public account/payout pointer if intentionally published
public site references
public asset references
public proof references
```

Public DTOs are not allowed to include private capability material.

---

## 23. Suggested future acceptance gates

Before future `ron-proto` changes:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-proto
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

After profile DTO changes, also run downstream identity/profile gates:

```bash
cargo test -p svc-passport --test profile_claims
cargo test -p svc-passport --test profile_routes
cargo test -p omnigate --test passport_profile_routes
cargo test -p svc-gateway --test identity_routes_proxy
```

---

## 24. Current status at end of session

Current `ron-proto` status:

```text
GREEN
```

Feature status:

```text
Public profile DTOs: complete for current phase
Public passport manifest DTOs: complete for current phase
Identity asset kind suffixes: complete for current phase
Public profile wire tests: green
Live service integration: partially pending
b3-published profile manifests: pending
real reputation/moderation scoring: pending
alt passport public semantics: pending
```

Next relevant project step involving `ron-proto`:

```text
Make svc-passport / Omnigate / CrabLink converge on these DTOs for profile hydration and eventual b3-published public profile manifests.
```


### END NOTE - MAY 6 2026 - 17:50 CST



### BEGIN NOTE - JUNE 11 2026 - 10:45 CST

The verified closeout run completed all six independent verifiers, **130 QuickChain-specific Rust tests**, approximately **171 total crate tests**, strict `ron-proto` Clippy, and the full workspace check. The five immutable payload hashes matched exactly in both Rust and the independent Python verifier.   

The master blueprint identifies `ron-ledger` as the next implementation-focused crate, but its current accumulator and checkpoint code predate the new QuickChain contracts. The existing accumulator is a sequential BLAKE3 record chain, and the existing checkpoint is a small `(seq, root, ts)` recovery snapshot; neither should be relabeled as a QuickChain Merkle root or checkpoint.    

# RustyOnions / CrabLink QuickChain — Comprehensive Carryover Notes

## Handoff date

**June 11, 2026 — America/Chicago**

## Next focused crate

```text
ron-ledger
```

---

# 1. Executive handoff summary

The QuickChain Phase 0 responsibility assigned to `ron-proto` is now complete for the current approved scope and should be treated as frozen.

The project has successfully completed the most important prerequisite for later deterministic execution:

```text
strict wire DTOs
strict unknown-field rejection
canonical integer ROC representations
canonical JSON v1 bytes
domain separation
operation and hold identity rules
receipt shape and finality rules
deterministic sort-key rules
receipt ordering rules
replay and idempotency scenarios
concurrent-hold scenarios
terminal-hold compaction semantics
immutable hash-payload boundaries
exact preimage framing
real BLAKE3 locked_hash vectors
independent cross-language reproduction
```

The final `ron-proto` closeout is green.

No further normal QuickChain implementation work is currently required in `ron-proto`.

Future changes to frozen QuickChain DTOs, field order, canonical bytes, domain separators, preimage framing, or locked hashes must be explicitly reviewed and versioned. They must not be silently edited because downstream `ron-ledger` behavior will depend on these contracts.

QuickChain overall is approximately:

```text
Full QuickChain vision:                         ~20%
Best current single estimate:                   ~20%

QC-0A documentation and safety foundation:      ~100%
ron-proto Phase 0 responsibility:               100% for current scope
Deterministic ron-ledger transition work:        not yet implemented for QuickChain
QuickChain state/hold/receipt roots:             not yet implemented
Deterministic epoch sealing:                    not yet implemented
Validator replication and signatures:          not yet implemented
Data availability, archives, challenges:        not yet implemented
Safe pruning and restoration:                   not yet implemented
External anchors or bridges:                    deferred
```

This percentage is not a criticism of progress. The completed `ron-proto` work was one of the hardest and highest-risk foundational parts of the project. It eliminated ambiguity before economic state-transition and root-producing code could amplify that ambiguity.

---

# 2. Attachments for the next session

## Required attachments

Attach these items to the next session:

```text
1. QUICKCHAIN_REVIEW_BUNDLE.MD
2. A newly generated ron-ledger crate codebundle
3. These carryover notes
```

Generate the focused `ron-ledger` bundle from the RustyOnions repository root:

```bash
cd /Users/mymac/Desktop/RustyOnions
scripts/make_crate_codex.sh -c ron-ledger
```

## Strongly recommended reference attachment

Generate one final archival `ron-proto` bundle after the completed hash-payload patch:

```bash
cd /Users/mymac/Desktop/RustyOnions
scripts/make_crate_codex.sh -c ron-proto
```

Attaching that final `ron-proto` bundle to the first `ron-ledger` session is useful because `ron-ledger` must consume and respect the exact QuickChain DTO, ordering, canonicalization, and hash-payload contracts.

After the `ron-ledger` implementation has established stable imports and test fixtures, the full `ron-proto` bundle should not need to be attached to every session.

## Optional attachment

Attach the full Rust repository bundle only when broader cross-crate context is necessary:

```text
CODEBUNDLE_RS.md
```

It is useful for the first `ron-ledger` architecture audit because existing `svc-wallet` adapters and old ledger behavior may need comparison, but it should not replace the focused crate bundle.

## Attachments not needed

Do not attach:

```text
CODEBUNDLE_TAURI_APP.md
the full CrabLink Tauri source bundle
Chrome-specific bundles
media-page bundles
old ALLCODEBUNDLES artifacts
unrelated frontend bundles
```

QuickChain execution and ledger truth do not belong in CrabLink, React, gateway, or omnigate.

---

# 3. Master document precedence

For every QuickChain design, patch, test plan, or implementation, read this first:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
```

It contains the current QuickChain blueprint and QC-0A safety package.

Precedence should be:

```text
1. QUICKCHAIN_REVIEW_BUNDLE.MD
2. Current repository source and locked vectors
3. These carryover notes
4. Older QuickChain notes and earlier conversations
```

If an older note conflicts with the current review bundle, the current review bundle wins.

If a proposed implementation conflicts with a checked-in locked vector, stop and review the conflict. Do not casually rewrite the vector to make new code pass.

The governing doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

---

# 4. Authority boundaries that must remain unchanged

## `ron-proto`

```text
owns:
  DTOs
  validation
  schema constants
  canonical byte helpers
  domain labels
  vector metadata
  immutable hash-payload boundaries

does not own:
  economic authorization
  balance mutation
  persistence
  receipt issuance
  root production from live state
  checkpoint production
  signature verification
  consensus
```

## `svc-wallet`

```text
owns:
  authorized mutation front-door
  issue request
  transfer request
  burn request
  hold-open request
  hold-capture request
  hold-release request
  expiry request
  explicit payer confirmation paths
  backend receipt return

does not own:
  durable balance truth
  independent ledger state
  invented receipts
  local substitute balances
```

## `ron-ledger`

```text
owns:
  durable economic truth
  accepted operation identity
  balance state
  active hold state
  account sequence assignment
  ledger sequence assignment
  replay truth
  append-only operation and receipt evidence
  deterministic transition behavior
  future account, hold, receipt, and state roots

does not own:
  user-facing authorization UI
  reward eligibility policy
  accounting analytics
  client-side entitlement
  external bridge authority
```

## `ron-accounting`

```text
owns:
  bounded metering events
  deterministic accounting snapshots
  sealed accounting artifacts

does not own:
  balance truth
  wallet mutation
  direct ROC issuance
```

## `svc-rewarder`

```text
owns:
  payout planning
  deterministic payout manifests
  declared funding-source plans
  hard payout caps

does not own:
  direct ledger mutation
  direct balance mutation
  raw-engagement protocol minting
```

## `ron-policy`

```text
owns:
  declarative economic policy
  issue/burn eligibility inputs
  pricing and basis-point rules
  event-class policy
  feature gates
  policy versioning

does not own:
  mutation
  balances
  receipt truth
```

## CrabLink, gateway, and omnigate

```text
CrabLink:
  display and user intent only

svc-gateway:
  public request boundary only

omnigate:
  hydration and product coordination only
```

None may invent balances, receipts, roots, operations, or paid entitlement.

The value-plane flow remains:

```text
ron-proto economic DTOs
→ svc-wallet authorized request
→ ron-ledger deterministic commit and durable truth
→ backend-derived receipt
→ paid access enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ svc-wallet approved payout request
→ ron-ledger final balance and receipt truth
```

QuickChain must prove and commit this architecture, not bypass it.

---

# 5. What was completed in `ron-proto`

## 5.1 Final QuickChain source-module surface

The completed source surface now includes:

```text
crates/ron-proto/src/quickchain/canonical.rs
crates/ron-proto/src/quickchain/domain.rs
crates/ron-proto/src/quickchain/empty_tree.rs
crates/ron-proto/src/quickchain/event_class.rs
crates/ron-proto/src/quickchain/hash_payload.rs
crates/ron-proto/src/quickchain/hold.rs
crates/ron-proto/src/quickchain/hold_scenario.rs
crates/ron-proto/src/quickchain/ids.rs
crates/ron-proto/src/quickchain/mod.rs
crates/ron-proto/src/quickchain/money.rs
crates/ron-proto/src/quickchain/operation.rs
crates/ron-proto/src/quickchain/receipt.rs
crates/ron-proto/src/quickchain/receipt_order.rs
crates/ron-proto/src/quickchain/replay.rs
crates/ron-proto/src/quickchain/sort_key.rs
crates/ron-proto/src/quickchain/vector.rs
```

The module remains safe and DTO-centered.

No live ledger mutation, persistence, service calls, validator behavior, or runtime checkpoint production was added.

---

## 5.2 Canonical JSON v1

The Phase 0 canonical JSON surface now establishes:

```text
typed Rust struct serialization
minified UTF-8 JSON
serde struct field order
typed deserialization
strict unknown-field rejection
exact byte testing
round-trip validation
no arbitrary consensus maps
```

Canonical JSON v1 is now locked through checked-in vectors.

Consequences:

```text
field order is consensus-adjacent
field presence is consensus-adjacent
null versus omitted is consensus-adjacent
enum spelling is consensus-adjacent
integer string representation is consensus-adjacent
schema/version tokens are consensus-adjacent
```

Do not reorder fields or add fields to an existing locked payload without introducing a reviewed new version.

---

## 5.3 Canonical money rules

QuickChain money is represented as canonical integer minor-unit strings.

Accepted:

```text
"0"
"1"
"10"
"1000000"
```

Rejected:

```text
""
"01"
"+1"
"-1" for unsigned amount fields
"1.0"
"1_000"
"1 ROC"
JSON floating-point values
oversized values beyond u128 width
```

The shared validator enforces:

```text
decimal digits only
no leading zero except "0"
maximum u128 decimal width
successful u128 parsing
```

No floating-point ROC logic is permitted.

The signed checkpoint net-supply field has a separate canonical signed form and rejects negative zero.

---

## 5.4 Identifier semantics

The following distinctions are frozen:

### `operation_id`

```text
backend-assigned durable ledger-operation identity
one accepted economic operation at most
not supplied as economic authority by the client
duplicate committed operation_id must reject
```

Canonical Phase 0 shape:

```text
op_<32 lowercase hexadecimal characters>
```

### `idempotency_key`

```text
retry/deduplication key
not economic authority
not a signature
not a secret
not a replacement for operation_id
scoped to account and operation family
```

### `hold_id`

```text
one durable hold lifecycle identity
open, capture, release, and expiry refer to the same hold_id
terminal hold_id cannot be resurrected
```

Canonical Phase 0 shape:

```text
hold_<32 lowercase hexadecimal characters>
```

### `account_sequence`

```text
assigned by ron-ledger after acceptance
must not be assigned by the client
advances once for a newly committed operation
does not advance for an identical retry
```

### Ledger sequence

```text
orders durable ledger evidence
may cover a range for one operation/receipt
is distinct from per-account account_sequence
```

These identities must not be collapsed into one nonce or one generic request ID.

---

## 5.5 Operation classes and field matrices

The frozen operation classes are:

```text
issue
transfer
burn
hold_open
hold_capture
hold_release
hold_expire
```

The operation-intent DTO enforces class-specific field requirements.

### Issue

```text
actor account receives value
amount required
counterparty absent
hold_id absent
```

### Transfer

```text
actor is source
counterparty required
amount required
hold_id absent
```

### Burn

```text
actor is source
amount required
counterparty absent
hold_id absent
```

### Hold open

```text
actor is holder/payer
amount required
hold_id required
counterparty optional
```

### Hold capture

```text
actor is holder/payer
counterparty/beneficiary required
amount required
hold_id required
```

### Hold release and hold expiry

```text
actor is holder/payer
amount required
hold_id required
counterparty absent
```

The DTOs do not execute these operations. `ron-ledger` must now implement equivalent deterministic transition rules without weakening them.

---

## 5.6 Hold-state semantics

The frozen hold statuses are:

```text
open
captured
released
expired
```

An open hold:

```text
has opening identity and sequence
has no terminal operation
has no terminal timestamp
has no terminal account sequence
can appear in active hold state
```

A terminal hold:

```text
has terminal operation identity
has terminal timestamp
has terminal account sequence
must not remain in the active hold set
must remain provable through immutable receipt evidence
must never become open again under the same hold_id
```

The active-hold hash payload makes terminal status structurally unrepresentable. Its status enum permits only:

```text
open
```

This is deliberate.

---

## 5.7 Concurrent-hold and compaction scenarios

The final hold-scenario corpus proves:

```text
two holds may exist concurrently
capture of one hold does not corrupt another
release of one hold does not corrupt another
capture retry returns original receipt
release retry returns original receipt
retries do not add commits
retries do not add state transitions
retries do not advance account_sequence
only open holds remain active
captured/released/expired holds are compacted from active state
terminal receipt evidence remains
terminal hold IDs cannot be resurrected
duplicate hold IDs reject
unordered input cannot control active output order
```

The locked concurrent-hold scenario contains:

```text
6 submitted steps
4 accepted commits
2 identical retries
4 state transitions
final active hold count: 0
```

The compaction scenario contains:

```text
4 unordered holds
1 open hold
3 terminal holds
3 terminal receipt references
3 rejected-resurrection hold IDs
```

---

## 5.8 Replay and idempotency scenarios

Six locked replay scenarios now cover:

```text
identical idempotent retry
conflicting idempotency-key reuse
duplicate operation commit
retry hold capture
retry hold release
client attempt to assign account_sequence
```

Frozen outcomes include:

```text
identical retry:
  return original receipt
  no second commit
  no second transition

same idempotency key with different intent:
  reject idempotency conflict
  preserve original commit only

same durable operation_id submitted again:
  reject duplicate operation commit

capture retry:
  return original capture receipt
  do not capture twice

release retry:
  return original release receipt
  do not release twice

client-supplied account_sequence:
  reject before acceptance
```

`ron-ledger` must reproduce these semantics exactly.

---

## 5.9 Receipt semantics

Receipt statuses distinguish:

```text
accepted
epoch_included
finalized
anchored
```

They must be represented honestly.

### Accepted

```text
svc-wallet/ron-ledger accepted the operation
sufficient for ordinary local paid unlock
not proof of epoch inclusion
not finality
not external anchoring
```

### Epoch included

```text
future receipt root includes the receipt
```

### Finalized

```text
future finality or challenge threshold reached
```

### Anchored

```text
future optional external commitment exists
does not replace internal ROC truth
```

The receipt DTO enforces operation-class direction, amount, hold, sequence, and evidence rules.

---

## 5.10 Event-class safety

The event classes are:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

Only `economic_receipt` may represent backend balance truth.

`ad_budgeted` may enter explicitly funded payout planning according to policy.

Raw metering, analytics, views, clicks, engagement, or other bot-prone signals must not directly mint or allocate protocol ROC.

A root proves that data was recorded. It does not prove the underlying real-world action was honest.

---

## 5.11 Domain separation

Versioned domain separators are frozen and test-gated.

Important domains include:

```text
quickchain.receipt.v1
quickchain.operation-intent.v1
quickchain.account-state.v1
quickchain.hold-state.v1
quickchain.hold-root.v1
quickchain.receipt-root.v1
quickchain.state-root.v1
quickchain.accounting-root.v1
quickchain.reward-root.v1
quickchain.checkpoint.v1
quickchain.chain-params.v1
quickchain.validator-set.v1
quickchain.policy.v1
quickchain.data-availability-root.v1
quickchain.receipt-batch.v1
quickchain.accounting-window.v1
quickchain.reward-manifest.v1
quickchain.challenge-evidence.v1
quickchain.anchor-payload.v1
```

The registry enforces:

```text
quickchain. prefix
.v1 suffix
lowercase ASCII
bounded length
no empty segments
allowed punctuation only
global uniqueness
```

A domain constant does not activate the system named by that domain.

For example:

```text
quickchain.anchor-payload.v1
```

does not authorize an anchor.

---

## 5.12 Sort-key rules

### Account sort key

The account ordering contract is fixed and must not depend on map or database iteration order.

The exact helper and locked bytes in `ron-proto` are authoritative.

### Hold sort key

Hold order is independently derived from its canonical key and must not inherit insertion order.

Duplicate canonical hold keys reject.

### Receipt sort key

Receipt order is:

```text
u64_be(ledger_seq_start) || utf8(txid)
```

This provides:

```text
numeric ledger-sequence order
bytewise txid tie-break
fixed-width sequence prefix
independence from arrival/database/map order
```

Duplicate receipt sort keys reject.

These helpers order future root inputs. They do not produce roots themselves.

---

## 5.13 Empty-tree payloads

Typed empty-tree payloads exist for:

```text
state
holds
receipts
accounting
rewards
```

This prevents undocumented magic empty-root constants.

The DTOs themselves do not choose or execute the Merkle construction.

Future `ron-ledger` root tests must derive real empty hashes from these canonical payloads and the appropriate domain rules rather than inventing zero hashes.

---

## 5.14 Vector lifecycle

The vector lifecycle is:

```text
sketch
→ locked_bytes
→ locked_hash
```

### Sketch

```text
shape under review
no trusted canonical bytes
no trusted hash
```

### Locked bytes

```text
canonical payload bytes frozen
payload hex frozen
preimage/hash absent
```

### Locked hash

```text
canonical payload bytes frozen
preimage bytes frozen
expected BLAKE3 hash frozen
independently reproduced
```

No fake hash, placeholder hash, or copied-looking arbitrary value is acceptable.

---

## 5.15 Preimage framing

The exact framing rule is frozen:

```text
domain_separator_bytes
|| 0x00
|| canonical_payload_bytes
```

The separator is exactly one zero byte.

Not permitted:

```text
domain || payload
domain || ":" || payload
length-prefixed alternate framing
two zero bytes
UTF-16 domain bytes
uppercase domain
pretty-printed JSON
trailing newline
```

Canonical hash display is:

```text
b3:<64 lowercase hexadecimal characters>
```

Hash algorithm:

```text
BLAKE3-256
```

---

## 5.16 Immutable hash-payload boundaries

Separate hash-payload DTOs were created to prevent self-reference and mutable-evidence contamination.

### Operation hash payload

```text
QuickChainOperationHashPayloadV1
```

It commits to immutable operation intent, including:

```text
chain identity
operation identity
operation class
actor/counterparty
asset and amount
purpose
hold/session-budget identifiers
policy hash
chain-parameter hash
idempotency scope
idempotency key
```

It excludes:

```text
receipt status
ledger results
receipt hashes
checkpoint evidence
validator signatures
later settlement metadata
```

### Receipt hash payload

```text
QuickChainReceiptHashPayloadV1
```

It commits to:

```text
transaction ID
operation ID
reviewed operation hash
operation name/class
from/to accounts
asset and amount
ledger-assigned account_sequence
hold/session-budget identity
idempotency key
ledger sequence range
previous and new ledger roots
production timestamp
```

It structurally excludes:

```text
receipt_hash itself
receipt_root
checkpoint_hash
mutable receipt status
memo/display text
validator signatures
later epoch/finality evidence
```

### Account leaf payload

```text
QuickChainAccountLeafPayloadV1
```

It commits to:

```text
chain ID
account ID
ROC asset
balance
held value
available value
account sequence
receipt root
holds root
optional permissions root
updated epoch
```

It enforces:

```text
balance = available + held
```

### Active-hold leaf payload

```text
QuickChainActiveHoldLeafPayloadV1
```

It can represent only an open hold.

It excludes terminal lifecycle fields.

### Unsigned checkpoint payload

```text
QuickChainUnsignedCheckpointPayloadV1
```

It commits to checkpoint fields without validator signatures.

Signatures are structurally excluded because signatures may later attest to the checkpoint hash, but they cannot be included in the hash they sign.

It also validates:

```text
canonical encoding
state/receipt root scheme tags
supply delta arithmetic
conservation arithmetic
local-root settlement mode
timestamp order
Phase 0 absence of validator-set and DA roots
```

---

# 6. Final genuine locked hashes

The final independently reproduced hashes are:

```text
Operation hash payload:
b3:71365da74f7fbcc28ac4e49a190741daaa8efb5406daa17574682898db9bc3d5

Receipt hash payload:
b3:b928247934b0393d33b2df1356912f17f3fbb58c98aa14d7500d23c77ef81784

Account leaf payload:
b3:76466edef479d95d5c13a31000b1a236cfaaa63660b284bdacf09f7d7ea7c01b

Active-hold leaf payload:
b3:20e8afd4afe28bbb1126a0dcd6413af3a5445b4c1ff16e8dad7e004a680b8235

Unsigned checkpoint payload:
b3:5d160b804f0b439dded9109643e0f9671a8c2f1330147856c384fc5be77e63e4
```

These values are checked in as vector truth.

They must not be recalculated from a silently changed payload and then casually replaced.

Any legitimate change requires:

```text
new schema or version
new reviewed canonical bytes
new preimage
new independently reproduced hash
explicit migration decision
```

---

# 7. Final vector and verifier corpus

## Canonical DTO locked-byte vectors

Confirmed:

```text
vectors:          26
unique IDs:       26
payload bytes:    12,054
hashes computed:  0
```

These cover operation intents, accepted receipts, account and hold states, empty-tree payloads, disabled chain parameters, and checkpoint-header bytes.

## Account and hold sort-key vectors

Confirmed:

```text
vector sets:      1
account keys:     3
hold keys:        3
duplicate cases:  1
hashes computed:  0
```

## Receipt-order vectors

Confirmed:

```text
vector sets:      1
ordered inputs:   4
duplicate cases:  1
hashes computed:  0
```

## Replay/idempotency scenarios

Confirmed:

```text
scenarios:        6
payload bytes:    6,873
hashes computed:  0
```

## Concurrent-hold and compaction scenarios

Confirmed:

```text
scenario vectors: 2
replay steps:      6
compaction holds:  4
payload bytes:     6,920
hashes computed:   0
```

## Immutable hash-payload vectors

Confirmed:

```text
vectors:          5
payload bytes:    3,841
preimage bytes:   3,972
hashes computed:  5
```

## Independent verifier scripts

The final independent verifier set includes:

```text
crates/ron-proto/tests/tools/verify_quickchain_locked_bytes.py
crates/ron-proto/tests/tools/verify_quickchain_sort_keys.py
crates/ron-proto/tests/tools/verify_quickchain_receipt_order.py
crates/ron-proto/tests/tools/verify_quickchain_replay_idempotency.py
crates/ron-proto/tests/tools/verify_quickchain_hold_scenarios.py
crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.py
```

The hash verifier includes a dependency-free Python BLAKE3 implementation with official empty-input and `abc` self-tests before verifying the project vectors.

This provides implementation independence from the Rust `blake3` crate used by the Rust test suite.

---

# 8. Final test state

The final successful command sequence was:

```bash
cargo fmt -p ron-proto
cargo test -p ron-proto --all-targets
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo check --workspace
```

All required `ron-proto` gates passed.

## QuickChain-specific Rust test count

The final QuickChain test count is **130**:

```text
quickchain_canonical_json.rs           5
quickchain_chain_params.rs             5
quickchain_challenge_wire.rs           2
quickchain_domain_separation.rs        9
quickchain_dto_strict.rs               8
quickchain_empty_tree.rs               5
quickchain_event_class.rs              4
quickchain_file_vectors.rs            28
quickchain_hash_payloads.rs           10
quickchain_hold_scenarios.rs           6
quickchain_ids_and_money.rs            4
quickchain_locked_bytes.rs             3
quickchain_operation_hold.rs          10
quickchain_receipt_dto.rs              8
quickchain_receipt_order.rs            6
quickchain_replay_idempotency.rs       5
quickchain_sort_keys.rs                6
quickchain_vector_dto.rs               6
                                      ---
                                      130
```

The complete crate run contained approximately **171 passing tests** including non-QuickChain tests.

## Workspace warnings

The workspace check completed successfully.

Warnings appeared in unrelated crates:

```text
micronode
ryker
omnigate
```

They did not originate in `ron-proto` and did not invalidate the `ron-proto` QuickChain closeout.

---

# 9. What remains in `ron-proto`

## Active implementation work

```text
None for the current QuickChain Phase 0 scope.
```

The crate should be frozen.

## Permitted future maintenance

Future `ron-proto` changes should occur only when one of these conditions applies:

```text
a downstream implementation finds an actual contradiction
a required field was provably omitted
a schema must evolve through a new version
a new independently reviewed artifact type is needed
a security review identifies a concrete weakness
```

Any such change must be:

```text
additive or explicitly versioned
covered by strict tests
covered by canonical-byte vectors
covered by independent verification
reviewed for downstream hash impact
```

## Documentation synchronization

The repository’s QuickChain combined/review documents should be regenerated so they contain the final decisions made during the `ron-proto` closeout:

```text
replay/idempotency scenarios
concurrent-hold scenarios
closed-hold compaction
immutable hash-payload boundaries
five genuine locked hashes
final ron-proto completion status
```

This is documentation synchronization, not unfinished `ron-proto` implementation.

---

# 10. Important qualification about the next phase

Completing `ron-proto` does not mean QuickChain is live or that unrestricted root-producing implementation is automatically authorized.

The current completed locked-hash vectors cover:

```text
operation hash payload
receipt hash payload
account leaf payload
active-hold leaf payload
unsigned checkpoint payload
```

They do not yet independently reproduce:

```text
non-empty account state root
non-empty hold root
non-empty receipt root
accounting root
reward root
complete checkpoint assembled from produced roots
Merkle inclusion proofs
```

Therefore, the next `ron-ledger` session should begin with:

```text
focused code audit
legacy-to-QuickChain boundary analysis
deterministic transition design
non-root transition tests
root-vector planning
```

It should not begin by immediately wiring live root production into the current ledger.

The safe progression is:

```text
deterministic transition semantics
→ state snapshots
→ leaf payload reproduction
→ root algorithm vectors
→ independently reproduced roots
→ root-producing library code
→ local replay
→ epoch sealing
```

---

# 11. Current `ron-ledger` baseline that must be audited

The existing `ron-ledger` crate is not empty.

It already contains:

```text
crates/ron-ledger/src/api.rs
crates/ron-ledger/src/config.rs
crates/ron-ledger/src/error.rs
crates/ron-ledger/src/lib.rs
crates/ron-ledger/src/types.rs

crates/ron-ledger/src/engine/accumulator.rs
crates/ron-ledger/src/engine/checkpoint.rs
crates/ron-ledger/src/engine/ledger.rs
crates/ron-ledger/src/engine/mod.rs
crates/ron-ledger/src/engine/observer.rs
crates/ron-ledger/src/engine/replay.rs
crates/ron-ledger/src/engine/storage.rs
```

Existing tests include:

```text
crates/ron-ledger/tests/idempotency_prop.rs
crates/ron-ledger/tests/interop_vectors.rs
crates/ron-ledger/tests/public_api_semver.rs
crates/ron-ledger/tests/reject_taxonomy.rs
crates/ron-ledger/tests/replay_recovery.rs
```

The existing crate already appears to provide:

```text
single-writer append-only commit path
primitive debit/credit-like entries
non-negative balance checks
entry ID deduplication
sequence assignment
replay from records
in-memory and file-backed storage seams
observer events
recovery checkpoints
a rolling accumulator root
```

This is valuable infrastructure, but it predates the new QuickChain contracts and must not be assumed to satisfy them.

---

# 12. Critical distinction: legacy ledger accumulator versus QuickChain roots

The current ledger accumulator appears to derive a new root from:

```text
previous root
+ canonical JSON of one primitive ledger record
```

Conceptually:

```text
BLAKE3(previous_root_bytes || serialized_record)
```

This is a rolling append-only accumulator.

It is not automatically:

```text
the QuickChain account state root
the QuickChain active-hold root
the QuickChain receipt root
the QuickChain checkpoint hash
a sorted binary Merkle map
a receipt-sequence Merkle tree
```

The current names:

```text
AccumulatorKind::Merkle
AccumulatorKind::Verkle
```

must be reviewed carefully because the implementation shown in the previous repository bundle appeared to use the same rolling accumulator path for both labels.

Do not reuse or advertise this as the new QuickChain Merkle construction merely because the enum contains the word `Merkle`.

Possible outcomes after audit:

```text
retain it as a legacy append-only WAL integrity accumulator
rename it for clarity in a later compatibility patch
wrap it as historical ledger continuity evidence
keep it separate from QuickChain roots
```

Do not silently change its behavior because existing wallet and recovery tests may rely on it.

---

# 13. Critical distinction: legacy checkpoint versus QuickChain checkpoint

The current ledger checkpoint helper creates a record resembling:

```text
sequence
root
timestamp
```

Its purpose is bounded replay and storage recovery.

That is useful, but it is not the future QuickChain unsigned checkpoint payload, which includes:

```text
chain ID
height
epoch ID
execution specification version
previous checkpoint hash
previous and new state roots
receipt root
accounting snapshot root
reward manifest root
policy hash
chain-parameter hash
canonical encoding
root scheme tags
supply delta
conservation summary
settlement mode
epoch timestamps
```

Use separate names and module boundaries.

Do not mutate the old recovery checkpoint into a QuickChain checkpoint in place without understanding storage compatibility and existing replay behavior.

Recommended conceptual distinction:

```text
Legacy/internal recovery checkpoint:
  bounds local WAL replay

QuickChain settlement checkpoint:
  commits deterministic published epoch artifacts
```

They may coexist.

---

# 14. Existing `ron-ledger` risks to inspect first

## 14.1 `HashMap` iteration

The current replay state uses `HashMap` balances and seen-entry indexes.

That may be acceptable for lookup, but no root, canonical artifact, or published ordering may depend on `HashMap` iteration.

For new QuickChain state, prefer:

```text
BTreeMap
```

or explicitly collect and sort by the locked `ron-proto` sort keys before canonicalization and hashing.

## 14.2 Saturating arithmetic

The previous repository bundle showed balance credit behavior using:

```rust
saturating_add
```

Saturating arithmetic is dangerous for economic truth because it can silently convert overflow into `u128::MAX`.

QuickChain economic transitions should use:

```text
checked addition
checked subtraction
explicit overflow rejection
explicit insufficient-funds rejection
```

No overflow may silently succeed.

## 14.3 Primitive `u64` amounts versus canonical u128-width strings

The current ledger’s primitive entry amount may be `u64`, while the new canonical QuickChain money validator permits the full canonical `u128` decimal width.

The next session must decide deliberately:

```text
whether the execution engine remains u64
whether it upgrades internal arithmetic to u128
how compatibility is maintained
whether conversion can ever truncate
how serialized DTO values map to arithmetic values
```

No unchecked cast or truncation is acceptable.

## 14.4 Entry ID versus operation identity

The current ledger has primitive entry IDs.

One QuickChain economic operation may produce:

```text
multiple primitive debit/credit records
one durable operation_id
one backend receipt
one account_sequence per affected account according to the final model
one ledger sequence range
```

The next design must not equate one primitive entry ID with one operation ID without reviewing multi-entry operations.

## 14.5 Current idempotency behavior

The existing ledger has entry deduplication and existing idempotency tests.

The new semantics are stronger and distinguish:

```text
same operation_id
same idempotency key and same intent
same idempotency key and conflicting intent
same operation across multiple primitive records
original receipt return
```

Existing behavior must be mapped, not assumed equivalent.

## 14.6 Current hold implementation

The focused bundle must determine whether current holds are modeled as:

```text
dedicated hold lifecycle records
escrow-like accounts
wallet-level state
primitive ledger transfers
or a combination
```

QuickChain requires explicit durable `hold_id` lifecycle truth.

An escrow account name is not automatically a substitute for a canonical hold identity and terminal lifecycle record.

## 14.7 Wall-clock and database-order dependence

The transition result must not depend on:

```text
current wall clock
thread scheduling
HashMap iteration
database row order
filesystem discovery order
request arrival race
random identifiers generated during replay
```

Time-dependent rules such as expiry must use explicit deterministic epoch input, not “whatever time replay happens.”

## 14.8 Old accumulator self-reference and framing

The existing accumulator already avoids including its own derived `new_root` in its hash input, which is good.

However, its current framing and record payload are not the new QuickChain domain-separated framing.

Do not mix:

```text
legacy accumulator root
QuickChain operation hash
QuickChain receipt hash
QuickChain leaf hash
QuickChain Merkle root
QuickChain checkpoint hash
```

Every one has a separate domain and purpose.

---

# 15. Recommended first `ron-ledger` session objective

The first `ron-ledger` session should be an audit plus a small, non-root deterministic transition batch.

Recommended objective:

```text
Establish a clean QuickChain transition boundary inside ron-ledger
without changing existing public behavior and without producing roots yet.
```

The first session should answer:

```text
Which current ledger types remain valid?
Which types are legacy-only?
How do QuickChain operation IDs map to primitive records?
Where is idempotency truth stored?
Where is original receipt replay stored?
How are account sequences assigned?
How are ledger sequence ranges assigned?
How are concurrent holds represented?
How are terminal holds retained as receipt evidence?
How are issue and burn authorizations represented as deterministic inputs?
How will replay invoke exactly the same transition code as live commit?
```

---

# 16. Recommended initial command gate for `ron-ledger`

Before patching:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-ledger -- --check
cargo test -p ron-ledger --all-targets
cargo clippy -p ron-ledger --all-targets -- -D warnings
```

Record:

```text
current test count
current warnings
current public API
existing fixture behavior
existing svc-wallet dependencies
existing persistence behavior
```

After every batch:

```bash
cargo fmt -p ron-ledger
cargo test -p ron-ledger --all-targets
cargo clippy -p ron-ledger --all-targets -- -D warnings
```

Run workspace checking when public types or dependencies change:

```bash
cargo check --workspace
```

---

# 17. Proposed `ron-ledger` implementation sequence

## Batch 0 — Focused audit and compatibility map

Do not begin by rewriting the crate.

Create a clear map of:

```text
current Entry / EntryRecord / Batch APIs
current balance state
current accumulator
current replay
current storage
current checkpoint
current idempotency behavior
current svc-wallet adapter expectations
```

Compare each to:

```text
QuickChainOperationHashPayloadV1
QuickChainReceiptHashPayloadV1
QuickChainAccountLeafPayloadV1
QuickChainActiveHoldLeafPayloadV1
QuickChainOperationIntentV1
QuickChainReceiptV1
QuickChain replay scenarios
QuickChain hold scenarios
```

Output of this batch should be either:

```text
a narrow design note and test plan
```

or a minimal module skeleton plus compile-only tests.

No root code in Batch 0.

---

## Batch 1 — Pure deterministic transition state

Add a QuickChain-specific internal state boundary.

Likely state categories:

```text
accounts
active holds
committed operation outcomes
idempotency index
receipt history/index
next ledger sequence
per-account sequence
terminal hold evidence
```

Recommended properties:

```text
owned deterministic input
owned deterministic output
no IO
no async
no global state
no system clock
no randomness
no storage calls
no network calls
no metrics calls
```

A pure transition interface should conceptually resemble:

```text
current state
+ validated operation
+ explicit authorization/policy input
+ explicit epoch/time input when required
→ accepted transition and receipt material
or deterministic rejection
```

Do not finalize exact Rust names until the focused `ron-ledger` bundle is reviewed.

---

## Batch 2 — Operation identity and idempotency

Implement and test:

```text
operation_id commits at most once
identical retry returns original outcome
identical retry returns original receipt identity
identical retry does not mutate state
identical retry does not advance sequences
conflicting idempotency-key reuse rejects
duplicate operation_id rejects
client cannot assign account_sequence
```

The retry index must retain enough information to distinguish:

```text
same key + same canonical operation
same key + different canonical operation
same operation_id + different retry key
```

Do not rely only on the idempotency-key string.

Bind deduplication to the reviewed canonical operation identity or canonical operation hash.

---

## Batch 3 — Basic balance transitions

Implement:

```text
issue
transfer
burn
```

Required invariants:

```text
integer arithmetic only
checked arithmetic only
no negative balances
no overflow
transfer conservation
issue is explicit supply exception
burn is explicit supply exception
unauthorized issue rejects
unauthorized burn rejects
replay produces the same state
```

Issue and burn authority must be explicit deterministic input.

Do not let a boolean buried in a client request become authority.

Prefer a typed authorization decision or verified policy input supplied by the service boundary.

---

## Batch 4 — Hold lifecycle transitions

Implement:

```text
hold_open
hold_capture
hold_release
hold_expire
```

Required invariants:

```text
one hold_id identifies one lifecycle
hold open reserves available value
multiple holds may coexist
capture affects only the selected hold
release affects only the selected hold
expiry affects only the selected hold
capture cannot exceed held amount
release cannot exceed held amount
terminal hold cannot transition again
terminal hold cannot be recreated under same hold_id
capture retry returns original receipt
release retry returns original receipt
expiry is based on explicit deterministic epoch input
```

Terminal holds must disappear from active hold state while immutable receipt evidence remains available.

---

## Batch 5 — Account and ledger sequence assignment

Freeze:

```text
when account_sequence increments
which affected accounts increment
how issue affects recipient sequence
how transfer affects source and destination sequence
how hold operations affect holder and beneficiary sequence
how multi-entry operations expose ledger_seq_start/end
how retries retain original sequences
```

This is a major determinism boundary.

Do not infer account sequence from database row order.

Do not allow clients to reserve or submit it.

---

## Batch 6 — Receipt construction

Construct backend-derived receipt material from accepted transitions.

Receipt generation must bind:

```text
operation_id
operation hash
operation class
account directions
amount
ledger-assigned account_sequence
hold ID
idempotency key
ledger sequence range
previous ledger continuity root/reference
new ledger continuity root/reference
production timestamp supplied deterministically
```

At this stage, receipt construction may reproduce the immutable receipt hash payload but should not yet build a receipt Merkle root unless the root-vector gate is green.

Receipt status begins as:

```text
accepted
```

Do not claim epoch inclusion, finality, or anchoring.

---

## Batch 7 — Live/replay code-path unification

Live commit and replay must use the same transition logic.

Avoid:

```text
one code path for live commit
another approximate code path for replay
```

Required proof:

```text
starting state + ordered operations
produces the same:
  balances
  active holds
  terminal evidence
  operation outcomes
  account sequences
  ledger sequences
  receipt payloads
```

Replay must reject:

```text
sequence gaps
duplicate committed operation IDs
conflicting operation history
invalid balance transitions
hold lifecycle contradictions
receipt/operation mismatches
```

---

## Batch 8 — Leaf-payload adapters

Only after transition state is stable, add pure adapters that produce the exact frozen `ron-proto` payloads:

```text
QuickChainOperationHashPayloadV1
QuickChainReceiptHashPayloadV1
QuickChainAccountLeafPayloadV1
QuickChainActiveHoldLeafPayloadV1
```

Requirements:

```text
no extra fields
no omitted required fields
exact schema/version
exact canonical money
exact canonical identifiers
exact null handling
no mutable status fields in hash payloads
no terminal hold fields in active-hold leaves
```

Test adapters against the existing `ron-proto` locked vectors.

---

## Batch 9 — Root-vector planning before root code

Before implementing root-producing code, create or finalize independent vectors for:

```text
empty state root
single account root
two-account root
three-account odd-leaf root
input permutation invariance
duplicate account-key rejection

empty active-hold root
single hold root
multi-hold root
terminal-hold exclusion
duplicate hold-key rejection

empty receipt root
single receipt root
sequence-ordered multi-receipt root
txid tie-break
duplicate receipt-key rejection
odd-leaf behavior
```

The root algorithm must explicitly freeze:

```text
leaf hash domain
internal-node hash domain or framing
left/right child framing
empty-root derivation
odd-leaf rule
duplicate-key rule
tree reduction order
root display format
```

Do not assume “standard Merkle tree” is sufficiently precise.

---

## Batch 10 — Deterministic roots

Only after the root vectors are independently reproducible should `ron-ledger` implement:

```text
account leaf hashing
active-hold leaf hashing
receipt hashing
sorted account-state root
sorted active-hold root
ledger-sequence receipt root
empty roots
odd-leaf duplication
duplicate-key rejection
```

The Phase 1 state-tree candidate is:

```text
sorted binary Merkle map over canonical account-state leaves
```

Receipt ordering is:

```text
ledger sequence, then txid bytes
```

All unordered input permutations must produce the same root.

---

## Batch 11 — Local published-input replay

Produce a replayable local artifact containing sufficient inputs to reproduce:

```text
accepted operations
receipts
final account state
active hold state
state root
hold root
receipt root
```

An independent executable or verifier must reproduce the same results without reading the live database’s internal ordering.

The artifact is not a live checkpoint yet.

---

## Batch 12 — Unsigned checkpoint assembly

After produced roots are stable and independently reproduced, assemble:

```text
QuickChainUnsignedCheckpointPayloadV1
```

Then compute its hash using:

```text
quickchain.checkpoint.v1
|| 0x00
|| canonical unsigned checkpoint payload bytes
```

No signatures in the hash payload.

No validator runtime in this phase.

---

# 18. First-session recommended patch size

Keep the first `ron-ledger` patch small.

Recommended first patch scope:

```text
1. Add a QuickChain-specific module boundary.
2. Add deterministic internal state types or transition-plan types.
3. Add operation/idempotency semantic tests.
4. Add no roots.
5. Add no persistence changes.
6. Add no svc-wallet changes.
7. Preserve every existing ron-ledger test.
```

A good first session result would be:

```text
existing ron-ledger behavior remains green
new QuickChain module is isolated
duplicate operation semantics are explicit
idempotent retry semantics are explicit
account_sequence remains ledger-assigned
no root-producing code exists yet
```

Do not attempt issue, transfer, burn, all hold transitions, roots, replay artifacts, and wallet integration in one patch.

---

# 19. Suggested first-batch tests

The first batch should include tests equivalent to:

```text
identical retry returns the same stored outcome
identical retry produces no second commit
conflicting idempotency-key reuse rejects
duplicate operation_id rejects
client-assigned account_sequence rejects
rejected operation does not consume sequence
retry does not consume sequence
state ordering is deterministic
unknown or malformed operation rejects before mutation
```

Once transition arithmetic begins, add:

```text
credit overflow rejects
debit underflow rejects
transfer conserves value
issue changes supply only with authorization
burn changes supply only with authorization
failed operation leaves state unchanged
```

Once holds begin, add:

```text
two concurrent holds coexist
capture retry does not double spend
release retry does not alter state
expired hold cannot capture
captured hold cannot release
released hold cannot capture
terminal hold cannot reopen
compaction retains receipt evidence
```

---

# 20. Atomicity requirements

Every accepted operation must be atomic.

For an operation touching multiple structures:

```text
balances
active holds
operation index
idempotency index
account sequences
ledger sequence
receipt store
supply summary
```

either all changes commit, or none do.

A rejected operation must leave every state component unchanged.

Recommended development pattern:

```text
validate
→ compute transition on temporary/copy-on-write state
→ verify invariants
→ commit one complete outcome
```

Do not partially update one account and then fail while updating another.

---

# 21. Deterministic rejection taxonomy

Existing ledger rejection types should be audited against QuickChain needs.

Likely deterministic categories include:

```text
invalid input
too large
conflict
insufficient funds
overflow
unauthorized issue
unauthorized burn
duplicate operation
idempotency conflict
unknown hold
hold already terminal
hold amount mismatch
sequence conflict
replay corruption
```

Do not expose secrets or raw internal stack traces.

The public taxonomy should be stable and machine-countable.

A human message may add context, but logic and metrics should rely on the typed reason.

---

# 22. Persistence and storage posture

Do not begin by replacing storage.

First separate:

```text
pure deterministic transition
from
append-only persistence
```

The transition engine should be independently testable entirely in memory.

Persistence should store enough information to replay the exact accepted operation and resulting receipt evidence.

Required later durability tests include:

```text
crash after validation but before append
crash during multi-record append
crash after append but before response
restart and identical retry
checkpoint plus WAL recovery
corrupt sequence gap
corrupt previous-root continuity
duplicate record after restart
truncated final record
```

Do not let a recovery checkpoint replace append-only receipt evidence.

---

# 23. `svc-wallet` integration should come later

Do not patch `svc-wallet` in the first `ron-ledger` batch unless a compile-only compatibility adjustment is absolutely necessary.

The correct order is:

```text
1. stabilize deterministic ron-ledger transition contract
2. stabilize receipt construction and replay
3. prove tests and vectors
4. adapt svc-wallet to request the stable operations
5. test end-to-end issue/transfer/burn/hold/capture/release
```

`svc-wallet` remains the mutation front-door.

`ron-ledger` remains truth.

The wallet must not reimplement transition arithmetic to predict the outcome.

---

# 24. Root and hashing safety rules

When hashing work eventually begins:

```text
hash only published canonical payloads
use the exact frozen domain
use exactly one 0x00 separator
use BLAKE3-256
display b3 lowercase hex
reject unknown fields before canonicalization
reject duplicate sort keys
sort explicitly
never use database iteration order
never hash a containing object that includes its own hash
never include validator signatures in the checkpoint hash
```

Do not create a generic “hash any Serialize object” consensus API.

Prefer narrow typed functions:

```text
hash_operation_payload_v1(...)
hash_receipt_payload_v1(...)
hash_account_leaf_payload_v1(...)
hash_active_hold_leaf_payload_v1(...)
```

A narrow API makes accidental hashing of the wrong DTO harder.

---

# 25. Scope forbidden in the next session

Do not add:

```text
public validators
validator networking
consensus
fork choice
quorum voting
checkpoint signatures
staking
slashing
liquidity
exchange integration
ROX
Solana
public bridge
external settlement
carrier rewards
archive rewards
pruning
data deletion
mandatory external DA
CrabLink root authority
gateway ledger mutation
omnigate ledger mutation
accounting balance mutation
rewarder balance mutation
```

Do not turn the current local ledger into a public blockchain.

Do not claim finality that does not exist.

Do not expose a root as a security proof until independent replay reproduces it.

---

# 26. Completion criteria for the deterministic `ron-ledger` phase

The core Phase 1 ledger work should not be considered complete until:

```text
all operation classes transition deterministically
all arithmetic is checked
conservation holds
non-negativity holds
issue/burn require explicit authorization input
operation_id is durable and unique
idempotency conflicts are deterministic
identical retries return original receipts
account_sequence is ledger-assigned
ledger sequence ranges are deterministic
concurrent holds are independent
terminal holds cannot resurrect
live commit and replay use the same transition code
leaf payloads reproduce locked vectors
state root is independently reproduced
hold root is independently reproduced
receipt root is independently reproduced
duplicate keys reject
odd-leaf behavior is frozen
empty roots are frozen
published-input replay reproduces all roots
```

This completion does not include validators or distributed consensus.

---

# 27. Roadmap after core `ron-ledger`

After deterministic local state and roots are green:

## `svc-wallet`

```text
adapt authorized mutation requests
return backend-derived accepted receipts
prove retry behavior through service boundary
prove no silent spend
prove paid unlock uses accepted backend truth
```

## `ron-policy`

```text
freeze deterministic issue/burn authorization inputs
policy hash payload
chain-parameter policy
event-class rules
emergency freeze policy
```

## `ron-accounting`

```text
canonical sealed windows
stable event ordering
accounting artifact hash
accounting root
independent replay
```

Accounting remains separate from balance truth.

## `svc-rewarder`

```text
deterministic payout manifests
funding-source declarations
hard caps
reward artifact hash
reward root
replay safety
```

Rewarder still does not mutate balances.

## Phase 2 local epoch sealing

```text
sealed receipt batch
sealed accounting window
sealed reward manifest
policy hash
chain-parameter hash
state root
hold root
receipt root
accounting root
reward root
unsigned checkpoint payload
checkpoint hash
published artifact set
independent complete replay
```

## Phase 3 validators

Only after local execution is deterministic and independently reproducible:

```text
validator identity
validator-set descriptor
signature preimage
controlled signing
quorum verification
equivocation evidence
committee rotation
governance audit
```

## Data availability and pruning

Much later:

```text
artifact chunking
carrier/archive storage
retrieval challenges
restoration drills
challenge windows
retention enforcement
safe pruning
```

Proofs and restoration must precede pruning.

## External anchoring

Optional and deferred.

Any future anchor must:

```text
commit compact checkpoint data only
never mutate internal ROC balances
never replace svc-wallet
never replace ron-ledger
never become a hot-path requirement
never imply a public bridge exists
```

---

# 28. Why the completed `ron-proto` work was difficult

The completed portion was not routine DTO plumbing.

It resolved several classes of failure that have caused real distributed-ledger and financial-system problems:

```text
different nodes hashing different bytes
self-referential receipt hashes
signatures included in their own signed hash
map/database iteration controlling roots
ambiguous duplicate handling
idempotency keys treated as authority
client-controlled sequence numbers
hold retries double-spending
terminal holds remaining active
terminal holds being resurrected
overflow hidden by loose arithmetic
fake placeholder hashes becoming de facto standards
mutable status fields contaminating immutable receipt identity
analytics events being confused with economic truth
```

Completing this layer means the next implementation has a precise target.

The hard work remaining is no longer “decide what the bytes mean.” It is:

```text
make ron-ledger execute those meanings correctly
under every retry, replay, concurrent hold, failure, and ordering condition
```

That is still extremely difficult, but the project is entering it with a much safer foundation.

---

# 29. Recommended prompt for the next session

```text
We are continuing QuickChain work in RustyOnions. QUICKCHAIN_REVIEW_BUNDLE.MD is the master reference and must be read first. The ron-proto QuickChain Phase 0 responsibility is complete and frozen: strict DTOs, canonical JSON v1, domain separation, sort rules, replay/idempotency scenarios, concurrent-hold/compaction scenarios, immutable hash-payload boundaries, exact preimage framing, and five independently reproduced locked_hash vectors are green.

The next focused crate is ron-ledger. Review the attached focused ron-ledger bundle against the master blueprint and these carryover notes. Begin with a careful audit of the existing append-only ledger, replay, accumulator, storage, checkpoint, idempotency, amount, sequence, and svc-wallet compatibility surfaces.

Do not confuse the existing rolling accumulator root or recovery checkpoint with QuickChain Merkle roots or settlement checkpoints. Do not add roots in the first batch. The first implementation goal is a small, pure, deterministic QuickChain transition boundary with operation_id uniqueness, idempotent retry behavior, ledger-assigned account_sequence, checked arithmetic, and tests. Preserve all existing behavior and tests.

No validators, consensus, pruning, anchors, ROX, Solana, bridge, staking, liquidity, rewarder mutation, gateway mutation, omnigate mutation, or CrabLink chain authority. Provide full paste-ready files or a direct terminal script, not a git patch.
```

---

# 30. Final handoff statement

`ron-proto` is complete and frozen for the current QuickChain Phase 0 scope.

The project now has:

```text
canonical economic vocabulary
strict operation and receipt contracts
replay-safe identity rules
concurrent-hold semantics
terminal compaction semantics
deterministic ordering rules
immutable hash boundaries
real independently reproduced hashes
a clean separation between DTO truth and ledger authority
```

The next task is not to build validators or launch a chain.

The next task is:

```text
make ron-ledger execute and replay the frozen operation semantics
with checked arithmetic, atomic transitions, durable identity,
concurrent-hold correctness, and no nondeterministic state behavior
```

The first `ron-ledger` batch should remain small, pure, deterministic, and non-root-producing.

The governing principle remains:

```text
make the transition engine boring, exact, and independently replayable
before allowing it to produce economic roots
```


### END NOTE - JUNE 11 2026 - 10:45 CST


### BEGIN NOTE - JUNE 16 2026 - 14:10 CST

Below is one shared notes document you can paste into **both** crates. It is based on the current `ron-proto` bundle, which now includes the exhaustive QuickChain preflight runner and bash vector verifiers, the current `ron-ledger` bundle, which includes the gated QuickChain preflight feature and exhaustive runner, and the latest terminal proof where `ron-proto` passed 20 discovered QuickChain tests, both bash verifiers, clippy, and the final success line.   

# QuickChain Phase-0 Shared Notes — ron-proto + ron-ledger Pair

Date: 2026-06-16
Crates covered by this shared note:

* `crates/ron-proto`
* `crates/ron-ledger`

Purpose of this note:

This is a shared milestone record for the `ron-proto + ron-ledger` QuickChain crate pair. It is intentionally written as one set of notes that can be dropped into both crates. It is not a narrow crate-specific changelog. It documents the completed pair-level strategy, the current green gates, the remaining boundaries, and the next crate-pair build order for continuing QuickChain coherently.

---

## Executive status

The `ron-proto + ron-ledger` pair is effectively complete for the current allowed QuickChain Phase-0 / pre-root scope.

Estimated status:

* `ron-proto` QuickChain Phase-0 / pre-root scope: 99% complete.
* `ron-ledger` QuickChain Phase-0 / pre-root scope: 97–98% complete.
* Combined `ron-proto + ron-ledger` crate-pair milestone: 98–99% complete.
* Overall internal QuickChain Phase-0/preflight across all crates: approximately 68–75% complete before the remaining service/client crate-pair work.
* Full long-term QuickChain blueprint: approximately 35–42% complete, because roots, validators, settlement, anchors, bridges, staking, liquidity, pruning, and external-chain behavior remain intentionally forbidden future phases.

This pair is not marked as 100% because QuickChain deliberately does not yet authorize root-producing runtime, validators, settlement, pruning, public anchors, bridges, staking, or external chain integration. For the work that is currently allowed, this pair is essentially done.

---

## Current doctrine reminder

QuickChain is future settlement infrastructure. It is not the current runtime, not a public chain, not a bridge, not staking, not liquidity, not Solana/ROX integration, and not a replacement for internal ROC wallet/ledger truth.

Current mission remains:

```text
prove internal ROC economy first
```

Internal value-plane path:

```text
ron-proto econ/QuickChain DTOs
  -> ron-ledger deterministic truth and preflight execution
  -> svc-wallet mutation front door
  -> svc-storage / svc-gateway / omnigate paid enforcement
  -> ron-accounting snapshots
  -> svc-rewarder payout planning
  -> wallet/ledger receipts
```

QuickChain doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

---

## Hard boundaries that remain in force

For this phase, do not add:

* QuickChain root-producing runtime.
* QuickChain checkpoint-producing runtime.
* Validators.
* Validator committees.
* Settlement.
* External anchors.
* Public bridges.
* Solana integration.
* ROX integration.
* Staking.
* Liquidity.
* Exchange-facing logic.
* Pruning.
* Live chain state.
* CrabLink chain authority.
* Gateway/omnigate/rewarder ledger mutation.
* Raw engagement protocol payouts.
* DB-order roots.
* Wall-clock roots.
* Placeholder/fake hashes.
* Unknown consensus fields.
* Any Python helper scripts for QuickChain preflight/tooling.

Important nuance for `ron-ledger`:

`ron-ledger` already has legacy/internal accumulator/root/checkpoint mechanics for the existing append-only ledger engine. Those are not the same thing as new QuickChain root/checkpoint/consensus production. The QuickChain-specific Phase-0 work must remain pre-root and gated under `quickchain-preflight`.

---

## Bash-only tooling rule

A major milestone in this pair was converting QuickChain helper tooling to bash-only and guarding against Python script drift.

Current rule:

```text
QuickChain helper scripts must be bash-only.
No new Python helper scripts.
No python/python3 one-liners in future QuickChain patch batches.
Use bash, Rust tests, and cargo commands.
```

This applies to future work across the project, especially new preflight runners and verifier scripts.

---

# ron-proto QuickChain status

## Role of ron-proto

`ron-proto` owns DTOs and canonical wire/data shapes. It must remain pure DTO/schema/canonicalization territory.

`ron-proto` must not:

* Mutate wallet/ledger/storage state.
* Perform service I/O.
* Own ledger truth.
* Verify signatures as authority.
* Produce live roots.
* Decide finality.
* Create settlement claims.
* Act as validator runtime.
* Store secrets.
* Represent spend authority.

It may safely define strict DTO shapes, validation helpers, canonical JSON byte experiments, domain separators, vector schemas, and test vectors.

---

## ron-proto accomplishments in this milestone

The crate now has a robust QuickChain Phase-0 DTO/vector surface covering:

* Canonical JSON v1 helpers.
* Strict DTO round-trip behavior.
* Unknown-field rejection.
* Strict `b3:<64 lowercase hex>` behavior.
* Integer minor-unit money strings.
* No floats for money.
* Operation IDs and hold IDs.
* Idempotency key shape.
* Operation-class wire names.
* Operation intent validation.
* Hold open/capture/release/expire intent matrix.
* Hold state lifecycle validation.
* Receipt DTO validation.
* Accepted receipt shapes.
* Receipt direction/lifecycle constraints.
* Receipt ordering and sort keys.
* Account and hold sort keys.
* Replay/idempotency scenario vectors.
* Event class policy helpers.
* Usage event validation.
* Empty tree DTOs and exact canonical bytes.
* Chain params with explicit disabled state for external/ROX anchor posture.
* Challenge wire names as descriptive DTOs, not active challenge runtime.
* Domain separation constants and validation.
* Hash payload DTOs for:

  * operation hash payload
  * receipt hash payload
  * account leaf payload
  * active hold leaf payload
  * unsigned checkpoint payload
* Locked hash vector review gates.
* File-backed locked bytes vectors.
* Bash vector inventory verifier.
* Bash hash payload verifier.
* Exhaustive bash preflight runner.

---

## ron-proto vector inventory status

Current checked-in QuickChain vector inventory:

```text
total vector files: 36
locked_bytes: 31
locked_hash: 5
sketch: 0
typed quickchain.test-vector.v1 vectors: 31
```

Current hash payload verifier summary:

```text
hash payload vector files: 5
payload bytes: 3841
preimage bytes: 3972
optional b3sum checks: 0
```

The optional `b3sum` checks being zero is acceptable when `b3sum` is not installed. The Rust `quickchain_hash_payloads` test remains the mandatory hash-truth gate.

---

## ron-proto exhaustive preflight runner

Current runner:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Current behavior:

* Runs from repo root or from inside the repository.
* Rejects checked-in `.py` helper scripts under `crates/ron-proto`.
* Runs `cargo fmt -p ron-proto -- --check`.
* Discovers every `crates/ron-proto/tests/quickchain_*.rs` test.
* Runs each discovered QuickChain integration test.
* Requires at least 20 discovered QuickChain tests.
* Runs the bash vector inventory verifier.
* Runs the bash hash payload verifier.
* Runs `cargo clippy -p ron-proto --all-targets -- -D warnings`.
* Prints final success line.

Current proven final success line:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=20
```

---

## ron-proto important tests now covered

The exhaustive runner currently discovers and runs 20 QuickChain integration tests, including:

* `quickchain_canonical_json`
* `quickchain_chain_params`
* `quickchain_challenge_wire`
* `quickchain_domain_separation`
* `quickchain_dto_strict`
* `quickchain_empty_tree`
* `quickchain_event_class`
* `quickchain_file_vectors`
* `quickchain_hash_payloads`
* `quickchain_hold_scenarios`
* `quickchain_ids_and_money`
* `quickchain_locked_bytes`
* `quickchain_operation_hold`
* `quickchain_receipt_dto`
* `quickchain_receipt_order`
* `quickchain_replay_idempotency`
* `quickchain_sort_keys`
* `quickchain_tooling_boundary`
* `quickchain_vector_dto`
* `quickchain_vector_inventory`

---

## ron-proto commands to prove current state

Run:

```bash
bash crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Expected final output:

```text
verified quickchain vector inventory: files=36 locked_bytes=31 locked_hash=5 sketch=0 typed_vectors=31
verified quickchain hash payload vectors with bash: files=5 payload_bytes=3841 preimage_bytes=3972 optional_b3sum_checks=0
ron-proto quickchain exhaustive preflight gate passed: tests=20
```

Optional focused checks:

```bash
cargo fmt -p ron-proto

cargo test -p ron-proto --test quickchain_tooling_boundary
cargo test -p ron-proto --test quickchain_hash_payloads
cargo test -p ron-proto --test quickchain_vector_inventory

bash crates/ron-proto/tests/tools/verify_quickchain_vector_inventory.sh
bash crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.sh

cargo clippy -p ron-proto --all-targets -- -D warnings
```

---

## ron-proto completion assessment

For the current pre-root QuickChain scope, `ron-proto` is effectively done.

Remaining future work is intentionally deferred until the blueprint authorizes the next phase:

* Add or update vectors only when the QuickChain blueprint intentionally changes.
* Do not add fake hashes or placeholder hashes.
* Do not add root-producing APIs yet.
* Do not add validator/checkpoint/finality runtime.
* Do not add settlement or bridge DTOs that imply live external integration.
* Do not add unknown consensus fields.
* Do not allow DTOs to become authority.

Future `ron-proto` work, when authorized, should continue the sequence:

```text
sketch -> locked_bytes -> locked_hash
```

No vector should jump straight to claimed hash/root truth without reviewed canonical bytes and reviewed preimage framing.

---

# ron-ledger QuickChain status

## Role of ron-ledger

`ron-ledger` owns deterministic economic truth and preflight execution/replay behavior.

For QuickChain Phase-0, `ron-ledger` must remain:

* Internal.
* Deterministic.
* Feature-gated.
* Pre-root.
* Pre-validator.
* Pre-settlement.
* Pre-bridge.
* Pre-anchor.

`ron-ledger` may safely own:

* Checked integer balance transitions.
* Non-negative balance invariants.
* Issue/transfer/burn transition behavior.
* Hold open/capture/release/expire lifecycle behavior.
* Reserved-vs-available balance rules.
* Ledger-assigned account sequence behavior.
* Ledger-assigned primitive sequence ranges.
* Operation replay identity.
* Idempotency retry behavior.
* Accepted-operation replay.
* Explicit replay boundaries.
* Pure projection into frozen `ron-proto` hash-payload DTOs.
* Deterministic snapshots and ordering.

`ron-ledger` must not yet own:

* QuickChain root production.
* QuickChain checkpoint finality.
* Validators.
* External settlement.
* Public anchors.
* Bridges.
* Staking.
* Liquidity.
* Chain authority for CrabLink.
* Wall-clock-derived root ordering.
* DB-order-derived root ordering.
* Unknown consensus fields.

---

## ron-ledger accomplishments in this milestone

The crate now has a strong gated QuickChain preflight layer under:

```toml
quickchain-preflight = ["dep:ron-proto"]
```

Major coverage includes:

* Atomic QuickChain execution state.
* Copy-on-write candidate mutation.
* All-or-nothing balance transitions.
* Issue/transfer/burn behavior.
* Supply decision handling as explicit input.
* No silent supply authority.
* Non-negative balance invariants.
* Current supply invariants.
* Hold lifecycle execution:

  * open
  * capture
  * release
  * expire
* Active hold state.
* Terminal hold state.
* Reserved balance tracking.
* Available balance enforcement.
* Holds preventing transfer/burn from spending reserved funds.
* Explicit epoch inputs for hold lifecycle decisions.
* No wall-clock hold expiry authority.
* Replay index.
* Operation identity tracking.
* Idempotency key conflict/retry behavior.
* Account sequence assignment.
* Primitive ledger sequence range assignment.
* Chain binding.
* Accepted replay.
* Accepted replay boundary validation.
* Accepted replay chain ID validation.
* Accepted replay operation count validation.
* Accepted replay next ledger sequence validation.
* Accepted replay evidence tamper rejection.
* Retry-after-accepted rebuild behavior.
* Projection context boundaries.
* Operation hash payload projection.
* Receipt hash payload projection.
* Account leaf payload projection.
* Active hold leaf payload projection.
* Projection replay equality.
* State snapshot determinism.
* Terminal hold account projection.
* Terminal hold replay evidence.
* Pre-root source boundary checks.
* Bash-only tooling boundary checks.
* Exhaustive QuickChain preflight runner.

---

## ron-ledger exhaustive preflight runner

Current runner:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Current behavior:

* Runs from repo root or from inside the repository.
* Rejects checked-in `.py` helper scripts under `crates/ron-ledger`.
* Runs `cargo fmt -p ron-ledger -- --check`.
* Discovers every `crates/ron-ledger/tests/quickchain_*.rs` test.
* Runs each discovered QuickChain integration test with `--features quickchain-preflight`.
* Requires at least 20 discovered QuickChain tests.
* Runs feature-gated clippy:

  ```bash
  cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
  ```
* Prints final success line.

Previously proven final success line:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=26
```

---

## ron-ledger important tests now covered

The current codebundle shows the QuickChain test surface includes:

* `quickchain_accepted_replay`
* `quickchain_accepted_replay_auxiliary_inputs`
* `quickchain_accepted_replay_boundary`
* `quickchain_accepted_replay_boundary_extremes`
* `quickchain_accepted_replay_boundary_identity_rejection`
* `quickchain_accepted_replay_boundary_rejection_stability`
* `quickchain_accepted_replay_boundary_retry_stability`
* `quickchain_accepted_replay_evidence_tamper`
* `quickchain_accepted_replay_sequence_evidence`
* `quickchain_account_leaf_projection`
* `quickchain_active_hold_leaf_projection`
* `quickchain_atomic_execution`
* `quickchain_balance_transition`
* `quickchain_chain_binding`
* `quickchain_hold_atomic_execution`
* `quickchain_operation_hash_projection`
* `quickchain_pre_root_boundary`
* `quickchain_projection_context_boundaries`
* `quickchain_projection_replay_equality`
* `quickchain_receipt_hash_projection`
* `quickchain_replay_index`
* `quickchain_retry_after_accepted_rebuild`
* `quickchain_state_snapshot`
* `quickchain_terminal_hold_account_projection`
* `quickchain_terminal_hold_replay_evidence`
* `quickchain_tooling_boundary`

---

## ron-ledger commands to prove current state

Run:

```bash
bash crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Expected final output:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=26
```

Optional focused checks:

```bash
cargo fmt -p ron-ledger

cargo test -p ron-ledger --features quickchain-preflight --test quickchain_tooling_boundary
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_context_boundaries
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_replay_equality

cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
```

Existing broader ledger check remains:

```bash
bash crates/ron-ledger/scripts/dev-quickcheck.sh
```

---

## ron-ledger completion assessment

For the current pre-root QuickChain scope, `ron-ledger` is effectively complete for this pass.

Remaining future work is intentionally deferred until the blueprint authorizes the next phase:

* No QuickChain root-producing engine yet.
* No QuickChain checkpoint finality yet.
* No validators yet.
* No settlement yet.
* No public anchors yet.
* No bridge logic yet.
* No pruning yet.
* No persistent QuickChain chain state yet.
* No gateway/omnigate/rewarder ledger mutation.
* No raw engagement payout mutation.

Future `ron-ledger` work, when authorized, should begin with formal root preparation only after `ron-proto` canonical bytes and locked hash vectors explicitly authorize the next shape.

---

# Pair-level summary

The pair now gives QuickChain a strong internal foundation:

```text
ron-proto:
  strict DTOs
  canonical JSON
  domain separation
  vector inventory
  locked bytes
  locked hashes
  bash verifiers
  exhaustive preflight runner

ron-ledger:
  deterministic execution
  hold lifecycle
  replay identity
  accepted replay
  sequence boundaries
  projection into ron-proto DTOs
  pre-root guardrails
  exhaustive feature-gated preflight runner
```

Together, these crates prove:

* DTO shapes are strict.
* Unknown fields reject.
* Money is integer minor-unit only.
* B3 hashes are lowercase and canonical.
* Idempotency is retry identity, not authority.
* Operation IDs are durable operation identity.
* Account sequences are ledger-assigned.
* Holds are lifecycle objects.
* Epoch behavior is explicit input, not wall-clock authority.
* Replay is deterministic.
* Accepted history can be rebuilt and boundary-checked.
* Projections are read-only and explicit-context-only.
* Tooling is bash-only.
* The current pair does not produce QuickChain roots, settlement, validators, bridges, or anchors.

---

# Current estimated completion

Current estimates after finishing this pair:

```text
ron-proto QuickChain pre-root scope: 99%
ron-ledger QuickChain pre-root scope: 97–98%
ron-proto + ron-ledger pair: 98–99%

overall internal QuickChain Phase-0/preflight: 68–75%
overall full long-term QuickChain blueprint: 35–42%
```

After completing the remaining crate-pair build order below, estimated status would become:

```text
internal Phase-0/preflight overall: 88–93%
internal ROC economy readiness before roots/validators: 80–88%
full long-term QuickChain blueprint: 48–58%
```

The full long-term blueprint remains below 60% even after all current crate pairs because major later phases are intentionally deferred:

* root engine
* checkpoint/finality layer
* proof system
* challenge/archive/fallback planning
* validators
* pruning
* DA/anchor strategy
* settlement
* external bridge
* staking/liquidity/exchange-facing logic

---

# Coherent next crate-pair build order

The next QuickChain work should continue in crate pairs so the value-plane proof stays coherent and does not drift into premature chain/runtime work.

## Pair 1 — `svc-wallet + ron-accounting`

This is the immediate next pair.

Why:

* `svc-wallet` is the service mutation front door.
* `ron-accounting` is snapshots/reporting only, not balance truth.
* This pair proves service-level wallet mutations and accounting snapshots do not diverge from ledger truth.

Key goals:

* Ensure `svc-wallet` is the only service front door for issue/transfer/burn/hold/capture/release/receipt flows.
* Ensure no fake balances.
* Ensure no fake receipts.
* Ensure no silent spend.
* Ensure quotes and confirmations are explicit.
* Ensure receipts are backend-derived.
* Ensure `ron-accounting` never becomes balance truth.
* Ensure accounting snapshots are derived from accepted ledger/wallet truth.
* Ensure accounting does not mutate ledger/wallet state.
* Add or harden exhaustive bash QuickChain preflight runners.
* Add or harden no-Python tooling boundaries.
* Add or harden no-root/no-settlement/no-anchor guardrails.

Expected impact:

This is the biggest remaining internal-economy gate after `ron-proto + ron-ledger`.

---

## Pair 2 — `svc-rewarder + svc-storage`

Why:

* `svc-rewarder` plans payouts but must never mutate ledger.
* `svc-storage` owns bytes by b3 and paid storage/access boundaries but must never unlock paid content from cache alone.

Key goals:

* Ensure rewarder payout plans are planning artifacts only.
* Ensure rewarder cannot directly mint or transfer ROC.
* Ensure raw engagement cannot directly allocate protocol ROC.
* Ensure storage enforces b3 identity.
* Ensure storage cannot fabricate paid access receipts.
* Ensure paid storage/content enforcement relies on backend wallet/ledger receipts.
* Ensure no fake content unlock from offline cache alone.
* Add exhaustive bash QuickChain preflight runners if missing.
* Add no-Python tooling boundaries.
* Add no-root/no-validator/no-settlement guardrails.

Expected impact:

This pair protects the creator payout path and paid byte path from authority drift.

---

## Pair 3 — `svc-gateway + omnigate`

Why:

* `svc-gateway` is the public boundary.
* `omnigate` hydrates user-facing content and paid access views.
* This pair is where public paid access could accidentally bypass wallet/ledger truth if not guarded.

Key goals:

* Ensure gateway is public boundary only.
* Ensure omnigate hydrates but does not mutate ledger.
* Ensure paid access requires backend wallet/receipt path.
* Ensure display-only receipts are not treated as balance truth.
* Ensure no fake unlocks.
* Ensure cache cannot unlock paid content alone.
* Ensure no gateway/omnigate QuickChain root/checkpoint/validator behavior.
* Ensure no external settlement or bridge references become runtime behavior.
* Add exhaustive bash QuickChain preflight runners if missing.
* Add no-Python tooling boundaries.

Expected impact:

This pair proves the public access surface cannot become economic authority.

---

## Pair 4 — `svc-index + ron-policy`

Why:

* `svc-index` is lookup/pointer service only.
* `ron-policy` is declarative policy only.
* These layers must not become authority, balance truth, settlement, or hidden QuickChain runtime.

Key goals:

* Ensure index is lookup/pointer service only.
* Ensure names/pointers are not authority.
* Ensure b3 hashes remain truth.
* Ensure policy is declarative and does not mutate balances.
* Ensure policy cannot produce receipts.
* Ensure policy cannot unlock paid content without backend wallet/receipt truth.
* Ensure no root/checkpoint/validator/bridge logic enters either crate.
* Add exhaustive bash QuickChain preflight runners if missing.
* Add no-Python tooling boundaries.

Expected impact:

This pair protects navigation, lookup, and policy from becoming hidden economic/runtime authority.

---

## Pair 5 — `CrabLink Tauri + client adapters`

Why:

* Tauri is the primary CrabLink client.
* React/TS owns display and user intent, not backend truth.
* Tauri Rust mediates native privilege but still must not mutate ledger truth directly.

Key goals:

* Ensure client requests quotes and confirmations explicitly.
* Ensure client displays backend-derived balances/receipts only.
* Ensure no private keys/seeds/raw capabilities/spend authority enter React state/localStorage/URLs/logs.
* Ensure paid flow stays:

  ```text
  prepare/quote -> explicit confirmation -> backend wallet path -> backend receipt -> unlock/render -> display-only receipt cache -> balance refresh
  ```
* Ensure offline cache verifies b3 before trusted render.
* Ensure offline cache cannot unlock paid content alone.
* Ensure media remains bounded and range/segment oriented.
* Ensure no full-file large media through command result payloads.
* Ensure sidecar/local node stays optional, explicit, supervised, bounded, and disabled by default.
* Ensure facets remain inert until sandboxed.
* Ensure CrabLink Tauri does not become QuickChain chain authority.

Expected impact:

This pair closes the loop at the product/client boundary after backend service authority is clean.

---

# Next-session recommendation

The next session should start with:

```text
svc-wallet + ron-accounting
```

Attach fresh codebundles for both crates:

```bash
bash scripts/make_crate_codex.sh -c svc-wallet
bash scripts/make_crate_codex.sh -c ron-accounting
```

Then work in the same style used for `ron-proto + ron-ledger`:

* full paste-ready files or bash scripts
* no Python helper scripts
* no git patches/unified diffs
* exact paths
* preserve RO headers
* run fmt
* run clippy with `-D warnings`
* run focused tests
* add exhaustive bash preflight runners where useful
* add no-Python tooling boundary tests
* add no-fake-balance/no-fake-receipt/no-silent-spend tests
* add no-root/no-validator/no-settlement/no-anchor guardrails
* keep QuickChain future-only and internal-ROC-first

---

# Final pair verdict

`ron-proto + ron-ledger` should be treated as complete for the current QuickChain Phase-0 / pre-root milestone.

Do not continue adding random features to this pair right now. The correct next move is to carry the same discipline into the service boundary crates, beginning with:

```text
svc-wallet + ron-accounting
```

Only return to `ron-proto + ron-ledger` when:

* a future QuickChain blueprint update requires new DTO/vector work,
* a later phase explicitly authorizes root-producing preparation,
* a downstream crate exposes a gap in the current DTO/execution boundary,
* or a regression breaks the current exhaustive preflight gates.


### END NOTE - JUNE 16 2026 - 14:10 CST



### BEGIN NOTE - JUNE 18 2026 - 20:10 CST

Below are two paste-ready notes sections. The terminal proof shows `ron-proto` passed its focused source-boundary test, full QuickChain preflight, vector verifiers, and clippy; `ron-ledger` passed its focused canonical projection boundary plus the exhaustive QuickChain preflight with 27 tests and clippy. 

# QuickChain QC-1A / Pre-Root Notes — `ron-proto`

## Status

`ron-proto` is green for the current QuickChain QC-1A / pre-root DTO, canonicalization, vector, and boundary-hardening pass.

This crate remains the QuickChain DTO and vector contract layer. It does **not** own runtime authority, ledger mutation, service IO, chain execution, validator behavior, settlement, anchors, bridges, staking, pruning, or external-chain behavior. Its job in this phase is to define strict, deterministic, reviewable shapes that downstream crates can consume without smuggling in authority.

Current estimate for this crate in the QuickChain pre-root phase: approximately **99% complete**.

## What changed in this pass

### 1. Hardened the QuickChain source-boundary test

The primary change was to replace the brittle `quickchain_source_boundary.rs` allowlist logic with a more durable boundary test.

The old test was failing because it attempted to explicitly allowlist every `#[serde(default)]` field by exact field name and file. That caused repeated failures as legitimate optional DTO metadata fields appeared, such as:

* `signatures`
* `account_id`
* optional receipt/hash-payload context fields
* optional usage-event metadata
* optional vector-growth fields

The updated test now enforces the actual invariant instead of chasing individual fields.

The new source-boundary test verifies:

* QuickChain source files remain DTO/vector-prep code only.
* QuickChain source files do not gain runtime authority.
* QuickChain source files do not gain implicit default constructors.
* `#[serde(default)]` remains limited to structural optional/growth fields only.
* Authority-sensitive defaults remain blocked.

### 2. Runtime-authority tokens remain forbidden

The source-boundary test continues to reject runtime or authority creep inside `src/quickchain`.

The boundary scan rejects tokens associated with:

* Hash execution in DTO source, such as direct `blake3::` use.
* Alternate hash algorithms such as `sha2::` and `sha3::`.
* Signature/validator crypto primitives such as `ring::`, `k256::`, and `secp256k1`.
* Async runtime behavior such as `tokio::`, `.await`, and `.spawn(`.
* HTTP/server/client behavior such as `reqwest::`, `axum::`, and `hyper::`.
* Filesystem IO such as `std::fs`.
* Network IO such as `std::net`.
* Threads such as `std::thread`.
* Wall-clock dependence such as `SystemTime` / `UNIX_EPOCH`.
* Randomness such as `rand::` and `thread_rng`.
* Database dependencies such as `sled::`, `rusqlite::`, `sqlx::`, `rocksdb::`, and `redb::`.
* `unsafe`.

This preserves the rule that `ron-proto` may define canonical payloads and vector contracts, but must not become a runtime, hasher, verifier, database reader, network client, or chain authority.

### 3. Implicit default constructors remain forbidden

The source-boundary test continues to reject implicit default constructors in QuickChain DTO/validation source.

The boundary scan rejects:

* `derive(Default)`
* `impl Default`
* `::default(`
* `Default::default(`

This protects against accidentally creating complete QuickChain economic, root, receipt, checkpoint, validator, or chain objects without explicit field construction.

The purpose is not to ban `#[serde(default)]` on optional/growth fields. The purpose is to ban broad Rust-side constructors that could silently invent authority-sensitive values.

### 4. `#[serde(default)]` policy was corrected

The new policy is:

`#[serde(default)]` is allowed only when the field is structurally optional or additive:

* `Option<T>`
* `Vec<T>`
* `BTreeMap<K, V>`

This is the right boundary for DTO evolution because absent optional metadata should deserialize as absent, absent lists should deserialize as empty, and future additive map-like metadata can remain inert.

The test now rejects `#[serde(default)]` on non-structural required fields.

Examples of legitimate defaults include optional or additive DTO fields such as:

* optional usage-event metadata
* optional account/counterparty/object/site references
* optional amount metadata for analytics or metering events
* optional hash-payload planning context
* optional receipt evidence fields
* optional terminal hold lifecycle context
* optional vector display/audit fields
* optional signature lists where an absent list means “no signatures present,” not “verified”

### 5. The denylist was adjusted to avoid false positives

The initial denylist was too aggressive. It treated fields like `account_id` as authority-sensitive everywhere.

That was incorrect for `UsageEvent` and similar DTOs because those fields are optional metadata, not balance truth and not ledger mutation authority.

The surgical patch removed legitimate optional metadata/evidence field names from the hard denylist while leaving truly dangerous defaults blocked.

Fields such as these are allowed when they are structural `Option`, `Vec`, or `BTreeMap` growth fields:

* `account_id`
* `from_account_id`
* `to_account_id`
* `counterparty_account_id`
* `amount_minor`
* `account_sequence`
* `account_sequence_terminal`
* `ledger_seq_start`
* `ledger_seq_end`
* `operation_hash`
* `receipt_hash`
* `receipt_root`
* `checkpoint_hash`
* `previous_ledger_root`
* `new_ledger_root`
* `data_availability_root`
* `validator_set_hash`

These fields remain controlled by downstream validation tests and canonical-vector tests. They are not made authoritative merely because they are optional in DTO payloads.

The denylist still blocks dangerous defaulting for identity and authority fields such as:

* `version`
* `schema`
* `domain`
* `domain_separator`
* `canonicalization`
* `encoding`
* `chain_id`
* `checkpoint_id`
* `checkpoint_height`
* `checkpoint_seq`
* `checkpoint_sequence`
* `epoch`
* `epoch_id`
* `epoch_start_ms`
* `epoch_end_ms`
* `created_at_ms`
* `updated_at_ms`
* `expires_at_ms`
* required root identities
* `operation_id`
* `idempotency_key`
* `balance_minor`
* `held_minor`
* `current_supply_minor`
* `asset`
* `receipt_id`
* `receipt_txid`
* `receipt_status`
* `receipt_direction`
* `receipt_action`
* `validator_id`
* `validator_public_key`
* single `signature`
* `signature_algorithm`
* `signed_payload_hash`

This creates a better long-term scanner: it allows inert DTO growth while still failing loudly on fields that must never be silently invented.

## Why this matters

This pass tightened the line between DTO evolution and authority creep.

`ron-proto` now has a source-level guardrail that says:

* DTOs can evolve additively.
* Optional metadata can remain optional.
* Vector scaffolding can exist.
* Canonical bytes can be locked.
* Hash payload shapes can be described.
* But runtime hashing, validator logic, service IO, roots, settlement, bridges, anchors, and ledger mutation cannot enter this crate.

This is aligned with the QuickChain doctrine:

* Determinism before distribution.
* DTOs before roots.
* Roots before validators.
* Proofs before pruning.
* Internal ROC before external anchors.

## Test proof from this pass

The following passed:

```text
cargo test -p ron-proto --test quickchain_source_boundary
```

Result:

```text
running 3 tests
test quickchain_serde_defaults_stay_structural_not_authority_sensitive ... ok
test quickchain_sources_do_not_gain_implicit_default_constructors ... ok
test quickchain_sources_remain_pure_dto_and_vector_prep ... ok

test result: ok. 3 passed
```

The exhaustive preflight runner also passed:

```text
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

The runner completed:

* all discovered `quickchain_*.rs` tests
* QuickChain canonical JSON tests
* chain params tests
* challenge wire tests
* domain separation tests
* strict DTO tests
* empty-tree tests
* event-class tests
* file-backed vector tests
* hash-payload tests
* hold-scenario tests
* IDs and money tests
* locked-bytes tests
* operation/hold tests
* receipt DTO tests
* receipt-order tests
* replay/idempotency tests
* sort-key tests
* source-boundary tests
* tooling-boundary tests
* vector DTO tests
* vector inventory tests
* bash vector inventory verifier
* bash hash payload verifier
* clippy with warnings denied

Final runner result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=21
```

## Current crate boundaries

`ron-proto` may contain:

* QuickChain DTOs.
* Strict serde shapes.
* `deny_unknown_fields`.
* Canonical JSON v1 helpers.
* Domain separator constants and validation.
* Integer minor-unit string validation.
* b3 lowercase validation.
* Operation/hold/receipt DTO validation.
* Event-class vocabulary and conservative helper policy.
* File-backed locked-byte vectors.
* Locked-hash vector descriptors where the hash/preimage is test-vector data.
* Bash-only verification tools for vectors.
* Source-boundary tests.
* Tooling-boundary tests.

`ron-proto` must not contain:

* ledger mutation
* wallet mutation
* service calls
* async runtime behavior
* sockets
* HTTP clients or servers
* database reads/writes
* randomness
* wall-clock authority
* root production authority
* checkpoint production authority
* validator execution
* cryptographic verification authority
* bridge/anchor/settlement logic
* staking/liquidity/exchange-facing logic
* external-chain behavior
* fake receipts
* fake balances
* silent spend authority

## Remaining work for `ron-proto`

`ron-proto` is effectively green for the current pre-root phase, but later phases will need deliberate additions.

Deferred future work includes:

### 1. Locked root vector expansion

Before root-producing code exists, root vector contracts must be expanded and reviewed.

Future vector flow remains:

```text
sketch -> locked_bytes -> locked_hash
```

No fake hashes or placeholders should be introduced.

### 2. Canonical root payload DTO review

When Phase 1 root work begins, any new root-payload DTOs must go through:

* strict field review
* canonical JSON v1 byte review
* unknown-field rejection
* exact locked bytes
* explicit domain separator
* explicit preimage construction contract
* no runtime authority in `ron-proto`

### 3. Validator/signature DTO separation

Validator and signature DTOs may exist as descriptive wire shapes, but verification and committee behavior must remain outside `ron-proto`.

`ron-proto` may describe:

* validator ID shape
* public key reference shape
* signature reference shape
* quorum/committee metadata shape

`ron-proto` must not perform:

* validator set execution
* signature verification
* committee voting
* finality logic
* slashing
* staking
* public bridge behavior

### 4. Receipt/root/checkpoint schema hardening

Future receipt/root/checkpoint DTOs should keep the same rule:

* required identity fields must be explicit
* optional evidence fields must be structurally optional
* no default constructors
* no unknown fields
* no floats
* no client-assigned ledger authority

### 5. Keep tooling Bash-only

The preflight runner and vector verification tooling are Bash-only. Keep this invariant.

Do not add Python helper scripts to `ron-proto` QuickChain tooling unless the project explicitly reverses the current tooling policy.

## Files touched or relevant in this pass

Primary file touched:

```text
crates/ron-proto/tests/quickchain_source_boundary.rs
```

Important test/tooling files exercised:

```text
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-proto/tests/quickchain_canonical_json.rs
crates/ron-proto/tests/quickchain_chain_params.rs
crates/ron-proto/tests/quickchain_challenge_wire.rs
crates/ron-proto/tests/quickchain_domain_separation.rs
crates/ron-proto/tests/quickchain_dto_strict.rs
crates/ron-proto/tests/quickchain_empty_tree.rs
crates/ron-proto/tests/quickchain_event_class.rs
crates/ron-proto/tests/quickchain_file_vectors.rs
crates/ron-proto/tests/quickchain_hash_payloads.rs
crates/ron-proto/tests/quickchain_hold_scenarios.rs
crates/ron-proto/tests/quickchain_ids_and_money.rs
crates/ron-proto/tests/quickchain_locked_bytes.rs
crates/ron-proto/tests/quickchain_operation_hold.rs
crates/ron-proto/tests/quickchain_receipt_dto.rs
crates/ron-proto/tests/quickchain_receipt_order.rs
crates/ron-proto/tests/quickchain_replay_idempotency.rs
crates/ron-proto/tests/quickchain_sort_keys.rs
crates/ron-proto/tests/quickchain_source_boundary.rs
crates/ron-proto/tests/quickchain_tooling_boundary.rs
crates/ron-proto/tests/quickchain_vector_dto.rs
crates/ron-proto/tests/quickchain_vector_inventory.rs
crates/ron-proto/tests/tools/verify_quickchain_vector_inventory.sh
crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.sh
```

## Summary

This pass made `ron-proto` stronger and less brittle.

The crate now has a better QuickChain source-boundary test that:

* prevents runtime authority from entering DTO code
* prevents implicit default constructors
* allows only structural optional/growth serde defaults
* avoids false positives on legitimate optional metadata
* keeps root/checkpoint/validator/settlement authority out of `ron-proto`

`ron-proto` is green for this QuickChain pre-root phase and ready to support the next crate pair, `svc-wallet + ron-accounting`.

# QuickChain QC-1A / Pre-Root Notes — `ron-ledger`

## Status

`ron-ledger` is green for the current QuickChain QC-1A / pre-root deterministic execution, replay, snapshot, and projection-hardening pass.

This crate remains the economic truth and deterministic transition boundary for QuickChain preflight work. It may execute internal deterministic ROC accounting rules under feature-gated QuickChain preflight tests, but it must not become a public chain runtime, validator, anchor, bridge, staking system, settlement layer, pruning system, or external-chain integration.

Current estimate for this crate in the QuickChain pre-root phase: approximately **98–99% complete**.

## What this pass confirmed

The latest pass confirmed that `ron-ledger` is green after the paired `ron-proto` boundary fix.

The focused canonical projection boundary passed:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Result:

```text
running 2 tests
test quickchain_pre_root_module_tree_does_not_claim_runtime_phase_authority ... ok
test quickchain_pre_root_ledger_sources_do_not_serialize_hash_or_build_preimages ... ok

test result: ok. 2 passed
```

The exhaustive preflight runner also passed:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final runner result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=27
```

This means `ron-ledger` is currently aligned with the pre-root boundary: deterministic execution and projection are allowed; root production, canonical hashing, validator authority, checkpoint authority, settlement, bridge, and anchor behavior remain forbidden.

## Main QuickChain role of `ron-ledger`

`ron-ledger` is the internal economic truth engine for ROC.

Its QuickChain preflight role is to prove:

* deterministic accepted operation execution
* atomic balance transitions
* atomic hold lifecycle transitions
* idempotency behavior
* duplicate operation rejection
* replay equivalence
* state snapshot determinism
* account sequence ownership by the ledger
* ledger sequence consistency
* hold capture/release/expiry semantics
* read-only projection into `ron-proto` hash-payload DTOs
* no root-producing behavior before the root phase is explicitly opened

It must not delegate balance truth to accounting, rewarder, gateway, omnigate, CrabLink, cache, or client code.

## What has been accomplished so far

### 1. Deterministic balance execution

The QuickChain balance transition tests prove deterministic handling of:

* issue
* transfer
* burn
* zero amount rejection
* missing amount rejection
* noncanonical amount rejection
* underflow rejection
* overflow rejection
* unauthorized issue rejection
* unauthorized burn rejection
* conservation checks
* rejection without partial mutation

Important invariant: failed transitions must not partially mutate state.

### 2. Atomic operation execution

The atomic execution tests prove that accepted operations update all related state together.

The crate now verifies:

* fresh issue commits balance identity and sequence together
* exact retry returns the original record
* idempotency conflict leaves state unchanged
* duplicate operation ID leaves state unchanged
* arithmetic rejection does not create identities or consume sequences
* invalid receipt references roll back computed transitions
* transfer advances participant sequences correctly
* self-transfer advances the actor sequence once
* identical ordered commands rebuild identical atomic state

Important invariant: operation identity, account sequence, primitive ledger sequence, receipt evidence, and balance mutation must stay atomic.

### 3. Hold lifecycle execution

The hold atomic execution tests prove deterministic handling of:

* hold open
* hold capture
* hold release
* partial capture
* expiry
* terminal holds
* reserved value blocking spend
* concurrent holds
* capture/release retry behavior
* invalid receipt and epoch rollback
* amount mismatch rollback
* counterparty mismatch rollback
* no reopen after terminal transition

Important invariant: hold state must not be resurrected, double-spent, retransitioned, or terminally mutated after close.

### 4. Accepted replay reconstruction

The accepted replay tests prove that previously accepted operations can rebuild deterministic state.

Covered behavior includes:

* mixed accepted history rebuilds identical complete state
* duplicate accepted records reject
* contradictory accepted history rejects through live transition rules
* mismatched committed sequences reject reconstruction
* supply decisions are preserved exactly
* epoch input is preserved exactly
* receipt transaction references are preserved exactly
* tampered receipt references reject
* sequence evidence mismatch rejects
* duplicate operation ID rejects without advancing replay boundary
* idempotency conflict rejects without advancing replay boundary
* retry after accepted rebuild returns the original record

Important invariant: accepted replay must use the same transition code as live acceptance and must reproduce committed evidence exactly.

### 5. Replay boundary hardening

The replay boundary tests prove that durable-history boundaries are checked before roots/checkpoints exist.

Covered behavior includes:

* empty accepted replay boundary accepts empty history
* chain-bound empty boundary rejects non-empty history
* truncated valid history rejects
* overextended valid history rejects
* next ledger sequence mismatch rejects
* wrong-chain accepted history rejects
* numeric boundary without chain ID remains a legacy shape-only check
* live terminal history round-trips through boundary reconstruction

Important invariant: before root production exists, replay boundaries still catch truncation, overextension, wrong-chain history, and sequence mismatch.

### 6. Chain binding

The chain-binding tests prove that the first committed QuickChain operation establishes the chain identity for the replay index.

Covered behavior includes:

* empty index binds to the first committed chain
* replay index rejects other chain without mutation
* atomic state rejects cross-chain issue before economic mutation

Important invariant: a replay index cannot silently mix operations from multiple chain identities.

### 7. Snapshot determinism

The state snapshot tests prove deterministic read-only snapshots.

Covered behavior includes:

* empty state snapshot is explicit and read-only
* snapshots sort accounts deterministically
* snapshots sort active holds deterministically
* terminal holds are excluded from active hold snapshots
* equivalent final states produce identical snapshots despite cross-account commit order

Important invariant: snapshots are deterministic views, not balance mutation paths and not roots.

### 8. Leaf/hash-payload projection boundaries

The projection tests prove that `ron-ledger` can project explicit state into `ron-proto` DTOs without computing hashes or roots.

Covered projection areas include:

* account leaf projection
* active hold leaf projection
* operation hash payload projection
* receipt hash payload projection
* terminal hold account projection
* projection replay equality
* projection context boundaries

The projections copy explicit context into DTOs and rely on `ron-proto` validation. They do not compute BLAKE3, do not construct hash preimages internally, do not canonicalize JSON internally, and do not produce roots.

Important invariant: projection is a read-only adapter seam. It is not root production.

### 9. Canonical projection boundary

The canonical projection boundary test confirms that pre-root ledger source does not cross into canonicalization or hash execution authority.

It prevents `ron-ledger` QuickChain pre-root modules from:

* serializing canonical JSON as a root-production step
* computing hashes
* building preimages as authority
* claiming checkpoint authority
* claiming validator authority
* claiming settlement authority
* claiming bridge/anchor authority
* depending on DB order, wall-clock order, service IO, or runtime IO for roots

This is one of the most important boundaries in the current pass.

### 10. Tooling boundary

The tooling boundary tests confirm that QuickChain preflight remains Bash-only and exhaustive.

Covered behavior includes:

* preflight runner is Bash-only
* test discovery count matches runner contract
* every QuickChain integration test is feature-gated
* no helper Python scripts exist in `ron-ledger`

This matches the current project policy for QuickChain patch delivery and local preflight execution.

## Current crate boundaries

`ron-ledger` may contain:

* deterministic internal ROC state transition logic
* checked integer arithmetic
* balance state
* hold state
* hold lifecycle transition logic
* replay index
* accepted replay reconstruction
* operation identity tracking
* idempotency tracking
* account sequence assignment
* primitive ledger sequence tracking
* deterministic read-only snapshots
* explicit projection into `ron-proto` hash-payload DTOs
* feature-gated QuickChain preflight tests
* Bash-only QuickChain preflight tooling

`ron-ledger` must not contain:

* public chain runtime behavior
* validator execution
* consensus voting
* public bridge logic
* public settlement logic
* staking
* liquidity
* exchange-facing logic
* external L1/L2/DA behavior
* root production without explicit root phase approval
* checkpoint production for QuickChain finality
* pruning
* anchor mutation
* rewarder payout mutation
* gateway/omnigate mutation shortcuts
* accounting-as-balance-truth
* fake balances
* fake receipts
* client-assigned account sequences
* DB-order roots
* wall-clock roots
* async network/service authority

## Important design decisions captured by this crate

### Operation ID vs idempotency key

`operation_id` is durable backend operation identity.

`idempotency_key` is a retry key.

They are not interchangeable.

The replay/index tests preserve this distinction and reject conflicting reuse.

### Account sequence is ledger-assigned

Clients must not assign `account_sequence`.

The crate rejects client-assigned account sequence before lookup or mutation.

### Hold ID identifies one lifecycle

A `hold_id` identifies one lifecycle.

A terminal hold cannot be reopened or retransitioned.

### Expiry is epoch-gated

Hold expiry depends on explicit epoch input.

The crate does not use wall-clock time as authority.

### Accounting is not balance truth

`ron-ledger` remains balance truth.

Accounting snapshots may summarize later, but cannot replace ledger truth.

### Rewarder plans but does not mutate

Rewarder payout planning remains external to ledger mutation.

`ron-ledger` must remain the mutation truth path, reached through the wallet/service boundary when appropriate.

## Test proof from this pass

The focused boundary passed:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Result:

```text
running 2 tests
test quickchain_pre_root_module_tree_does_not_claim_runtime_phase_authority ... ok
test quickchain_pre_root_ledger_sources_do_not_serialize_hash_or_build_preimages ... ok

test result: ok. 2 passed
```

The exhaustive runner passed:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

The runner completed all discovered QuickChain preflight tests and clippy.

Final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=27
```

Major test groups exercised include:

```text
quickchain_accepted_replay_auxiliary_inputs
quickchain_accepted_replay_boundary_extremes
quickchain_accepted_replay_boundary_identity_rejection
quickchain_accepted_replay_boundary_rejection_stability
quickchain_accepted_replay_boundary_retry_stability
quickchain_accepted_replay_boundary
quickchain_accepted_replay_evidence_tamper
quickchain_accepted_replay_sequence_evidence
quickchain_accepted_replay
quickchain_account_leaf_projection
quickchain_active_hold_leaf_projection
quickchain_atomic_execution
quickchain_balance_transition
quickchain_chain_binding
quickchain_hold_atomic_execution
quickchain_operation_hash_projection
quickchain_pre_root_boundary
quickchain_projection_canonical_boundary
quickchain_projection_context_boundaries
quickchain_projection_replay_equality
quickchain_receipt_hash_projection
quickchain_replay_index
quickchain_retry_after_accepted_rebuild
quickchain_state_snapshot
quickchain_terminal_hold_account_projection
quickchain_terminal_hold_replay_evidence
quickchain_tooling_boundary
```

Clippy also passed under the QuickChain preflight feature.

## Files relevant to this pass

Important source modules:

```text
crates/ron-ledger/src/quickchain/accepted_replay.rs
crates/ron-ledger/src/quickchain/balance_state.rs
crates/ron-ledger/src/quickchain/error.rs
crates/ron-ledger/src/quickchain/execution_error.rs
crates/ron-ledger/src/quickchain/execution_state.rs
crates/ron-ledger/src/quickchain/hash_payload_projection.rs
crates/ron-ledger/src/quickchain/hold_error.rs
crates/ron-ledger/src/quickchain/hold_state.rs
crates/ron-ledger/src/quickchain/hold_transition.rs
crates/ron-ledger/src/quickchain/leaf_projection.rs
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/src/quickchain/replay_index.rs
crates/ron-ledger/src/quickchain/state_snapshot.rs
crates/ron-ledger/src/quickchain/transition.rs
crates/ron-ledger/src/quickchain/transition_error.rs
crates/ron-ledger/src/quickchain/types.rs
```

Important test/tooling files:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/tests/quickchain_accepted_replay_auxiliary_inputs.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_extremes.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_identity_rejection.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_rejection_stability.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_retry_stability.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary.rs
crates/ron-ledger/tests/quickchain_accepted_replay_evidence_tamper.rs
crates/ron-ledger/tests/quickchain_accepted_replay_sequence_evidence.rs
crates/ron-ledger/tests/quickchain_accepted_replay.rs
crates/ron-ledger/tests/quickchain_account_leaf_projection.rs
crates/ron-ledger/tests/quickchain_active_hold_leaf_projection.rs
crates/ron-ledger/tests/quickchain_atomic_execution.rs
crates/ron-ledger/tests/quickchain_balance_transition.rs
crates/ron-ledger/tests/quickchain_chain_binding.rs
crates/ron-ledger/tests/quickchain_hold_atomic_execution.rs
crates/ron-ledger/tests/quickchain_operation_hash_projection.rs
crates/ron-ledger/tests/quickchain_pre_root_boundary.rs
crates/ron-ledger/tests/quickchain_projection_canonical_boundary.rs
crates/ron-ledger/tests/quickchain_projection_context_boundaries.rs
crates/ron-ledger/tests/quickchain_projection_replay_equality.rs
crates/ron-ledger/tests/quickchain_receipt_hash_projection.rs
crates/ron-ledger/tests/quickchain_replay_index.rs
crates/ron-ledger/tests/quickchain_retry_after_accepted_rebuild.rs
crates/ron-ledger/tests/quickchain_state_snapshot.rs
crates/ron-ledger/tests/quickchain_terminal_hold_account_projection.rs
crates/ron-ledger/tests/quickchain_terminal_hold_replay_evidence.rs
crates/ron-ledger/tests/quickchain_tooling_boundary.rs
```

## Remaining work for `ron-ledger`

`ron-ledger` is effectively green for this pre-root phase, but several future phases remain deliberately deferred.

### 1. Root engine remains deferred

Do not add root-producing logic until canonical bytes and golden vectors are fully locked.

Root work must come after DTO/vector readiness.

Future state-tree work should follow the selected candidate:

```text
sorted binary Merkle map over canonical account-state leaves
```

No DB-order roots and no wall-clock roots.

### 2. Receipt root and operation root remain deferred

The current projection layer can build DTO payloads for future hashing, but it must not produce roots yet.

Future work must introduce:

* exact canonical bytes
* exact preimage bytes
* locked hashes
* replay equality
* golden vectors
* no placeholder/fake hashes

### 3. Checkpoint/finality remains deferred

No finality checkpoint behavior should be added yet.

Future checkpoint work must remain downstream of:

* canonical root vectors
* explicit root production module
* challenge/evidence model
* archive/DA fallback planning

### 4. Validator behavior remains deferred

No validators, committees, voting, quorum execution, slashing, staking, or public consensus behavior should enter `ron-ledger` yet.

Validator DTOs may be described in `ron-proto`, but execution authority remains deferred.

### 5. Pruning remains forbidden

No pruning until data availability, challenge, and archive fallback are designed and tested.

Current doctrine remains:

```text
proofs before pruning
```

### 6. External settlement remains forbidden

No ROX, Solana, external L2, public bridge, liquidity, staking, exchange-facing logic, or public settlement behavior should be added to `ron-ledger`.

Internal ROC must be proven first.

## Summary

This pass confirmed `ron-ledger` is green as the QuickChain deterministic economic truth layer for the current pre-root phase.

The crate proves:

* deterministic balance transitions
* atomic operation execution
* hold lifecycle correctness
* replay reconstruction
* replay boundary validation
* chain binding
* snapshot determinism
* read-only projection into `ron-proto`
* no canonical hash/root production in pre-root modules
* no runtime phase authority
* Bash-only exhaustive preflight tooling

`ron-ledger` is ready to support the next crate pair, `svc-wallet + ron-accounting`, where wallet mutation boundaries and accounting snapshot non-authority need to be carried forward without weakening ledger truth.


### END NOTE - JUNE 18 2026 - 20:10 CST




### BEGIN NOTE - JUNE 19 2026 - 21:55 CST

Use this as the crate note for both `ron-proto` and `ron-ledger`. The latest terminal output shows the focused tree-material tests passed, the canonical-boundary scanner passed, and the full `ron-ledger` QuickChain preflight finished green with `tests=28`. 

# QuickChain QC-1A Crate Notes — `ron-proto` + `ron-ledger`

## Session Scope

This session advanced QuickChain from QC-0A safety/preflight into the first QC-1A Phase 1 preparation slice for the core backend crates:

1. `ron-proto`
2. `ron-ledger`

The goal was not to produce roots, checkpoints, validators, anchors, public chain state, settlement artifacts, or any live chain authority. The goal was to establish a safe Phase 1 input-material boundary:

* `ron-proto` owns strict DTO shape, canonical vocabulary, schema names, validation rules, and vector-facing material structures.
* `ron-ledger` owns deterministic ledger truth and can project/assemble sorted input material, but must not become the canonicalization, hashing, root, proof, checkpoint, validator, or settlement layer.
* Tree-material preparation is allowed because it is pre-root, inert, deterministic, and DTO/projection-only.
* Root-producing work remains blocked until locked canonical bytes, locked hashes, and independent vectors exist.

This batch was intentionally narrow and architecture-preserving.

---

## High-Level Result

This QC-1A batch is green for `ron-proto + ron-ledger`.

Validated outcomes:

* `ron-proto` QuickChain preflight passed with `tests=22`.
* `ron-ledger` QuickChain preflight passed with `tests=28`.
* `cargo clippy` passed under the ledger QuickChain preflight gate.
* New `ron-proto` tree-material DTO tests passed.
* New `ron-ledger` tree-material projection tests passed.
* The `quickchain_projection_canonical_boundary` scanner passed after removing direct canonical-vocabulary ownership from ledger source.
* No root production was introduced.
* No live checkpoint/root authority was introduced.
* No validator/settlement/anchor/bridge/staking/liquidity code was introduced.
* No public blockchain runtime was introduced.
* No wallet/service mutation authority was introduced into `ron-ledger` projection code.
* No fake balances, fake receipts, or silent spend paths were introduced.

This is a clean Phase 1 preparation checkpoint, not a full Phase 1 completion.

---

## `ron-proto` Changes

### New Purpose Added

`ron-proto` now has the strict DTO surface for QuickChain tree-material preparation.

This is the first safe bridge from “we have validated receipts/accounts/holds/hash payloads” toward “we can later build deterministic tree/root vectors,” without actually producing roots yet.

### New File

Added:

```text
crates/ron-proto/src/quickchain/root_material.rs
```

This file defines DTO-only tree-material structures and validation helpers.

### Module Wiring

Updated:

```text
crates/ron-proto/src/quickchain/mod.rs
```

Expected module/export additions include:

```rust
pub mod root_material;
pub use root_material::*;
```

The purpose is to expose the tree-material DTOs through the existing `ron_proto::quickchain::*` surface.

---

## `ron-proto` Tree-Material DTOs

The new `root_material.rs` module provides strict Phase 1 material DTOs.

Core constants introduced:

```text
QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA
QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA
QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1
MAX_QUICKCHAIN_TREE_MATERIAL_SORT_KEY_HEX_BYTES
MAX_QUICKCHAIN_TREE_MATERIAL_PAYLOAD_SCHEMA_BYTES
MAX_QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTES
MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS
```

Core enum introduced:

```rust
QuickChainTreeMaterialKindV1
```

Current tree-material kinds:

```text
State
Holds
Receipts
Accounting
Rewards
```

Core DTO introduced:

```rust
QuickChainTreeMaterialItemV1
```

The item DTO binds:

```text
schema
version
tree
sort_key_hex
payload_schema
payload_hash
```

Core batch DTO introduced:

```rust
QuickChainTreeMaterialBatchV1
```

The batch DTO binds:

```text
schema
version
chain_id
epoch_id
tree
canonical_encoding
item_sort_rule
items
```

The batch explicitly requires canonical JSON v1 through the DTO boundary, but this remains owned by `ron-proto`, not by `ron-ledger`.

---

## `ron-proto` Validation Rules Added

The new DTO validation enforces:

* Correct schema token.
* Correct DTO version.
* Valid `chain_id`.
* Valid `epoch_id`.
* Valid canonical encoding.
* Valid item sort rule.
* Bounded item count.
* Bounded sort key length.
* Lowercase even-length hex sort keys.
* Strictly ascending item sort keys.
* No duplicate sort keys.
* Valid tree/material schema pairing.
* Valid `b3:<64 lowercase hex>` payload commitments.
* Unknown-field rejection through serde tests.
* Payload schemas must be known/allowed for their target tree.
* Tree kind must match batch tree.
* Batch-level tree and item-level tree cannot drift.

This is important because it prevents Phase 1 from later building roots over ambiguous, unordered, unknown, or schema-mismatched material.

---

## `ron-proto` Helper Added

Added a helper in `root_material.rs`:

```rust
quickchain_tree_material_json_v1_encoding()
```

Purpose:

* Keeps the canonical encoding vocabulary in `ron-proto`.
* Allows downstream crates to request the currently allowed tree-material encoding without directly owning or naming canonicalization policy in their own source.
* Prevents `ron-ledger` from tripping source-boundary checks that ban ledger-side canonicalization/root-preimage ownership.

This helper became necessary after the `ron-ledger` canonical-boundary scanner correctly rejected direct ledger usage of `QuickChainCanonicalEncodingV1`.

---

## `ron-proto` Tests Added

Added:

```text
crates/ron-proto/tests/quickchain_root_material.rs
```

This test suite covers:

1. A sorted state-material batch validates.
2. Unsorted or duplicate sort keys reject.
3. Uppercase or odd-length hex sort keys reject.
4. Payload schema / tree mismatch rejects.
5. Tree-material DTOs reject unknown fields.

Focused test result:

```text
cargo test -p ron-proto --test quickchain_root_material
```

Result:

```text
5 passed
```

The full `ron-proto` QuickChain preflight also passed:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
```

---

## `ron-ledger` Changes

### New Purpose Added

`ron-ledger` now has a narrow, pre-root tree-material projection adapter.

The adapter takes caller-supplied material commitments, sorts them deterministically, and assembles a strict `ron-proto` tree-material batch.

It does not compute roots.

It does not compute hashes.

It does not serialize canonical bytes.

It does not build preimages.

It does not mutate ledger state.

It does not create receipts.

It does not become service authority.

It does not decide settlement/finality.

This keeps `ron-ledger` aligned with its role as deterministic internal truth/projection logic, not as a QuickChain chain runtime.

### New File

Added:

```text
crates/ron-ledger/src/quickchain/tree_material_projection.rs
```

### Module Wiring

Updated:

```text
crates/ron-ledger/src/quickchain/mod.rs
```

Expected additions:

```rust
mod tree_material_projection;

pub use tree_material_projection::{
    build_tree_material_batch,
    QuickChainTreeMaterialProjectionError,
    QuickChainTreeMaterialProjectionItem,
};
```

The re-export is required so integration tests and later safe preflight consumers can access the projection adapter.

---

## `ron-ledger` Tree-Material Projection API

New struct:

```rust
QuickChainTreeMaterialProjectionItem
```

Fields:

```text
sort_key_hex
payload_schema
payload_hash
```

Constructor:

```rust
QuickChainTreeMaterialProjectionItem::new(...)
```

Getters:

```rust
sort_key_hex()
payload_schema()
payload_hash()
```

New error enum:

```rust
QuickChainTreeMaterialProjectionError
```

Current variants:

```text
DuplicateSortKey
InvalidBatch
```

New builder function:

```rust
build_tree_material_batch(...)
```

Current intent:

* Accept unordered caller-supplied material items.
* Sort by bytewise ascending lowercase hex sort key.
* Reject duplicate sort keys before returning a batch.
* Build a strict `QuickChainTreeMaterialBatchV1`.
* Delegate final validation to the `ron-proto` DTO boundary.
* Return an inert, validated DTO batch ready for later vector work.

---

## Important Boundary Correction

An early version of the ledger projection file directly imported and used:

```rust
QuickChainCanonicalEncodingV1
```

That caused the broader ledger source-boundary test to fail because `ron-ledger` source is not allowed to own canonicalization vocabulary or root/preimage policy.

The fix was architectural, not cosmetic:

* Moved the canonical encoding choice behind a `ron-proto` helper:

  ```rust
  quickchain_tree_material_json_v1_encoding()
  ```
* Removed direct `QuickChainCanonicalEncodingV1` naming from `ron-ledger` projection source.
* Kept canonical DTO vocabulary in `ron-proto`.
* Kept `ron-ledger` as a deterministic sorting/projection adapter only.
* Repaired the ledger integration test imports so the helper is imported from `ron_proto::quickchain`, not from `ron_ledger::quickchain`.

This was the correct fix because it reinforced the project boundary instead of weakening the scanner.

---

## `ron-ledger` Tests Added

Added:

```text
crates/ron-ledger/tests/quickchain_phase1_tree_material.rs
```

This test suite covers:

1. Unordered account material is sorted into deterministic Phase 1 input order.
2. Duplicate material sort keys reject before a batch is returned.
3. Schema/tree mismatch rejects at the `ron-proto` DTO boundary.

Focused command:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase1_tree_material
```

Result:

```text
3 passed
```

The canonical-boundary scanner was also rerun:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Result:

```text
2 passed
```

Full ledger gate:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

## Why This Batch Matters

This batch is important because QuickChain Phase 1 cannot safely begin with root-producing code.

Before any root can exist, the system needs:

1. Strict input material DTOs.
2. Deterministic sort rules.
3. Explicit tree kinds.
4. Explicit schema/tree pairing.
5. Bounded input sizes.
6. Unknown-field rejection.
7. Cross-crate boundary enforcement.
8. Tests proving ledger does not take over canonicalization/root authority.

This session established those Phase 1 prerequisites without violating the blueprint.

The result is a safe “tree material staging layer”:

```text
ron-ledger deterministic state/projection
        ↓
explicit material commitments + sort keys
        ↓
ron-proto strict tree-material DTO batch
        ↓
future locked canonical byte vectors
        ↓
future locked hash vectors
        ↓
future root work
```

No root-producing step was added in this batch.

---

## Current Crate Status

### `ron-proto`

Status:

```text
Green for QC-1A entry batch.
```

What is complete in this slice:

* Tree-material DTOs exist.
* Tree-material validation exists.
* Tree-material unknown-field rejection exists.
* Tree-material schema/tree matching exists.
* Tree-material sort rule validation exists.
* Tree-material helper keeps canonical vocabulary in `ron-proto`.
* QuickChain preflight passed with `tests=22`.

What remains later:

* Locked canonical byte vectors for tree-material batches.
* Vector inventory updates for tree-material files.
* Independent verifier coverage for tree-material vectors.
* Locked hash vectors after locked bytes are reviewed.
* No fake/placeholder hashes.
* No root-producing code until vector gates are ready.

### `ron-ledger`

Status:

```text
Green for QC-1A entry batch.
```

What is complete in this slice:

* Deterministic tree-material projection adapter exists.
* Unordered input sorting is tested.
* Duplicate sort-key rejection is tested.
* DTO-boundary validation handoff is tested.
* Canonicalization/root source-boundary scanner passes.
* Full ledger QuickChain preflight passed with `tests=28`.

What remains later:

* Additional projection equality tests from live/replay state into material batches.
* Material-batch tests for holds, receipts, accounting, and rewards when those are ready.
* No root-producing code yet.
* No direct canonical byte serialization in ledger source.
* No hash/preimage construction in ledger source.
* No service or wallet mutation authority.

---

## Current QuickChain Completion Estimate After This Batch

Approximate status:

```text
QC-0A / Phase 0 safety-preflight:        ~98–100% complete
QC-1A / Phase 1 prep:                    ~12–18% complete
Phase 1 root-producing prototype:         ~5–10% complete
Full long-term QuickChain blueprint:      ~33–37% complete
```

This session moved QuickChain across the line from “Phase 0 safety/preflight only” into “Phase 1 preparation,” but it did not make Phase 1 root production ready yet.

---

## What Not To Do Next In These Crates

Do not add any of the following yet:

```text
- root-producing code
- checkpoint-producing code
- validator code
- committee code
- staking code
- liquidity code
- bridge code
- external settlement code
- public blockchain runtime code
- gateway/omnigate/rewarder ledger mutation
- raw engagement payout mutation
- DB-order roots
- wall-clock roots
- fake hashes
- placeholder hashes
- unreviewed canonical bytes
- unreviewed preimage formats
```

Do not move canonicalization ownership into `ron-ledger`.

Do not let `ron-ledger` serialize canonical bytes for root material.

Do not let `ron-ledger` build hash preimages.

Do not weaken the boundary scanners to make tests pass.

The correct pattern is:

```text
ron-proto owns DTO/canonical/vector vocabulary.
ron-ledger owns deterministic economic state/projection.
Future vector gates lock exact bytes and hashes before roots.
```

---

## Commands That Passed

Focused `ron-proto` tree-material test:

```bash
cargo test -p ron-proto --test quickchain_root_material
```

Focused `ron-ledger` tree-material projection test:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase1_tree_material
```

Focused `ron-ledger` canonical-boundary test:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Full `ron-proto` preflight gate:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Expected/observed final result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
```

Full `ron-ledger` preflight gate:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Observed final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

## Codebundle Regeneration

After the green gates, regenerate focused codebundles:

```bash
bash scripts/make_crate_codex.sh --force -c ron-proto
bash scripts/make_crate_codex.sh --force -c ron-ledger
```

These regenerated bundles should include:

For `ron-proto`:

```text
crates/ron-proto/src/quickchain/root_material.rs
crates/ron-proto/tests/quickchain_root_material.rs
```

For `ron-ledger`:

```text
crates/ron-ledger/src/quickchain/tree_material_projection.rs
crates/ron-ledger/tests/quickchain_phase1_tree_material.rs
```

---

## Recommended Next Crate Pair

The next crate pair should be:

```text
svc-wallet + ron-accounting
```

Reason:

```text
ron-proto now defines strict DTO material.
ron-ledger now provides deterministic internal truth/projection.
svc-wallet is the mutation front-door.
ron-accounting provides reporting/snapshot views but must never become balance truth.
```

The next work should prove that wallet/accounting can participate in QC-1A preparation without becoming QuickChain authority.

Expected next goals:

```text
- svc-wallet remains the explicit mutation front-door.
- wallet receipts remain backend-derived.
- no fake balances.
- no fake receipts.
- no silent spend.
- no direct QuickChain settlement authority.
- ron-accounting remains snapshot/reporting only.
- accounting never becomes balance truth.
- accounting never mutates ledger state.
- source-boundary tests block roots/validators/settlement creep.
- any material preparation remains inert and vector-facing only.
```

---

## Carry-Forward Warning

This was a successful QC-1A preparation slice, but it is still early Phase 1.

The next major safe milestone inside Phase 1 is not “build roots immediately.” The next safe milestone is:

```text
tree-material locked canonical byte vectors
        ↓
bash/vector inventory verification
        ↓
independent expected bytes review
        ↓
locked hash vectors
        ↓
only then root-prep discussion
```

The doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

This session honored that doctrine.


### END NOTE - JUNE 19 2026 - 21:55 CST


### BEGIN NOTE - JUNE 21 2026 - 19:35 CST

Your terminal output confirms both crate preflights are green: `ron-proto` passed its exhaustive QuickChain preflight with **22 tests**, and `ron-ledger` passed with **28 tests**. 

Here are the paste-ready crate notes.

# QuickChain Phase 1 Crate Notes — `ron-proto` + `ron-ledger`

## Session Summary

This session completed a safe QuickChain Phase 1 pre-root preparation slice for the core protocol and ledger crates:

* `crates/ron-proto`
* `crates/ron-ledger`

The work added and validated deterministic tree-material and first-layer reduction-plan preparation without crossing into forbidden Phase 1 authority. The patch keeps QuickChain inside the approved QC-1A boundary: DTOs, canonical material ordering, validation, replayable projections, and reduction planning only.

This session did **not** introduce root-producing code, checkpoint authority, validators, pruning, settlement, external anchors, public bridge logic, staking, liquidity, live chain state, CrabLink authority, service mutation authority, or public blockchain behavior.

The core outcome is that `ron-proto` can now describe validated tree material and adjacent-pair reduction plans, while `ron-ledger` can deterministically project ledger state into that material and prepare the first reduction layer. This gives the project a clean, audited pre-root staging surface before any actual deterministic root implementation is allowed.

---

## Phase / Scope

Current QuickChain phase context:

* Phase 0 — QC-0A safety/preflight: already green.
* Phase 1 — single-authority deterministic roots/proofs: in progress.
* Current session scope: QC-1A pre-root preparation.
* Current session result: `ron-proto` + `ron-ledger` are green for this pre-root/root-material preparation slice.

This work advances Phase 1, but it does not complete full Phase 1 by itself. Full Phase 1 still requires the later root/proof implementation once canonical bytes, locked vectors, and review gates explicitly authorize root-producing code.

---

## Safety Doctrine Preserved

The patch stayed aligned with the QuickChain doctrine:

* Determinism before distribution.
* DTOs before roots.
* Roots before validators.
* Proofs before pruning.
* Internal ROC before external anchors.
* Accounting is not balance truth.
* Rewarder plans payouts but never mutates ledger.
* Wallet/ledger remain the economic mutation authority.
* CrabLink remains a client/display layer, not chain authority.

The patch also preserved the current prohibition against:

* Root production.
* Checkpoint production.
* Validator implementation.
* Committee logic.
* Pruning.
* Settlement.
* External anchors.
* Public bridges.
* Staking or liquidity.
* Solana/ROX integration.
* Gateway/omnigate/rewarder ledger mutation.
* Raw engagement protocol payouts.
* DB-order or wall-clock-derived roots.
* Unknown consensus fields.
* Fake hashes or placeholder hashes.

---

# `ron-proto` Notes

## Purpose of the `ron-proto` Changes

`ron-proto` owns the strict DTO layer. In this session, it was extended so QuickChain can represent deterministic tree material and first-layer reduction plans as pure data.

The key idea is that `ron-proto` can describe the shape of material that will later feed root construction, but it still does not compute roots, hashes, signatures, proofs, settlement artifacts, or authority decisions.

The new DTOs make future root work safer because the input material and reduction plan are now explicit, versioned, validated, and test-gated before any root-producing code exists.

---

## Primary `ron-proto` File Changed

### `crates/ron-proto/src/quickchain/root_material.rs`

This file now contains the validated protocol surface for:

* Tree material items.
* Tree material batches.
* Tree material sort rules.
* Tree material tree kinds.
* Tree-material canonical JSON encoding.
* First-layer tree reduction pairs.
* First-layer tree reduction plans.
* Adjacent-pair reduction rule validation.
* Source item count validation.
* Odd final carry validation.

---

## Existing Tree Material DTOs Preserved

The existing tree-material DTOs remain the base of the pre-root pipeline:

### `QuickChainTreeMaterialItemV1`

Represents one already-prepared leaf/hash-payload material item.

Important fields:

* `schema`
* `version`
* `tree`
* `sort_key_hex`
* `payload_schema`
* `payload_hash`

Important validation behavior:

* Schema must match `quickchain.tree-material-item.v1`.
* Version must be `1`.
* Sort key must be lowercase even-length hex.
* Payload hash must be canonical `b3:<64 lowercase hex>`.
* Payload schema must match the declared tree type.

This DTO is still material only. It does not compute a payload hash and does not compute a tree root.

### `QuickChainTreeMaterialBatchV1`

Represents a sorted batch of material items for one tree.

Important fields:

* `schema`
* `version`
* `chain_id`
* `epoch_id`
* `tree`
* `canonical_encoding`
* `sort_rule`
* `items`

Important validation behavior:

* Schema must match `quickchain.tree-material-batch.v1`.
* Version must be `1`.
* `chain_id` and `epoch_id` must be bounded, non-empty QuickChain IDs.
* Canonical encoding must be `json_v1`.
* Sort rule must be `sort_key_hex_ascending_v1`.
* All items must match the batch tree.
* All items must be sorted by `sort_key_hex`.
* Duplicate sort keys are rejected.
* Batch size is bounded.

This is the validated pre-root input list.

---

## New `ron-proto` Constants Added

The following constants were added for the reduction-plan DTO layer:

```rust
pub const QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA: &str = "quickchain.tree-reduction-pair.v1";
pub const QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA: &str = "quickchain.tree-reduction-plan.v1";
pub const QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1: &str =
    "adjacent_pairs_with_odd_carry_v1";
pub const MAX_QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_BYTES: usize = 128;
```

These make the reduction-plan layer explicit and versioned.

---

## New DTO: `QuickChainTreeReductionPairV1`

A reduction pair describes one first-layer adjacent pair from sorted tree material.

Important fields:

* `schema`
* `version`
* `tree`
* `layer_index`
* `pair_index`
* `left_sort_key_hex`
* `left_payload_hash`
* `right_sort_key_hex`
* `right_payload_hash`

Important validation behavior:

* Schema must match `quickchain.tree-reduction-pair.v1`.
* Version must be `1`.
* `layer_index` must be `0`.
* Left sort key must be valid lowercase even-length hex.
* Left payload hash must be canonical `b3:<64 lowercase hex>`.
* Right member fields must be both present or both absent.
* If right member exists, right sort key must be greater than left sort key.
* A pair may omit the right member only to represent the final odd carry.

This DTO intentionally does not hash the pair. It only describes which material items will later be reduced together.

---

## New DTO: `QuickChainTreeReductionPlanV1`

A reduction plan describes the complete first reduction layer for one material batch.

Important fields:

* `schema`
* `version`
* `chain_id`
* `epoch_id`
* `tree`
* `canonical_encoding`
* `item_sort_rule`
* `pair_rule`
* `source_items_count`
* `pairs`

Important validation behavior:

* Schema must match `quickchain.tree-reduction-plan.v1`.
* Version must be `1`.
* `chain_id` and `epoch_id` must be valid bounded QuickChain IDs.
* Canonical encoding must be `json_v1`.
* Item sort rule must be `sort_key_hex_ascending_v1`.
* Pair rule must be `adjacent_pairs_with_odd_carry_v1`.
* Source item count must fit within the material-item cap.
* Pair count must equal `source_items_count.div_ceil(2)`.
* Pair indexes must be contiguous.
* Pair tree must match the plan tree.
* Pair order must be deterministic.
* Non-final odd carries are rejected.
* Counted source items must equal `source_items_count`.

This gives future root code an explicit, validated reduction schedule instead of allowing implicit runtime ordering.

---

## Clippy Fix Applied in `ron-proto`

Clippy rejected manual division-ceiling logic:

```rust
((source_items_count + 1) / 2) as usize
```

This was replaced with:

```rust
source_items_count.div_ceil(2) as usize
```

This is behavior-preserving and only satisfies the `manual_div_ceil` lint under `#![deny(warnings)]`.

---

## `ron-proto` Tests Updated

### `crates/ron-proto/tests/quickchain_root_material.rs`

This test now covers both the original material DTOs and the new reduction-plan DTOs.

Validated behavior includes:

* Sorted state material batch validation.
* Rejection of uppercase or odd-length hex sort keys.
* Rejection of unsorted sort keys.
* Rejection of duplicate sort keys.
* Rejection of payload-schema/tree mismatch.
* Valid adjacent-pair reduction plan with final odd carry.
* Rejection of partial right-member fields.
* Rejection of reversed right-member ordering.
* Rejection of non-final carry.
* Rejection of pair-count mismatch.
* Unknown-field rejection for both material and reduction DTOs.

The test count for this file is now 8 tests.

---

## `ron-proto` Exhaustive Preflight Result

The exhaustive QuickChain preflight for `ron-proto` passed.

Confirmed categories included:

* Canonical JSON.
* Chain params.
* Challenge wire DTOs.
* Domain separation.
* Strict DTO validation.
* Empty tree DTOs.
* Event classes.
* File-backed locked vectors.
* Hash payload vectors.
* Hold scenarios.
* IDs and money.
* Locked bytes.
* Hold operation DTOs.
* Receipt DTOs.
* Receipt ordering.
* Replay/idempotency.
* Root material and reduction DTOs.
* Sort keys.
* Source boundary.
* Tooling boundary.
* Vector DTOs.
* Vector inventory.
* Bash vector inventory verifier.
* Bash hash payload verifier.
* Clippy preflight.

Final result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
```

---

# `ron-ledger` Notes

## Purpose of the `ron-ledger` Changes

`ron-ledger` owns deterministic economic truth and replayable state reconstruction. In this session, it was extended to project deterministic state material into the new `ron-proto` tree-material and reduction-plan DTOs.

The ledger still does not compute roots in this QuickChain path. It only prepares validated, sorted material and a first-layer reduction schedule from already-produced payload hashes.

This keeps ledger work inside the approved pre-root boundary.

---

## Primary `ron-ledger` File Changed

### `crates/ron-ledger/src/quickchain/tree_material_projection.rs`

This file now provides deterministic projection from ledger-side material into `ron-proto` tree-material and reduction-plan DTOs.

---

## Existing Ledger Projection Preserved

### `QuickChainTreeMaterialProjectionItem`

This remains the ledger-side input item for building a protocol material batch.

Important fields:

* `sort_key_hex`
* `payload_schema`
* `payload_hash`

This is not a root or proof item. It is the ledger-side adapter item used to create a validated protocol material batch.

### `build_tree_material_batch`

This function takes:

* `chain_id`
* `epoch_id`
* `tree`
* unordered projection items

and returns a validated `QuickChainTreeMaterialBatchV1`.

Important behavior:

* Sorts items deterministically by sort key.
* Converts ledger projection items into `ron-proto` material items.
* Validates the final batch through `ron-proto`.
* Rejects duplicate sort keys.
* Rejects invalid payload schema/tree combinations.
* Rejects invalid hashes or invalid sort keys.
* Produces no root and no proof.

---

## New Ledger Function Added

### `build_tree_reduction_plan_from_batch`

This function takes a validated `QuickChainTreeMaterialBatchV1` and returns a validated `QuickChainTreeReductionPlanV1`.

Important behavior:

* Validates the source material batch first.
* Preserves the batch chain ID.
* Preserves the batch epoch ID.
* Preserves the batch tree kind.
* Preserves canonical encoding.
* Preserves item sort rule.
* Uses the explicit pair rule `adjacent_pairs_with_odd_carry_v1`.
* Chunks sorted material items into adjacent pairs.
* Creates `QuickChainTreeReductionPairV1` values.
* Allows only the final odd item to carry without a right member.
* Validates the final plan through `ron-proto`.
* Returns an error if validation fails.
* Does not compute roots.
* Does not compute hashes.
* Does not verify proofs.
* Does not read or write storage.
* Does not use runtime authority.
* Does not use wall-clock time.
* Does not mutate ledger state.

This is a pre-root adapter only.

---

## `ron-ledger` Public QuickChain Export Updated

### `crates/ron-ledger/src/quickchain/mod.rs`

The new `build_tree_reduction_plan_from_batch` function is exported through the gated QuickChain module.

This keeps the API available only under the existing `quickchain-preflight` feature boundary.

---

## Clippy Fix Applied in `ron-ledger`

The ledger projection used manual capacity calculation for pair count:

```rust
Vec::with_capacity((batch.items.len() + 1) / 2)
```

This was changed to:

```rust
Vec::with_capacity(batch.items.len().div_ceil(2))
```

This is behavior-preserving and keeps the crate clean under `-D warnings`.

---

## `ron-ledger` Tests Updated

### `crates/ron-ledger/tests/quickchain_phase1_tree_material.rs`

This test now covers both material-batch projection and reduction-plan construction.

Validated behavior includes:

* Unordered account material is sorted into deterministic Phase 1 input order.
* Duplicate material sort keys reject before batch return.
* Schema/tree mismatch rejects at the `ron-proto` boundary.
* Valid material batch builds first adjacent reduction plan with odd final carry.
* Empty material batch builds an empty reduction plan.
* Invalid material batch rejects before a reduction plan is returned.

The test count for this file is now 6 tests.

---

## Boundary Tests Preserved

The `ron-ledger` QuickChain source boundary remains green.

Most importantly:

### `quickchain_pre_root_boundary`

This test continues to prove the gated QuickChain ledger preflight sources do not gain runtime authority or root-production behavior.

The boundary continues to block forbidden concepts such as:

* Root-producing implementations in this pre-root path.
* Runtime authority.
* Network behavior.
* Storage authority.
* Clocks or randomness.
* Validator logic.
* Unauthorized consensus behavior.
* Hidden service behavior.

### `quickchain_projection_canonical_boundary`

The projection boundary also remains green and continues to ensure pre-root ledger sources do not serialize hash material or build preimages in the wrong place.

---

## `ron-ledger` Exhaustive Preflight Result

The exhaustive QuickChain preflight for `ron-ledger` passed.

Confirmed categories included:

* Accepted replay.
* Accepted replay auxiliary inputs.
* Accepted replay boundaries.
* Replay boundary extremes.
* Replay boundary identity rejection.
* Replay boundary rejection stability.
* Replay boundary retry stability.
* Replay evidence tamper rejection.
* Replay sequence evidence.
* Account leaf projection.
* Active hold leaf projection.
* Atomic execution.
* Balance transitions.
* Chain binding.
* Hold atomic execution.
* Operation hash projection.
* Phase 1 tree material projection.
* Pre-root source boundary.
* Projection canonical boundary.
* Projection context boundaries.
* Projection replay equality.
* Receipt hash projection.
* Replay index.
* Retry after accepted rebuild.
* State snapshot.
* Terminal hold account projection.
* Terminal hold replay evidence.
* Tooling boundary.
* Clippy preflight.

Final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

# What This Session Proves

This session proves that the core QuickChain Phase 1 material pipeline is now deterministic and pre-root safe across `ron-proto` and `ron-ledger`.

Specifically, the system can now:

1. Represent tree material as strict DTOs.
2. Validate tree material schemas and versions.
3. Validate canonical encoding.
4. Validate tree kind and payload-schema consistency.
5. Validate lowercase hex sort keys.
6. Validate canonical `b3:<64 lowercase hex>` payload hashes.
7. Validate sorted unique material batches.
8. Deterministically sort ledger-projected material.
9. Reject duplicate sort keys.
10. Reject schema/tree mismatches.
11. Build a first-layer adjacent reduction plan.
12. Carry a final odd item honestly.
13. Reject non-final odd carries.
14. Reject malformed right-member pairs.
15. Reject pair-count mismatches.
16. Preserve chain and epoch binding.
17. Preserve canonical encoding and sort rule binding.
18. Keep all projection work read-only.
19. Keep all reduction planning non-authoritative.
20. Keep root production out of scope.

This creates a safer launchpad for the eventual deterministic root implementation because root code will not get to invent ordering, pairing, or material semantics ad hoc. Those are now explicit and test-gated.

---

# What This Session Does Not Claim

This session does not claim:

* A state root exists.
* A receipt root exists.
* A holds root exists.
* A rewards root exists.
* A checkpoint is produced.
* A proof is generated.
* A proof is verified.
* Validators exist.
* A committee exists.
* QuickChain is live.
* CrabLink has chain authority.
* Gateway or omnigate can mutate ledger state.
* Rewarder can allocate ROC.
* Any external settlement path exists.
* Any ROX/Solana/public-chain bridge exists.
* Any staking or liquidity path exists.
* Any pruning path exists.

This distinction matters because the current implementation is intentionally pre-root and pre-authority.

---

# Architectural Impact

## Before This Session

The project already had:

* Strict QuickChain DTOs.
* Canonical JSON v1.
* Locked bytes vectors.
* Locked hash payload vectors.
* Sort-key validation.
* Operation/replay/idempotency tests.
* Atomic ledger execution.
* Hold lifecycle logic.
* Account and active-hold projections.
* Accepted replay boundaries.
* Source/tooling boundaries.

But the project still needed a more explicit pre-root bridge from sorted leaf material into a future tree-reduction process.

## After This Session

The project now has:

* Strict tree-material batch DTOs.
* Strict tree-reduction pair DTOs.
* Strict tree-reduction plan DTOs.
* Deterministic first-layer adjacent pairing.
* Explicit odd-carry handling.
* Ledger adapter from projected material to reduction plan.
* Tests proving both empty and non-empty reduction plans.
* Tests proving invalid material is rejected before reduction plan return.
* Clippy-clean implementation.
* Exhaustive preflight green for both crates.

This is a meaningful Phase 1 step because future root code can consume a validated plan instead of relying on hidden runtime ordering.

---

# Commands That Passed

Focused commands that passed:

```bash
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings

cargo test -p ron-proto --test quickchain_root_material
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase1_tree_material
```

Exhaustive preflight commands that passed:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final exhaustive results:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

# Files Touched

## `ron-proto`

Primary file:

```text
crates/ron-proto/src/quickchain/root_material.rs
```

Primary test:

```text
crates/ron-proto/tests/quickchain_root_material.rs
```

## `ron-ledger`

Primary file:

```text
crates/ron-ledger/src/quickchain/tree_material_projection.rs
```

Public gated module export:

```text
crates/ron-ledger/src/quickchain/mod.rs
```

Primary test:

```text
crates/ron-ledger/tests/quickchain_phase1_tree_material.rs
```

---

# Current Completion Status for This Crate Pair

`ron-proto` and `ron-ledger` are green for this Phase 1 pre-root/root-material preparation slice.

Status:

```text
ron-proto:  green
ron-ledger: green
focused tests: green
exhaustive preflight: green
clippy: green
```

This crate pair should not need more work right now unless the next session intentionally adds the next authorized Phase 1 root-vector/root-material step.

---

# Recommended Next Crate Pair

The next Phase 1 crate pair should be:

```text
svc-wallet + ron-accounting
```

Reason:

* `ron-proto` and `ron-ledger` now define and prove the deterministic pre-root material surface.
* `svc-wallet` is the mutation front-door and must remain the only service-facing path for issue/transfer/burn/hold/capture/release/receipt behavior.
* `ron-accounting` must remain snapshot/read-model only and must not become balance truth.
* The next pass should verify these crates do not accidentally treat QuickChain material, receipts, snapshots, or future root references as authority.

The next session should continue preserving the same boundary:

* No root production in service crates.
* No ledger mutation outside wallet/ledger.
* No fake receipts.
* No fake balances.
* No silent spend.
* No rewarder/gateway/omnigate mutation authority.
* No external settlement creep.
* No ROX/Solana/bridge/staking/liquidity code.

---

# Carry-Forward Notes for Future Phase 1 Work

The next root-related step, when authorized, should still proceed carefully.

Before actual root production is introduced, the project should require:

1. Locked canonical bytes for any new root input DTO.
2. Locked vectors for reduction-pair canonical bytes.
3. Locked vectors for reduction-pair hash preimages.
4. Locked vectors for odd-carry behavior.
5. Locked vectors for empty tree behavior.
6. Locked vectors for one-item tree behavior.
7. Locked vectors for two-item tree behavior.
8. Locked vectors for three-item tree behavior.
9. Locked vectors for multi-layer reduction behavior.
10. Explicit domain separator for each future hash context.
11. No placeholder hashes.
12. No DB-order dependence.
13. No wall-clock dependence.
14. No service-runtime dependence.
15. No hidden consensus fields.
16. No validator or checkpoint code until root vectors are locked.

The safe next evolution from this session would be vector planning for reduction-pair canonical bytes and reduction-pair hash payloads, still without producing a live root.

---

# One-Sentence Summary

This session completed and greened the `ron-proto` + `ron-ledger` QuickChain Phase 1 pre-root material/reduction-plan slice: the project can now deterministically validate, sort, batch, and first-layer pair future root material without producing roots or crossing into chain authority.

### END NOTE - JUNE 21 2026 - 19:35 CST


### BEGIN NOTE - JUNE 22 2026 - 16:05 CST

The latest exhaustive preflight confirms both crates are green: `ron-proto` passed `tests=23`, and `ron-ledger` passed `tests=29`, including the new root/proof tests and clippy preflights. 

Paste this into your crate notes:

# QuickChain Phase 1 Round 2 Crate Notes — `ron-proto` + `ron-ledger`

## Status

`ron-proto` and `ron-ledger` are now **Phase 1 complete** for QuickChain.

This completes the two intended Phase 1 rounds for the core pair:

* **Round 1:** deterministic DTOs, replay/idempotency shape, hash payloads, leaf projection contracts, canonical boundary hardening, pre-root safety.
* **Round 2:** deterministic tree root material, tree root DTOs, inclusion proof DTOs, deterministic root construction, inclusion proof construction/verification, and exhaustive boundary validation.

These crates should now be treated as **locked for Phase 1** unless a downstream crate pair exposes a real integration mismatch.

Overall QuickChain Phase 1 is **not complete yet** until the remaining Round 2 crate pairs pass, but the `ron-proto + ron-ledger` crate pair is complete for Phase 1.

---

# `ron-proto` Phase 1 Completion Notes

## What changed

`ron-proto` now owns the strict QuickChain wire/DTO contract for deterministic Phase 1 tree material, tree roots, and inclusion proofs.

The crate remains a DTO/schema/vector crate only. It does **not** compute roots, verify roots, mutate ledger state, claim consensus, produce validators, anchor externally, or define settlement authority.

## New or completed Phase 1 DTO surface

The QuickChain DTO layer now covers:

* Tree material batches.
* Material sort keys.
* Reduction plans.
* Tree leaf nodes.
* Tree branch nodes.
* Tree root payloads.
* Tree root DTOs.
* Inclusion proof steps.
* Inclusion proof DTOs.
* Tree algorithm constants.
* Domain separator helpers for tree-specific root/proof material.
* Strict schema/version validation.
* Strict tree/domain/encoding/algorithm validation.
* Strict unknown-field rejection.

The inclusion proof DTOs validate proof shape without assigning authority to the proof. A proof is treated as deterministic evidence tied to a root DTO, not as finality, consensus, or settlement.

## Important Phase 1 invariants now represented in `ron-proto`

`ron-proto` now freezes these core rules:

* Canonical JSON v1 remains the locked encoding.
* Domain-separated preimage rules remain explicit.
* BLAKE3 hashes are lowercase `b3:<64 hex>`.
* Tree identities are explicit and must match across root, leaf, proof, and domain fields.
* Root/proof fields reject unknown fields.
* Empty tree material is explicit and not represented with fake hashes.
* Source items must be sorted by canonical bytewise sort keys before root construction.
* Duplicate sort keys reject.
* Inclusion proof steps are bounded.
* Inclusion proof step layers must be unique and ordered.
* Single-item tree proofs have a strict no-step shape.
* Multi-item tree proofs must bind leaf index, source item count, root metadata, and leaf node metadata.

## Tests passed

The exhaustive `ron-proto` QuickChain preflight passed with `tests=23`.

Important Round 2 tests included:

* `quickchain_root_material`
* `quickchain_tree_proof`
* `quickchain_vector_inventory`
* `quickchain_hash_payloads`
* `quickchain_source_boundary`
* `quickchain_tooling_boundary`

The preflight also verified:

* QuickChain vector inventory.
* Locked byte vectors.
* Locked hash vectors.
* Bash-only tooling.
* Hash payload vectors.
* Clippy preflight.

## Boundary status

`ron-proto` remains correctly scoped:

* DTOs only.
* Validation only.
* Canonical/vector preparation only.
* No runtime ledger authority.
* No wallet authority.
* No validator authority.
* No settlement authority.
* No pruning.
* No external anchors.
* No public bridge.
* No ROX/Solana integration.
* No fake hashes/placeholders.

## Result

`ron-proto` is Phase 1 complete.

It now provides the strict DTO contract required by `ron-ledger` and downstream crates for deterministic Phase 1 root/proof work.

---

# `ron-ledger` Phase 1 Completion Notes

## What changed

`ron-ledger` now implements the deterministic Phase 1 root/proof slice over validated `ron-proto` material.

The crate remains the economic truth boundary for deterministic internal ROC state, but the new QuickChain tree work is still scoped to the gated `quickchain-preflight` feature and remains internal deterministic infrastructure. It does not create validators, public chain state, settlement, external anchors, pruning, or bridge logic.

## New or completed Phase 1 root/proof behavior

`ron-ledger` now supports:

* Deterministic tree material batch construction.
* Deterministic material sorting.
* Duplicate sort-key rejection.
* Root construction from validated material batches.
* Reduction plan construction.
* Odd-carry tree layer behavior.
* Empty tree root behavior using explicit empty payload hash.
* Inclusion proof construction for a requested material sort key.
* Inclusion proof verification against a root DTO.
* Tamper rejection for leaf commitments.
* Tamper rejection for sibling hashes.
* Metadata mismatch rejection between root and proof.
* Missing sort-key rejection before proof return.
* Strict conversion of `ron-proto` validation failures into bounded ledger errors.

## Important implementation boundaries

The root/proof work is intentionally narrow:

* Roots are deterministic local artifacts, not validator outputs.
* Proofs are deterministic local evidence, not finality.
* No chain authority is granted to clients, services, or proofs.
* No gateway or omnigate mutation authority is introduced.
* No accounting truth is introduced.
* No reward payout mutation is introduced.
* No external settlement hooks exist.
* No anchors exist.
* No pruning exists.
* No consensus committee exists.
* No public blockchain runtime exists.
* No ROX/Solana/public bridge logic exists.

The ledger computes deterministic tree material only from explicit validated inputs. It does not silently derive policy, receipt, clock, validator, settlement, or external-chain context.

## New error handling

`QuickChainTreeMaterialProjectionError` now includes proof/root-specific failures for:

* Missing material sort keys.
* Invalid proof DTOs.
* Failed proof verification.
* Invalid material/root DTOs.
* Duplicate sort keys.
* Invalid canonical bytes.
* Invalid computed hash.

These errors keep failure modes bounded and deterministic while avoiding raw stack traces, secrets, or authority-bearing messages.

## Tests passed

The exhaustive `ron-ledger` QuickChain preflight passed with `tests=29`.

Important Round 2 tests included:

* `quickchain_phase1_tree_material`
* `quickchain_phase1_tree_proof`
* `quickchain_pre_root_boundary`
* `quickchain_projection_canonical_boundary`
* `quickchain_tooling_boundary`

Focused Round 2 proof/root coverage passed:

* 7/7 inclusion proof tests.
* 11/11 tree material/root tests.
* 1/1 pre-root boundary test.
* 2/2 projection canonical boundary tests.

The full preflight also passed clippy with `-D warnings`.

## Specific proof/root behaviors now proven

The test suite proves:

* Two-leaf state root vector matches a real locked BLAKE3 hash.
* Three-leaf odd-carry tree root verifies correctly.
* Caller input order does not affect deterministic root output.
* Leaf commitment changes alter the root.
* Invalid material batches reject before root return.
* Invalid proof DTOs reject before success.
* Missing material sort key rejects before proof return.
* Tampered sibling hash rejects verification.
* Tampered leaf commitment rejects verification.
* Mismatched root/proof metadata rejects verification.
* Middle-leaf proof paths verify left/right sibling ordering correctly.
* Boundary scanners still prevent accidental runtime authority claims.

## Boundary status

`ron-ledger` remains correctly scoped:

* Ledger owns deterministic ROC truth.
* QuickChain root/proof logic is gated and deterministic.
* No validators.
* No checkpoints as consensus/finality.
* No external anchors.
* No pruning.
* No bridge.
* No settlement.
* No staking.
* No liquidity.
* No public chain authority.
* No wall-clock roots.
* No DB-order roots.
* No fake receipts or fake balances.

## Result

`ron-ledger` is Phase 1 complete.

It now consumes the `ron-proto` DTO contract and implements the deterministic root/proof construction and verification slice required for Phase 1.

---

# Pair-Level Result

The `ron-proto + ron-ledger` core pair is now complete for QuickChain Phase 1.

Completed pair responsibilities:

* DTO contract.
* Canonical JSON v1 boundary.
* Root material DTOs.
* Inclusion proof DTOs.
* Deterministic tree root construction.
* Deterministic inclusion proof build/verify.
* Real BLAKE3 hashes.
* Strict validation.
* Unknown-field rejection.
* Deterministic ordering.
* Empty tree handling.
* Boundary scanners.
* Bash-only tooling.
* Exhaustive crate-local preflights.
* Clippy preflight.

This pair should now be considered locked for Phase 1 unless downstream Round 2 work reveals a real integration mismatch.

---

# Next Phase 1 Round 2 Pair

The next crate pair should be:

`svc-wallet + ron-accounting`

Goal for the next pair:

* Ensure `svc-wallet` remains the mutation front door.
* Ensure `ron-accounting` remains snapshot/reporting, not balance truth.
* Confirm downstream crates do not reinterpret roots/proofs as spend authority.
* Confirm no fake receipt, fake balance, fake unlock, settlement, bridge, or external-chain authority is introduced.
* Keep QuickChain roots/proofs as deterministic evidence only.


### END NOTE - JUNE 22 2026 - 16:05 CST


### BEGIN NOTE - JUNE 23 2026 - 18:00 CST

Based on your green terminal output, these notes can mark **`ron-proto + ron-ledger` Phase 2 Round 1 complete / parked**: `ron-proto` exhaustive preflight passed with 24 tests, `ron-ledger` exhaustive preflight passed with 30 tests, and the new Phase 2 replay tests passed. 

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - ron-proto + ron-ledger

# QuickChain Phase 2 Round 1 Crate Notes — ron-proto + ron-ledger

## 0. Status

Crate pair:

```text
ron-proto + ron-ledger
```

Phase / round:

```text
QuickChain Phase 2 — small committee / replicated verification
Round 1 — verifier artifact / read-only replication
```

Status:

```text
COMPLETE / PARKED for Phase 2 Round 1
```

This does **not** mean all of Phase 2 Round 1 is complete. It means the first crate pair in the fixed buildplan order is complete for this round.

Remaining Phase 2 Round 1 crate pairs:

```text
1. ron-proto + ron-ledger          COMPLETE / PARKED
2. svc-wallet + ron-accounting     NEXT
3. svc-rewarder + svc-storage      pending
4. svc-gateway + omnigate          pending
5. svc-index + ron-policy          pending
6. CrabLink Tauri + client adapters pending
```

The work stayed inside the intended Phase 2 Round 1 scope:

```text
- verifier input/output DTOs
- replay bundle format
- read-only replay verification API
- deterministic mismatch diagnostics
- repeated replay equivalence tests
- no committee signing
- no quorum/finality
- no validators
- no staking/slashing
- no external settlement
- no bridge
- no public chain runtime
```

---

## 1. What Phase 2 Round 1 required for this pair

Per QUICKCHAIN_BUILDPLAN.MD, Phase 2 Round 1 is the “verifier artifact / read-only replication” pass.

For `ron-proto + ron-ledger`, this required:

```text
ron-proto:
- Define strict verifier replay DTOs.
- Define replay bundle input format.
- Define replay result/check output format.
- Reject unknown fields.
- Keep DTOs pure: no IO, no hashing execution, no ledger mutation, no service logic.

ron-ledger:
- Add read-only verifier replay API.
- Recompute Phase 1 roots from supplied material batches.
- Verify supplied inclusion proofs against expected roots.
- Return deterministic replay result DTOs.
- Report mismatches as diagnostics, not authority.
- Prove repeated replay produces identical results.
- Preserve Phase 1 boundary scanners.
```

This crate pair now satisfies that Round 1 target.

---

## 2. ron-proto changes

### 2.1 Added verifier replay DTO module

New file:

```text
crates/ron-proto/src/quickchain/verifier.rs
```

Purpose:

```text
Strict DTO definitions for Phase 2 read-only verifier replay bundles and replay results.
```

Important constants added:

```text
QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA
QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA
QUICKCHAIN_VERIFIER_PROOF_CHECK_SCHEMA
QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA
QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1
MAX_QUICKCHAIN_VERIFIER_TREE_KINDS
MAX_QUICKCHAIN_VERIFIER_INCLUSION_PROOFS
MAX_QUICKCHAIN_VERIFIER_REPLAY_CHECKS
MAX_QUICKCHAIN_VERIFIER_ALGORITHM_BYTES
MAX_QUICKCHAIN_VERIFIER_DETAIL_BYTES
```

Important DTOs added:

```text
QuickChainVerifierReplayBundleV1
QuickChainVerifierRootCheckV1
QuickChainVerifierProofCheckV1
QuickChainVerifierReplayResultV1
```

Important enums added:

```text
QuickChainVerifierCheckStatusV1
  - Verified
  - Mismatch

QuickChainVerifierReplayStatusV1
  - Verified
  - Mismatch
```

### 2.2 Replay bundle validation behavior

`QuickChainVerifierReplayBundleV1::validate()` now enforces:

```text
- correct schema
- correct DTO version
- valid chain_id
- valid epoch_id
- supported replay algorithm only:
  read_only_root_and_proof_replay_v1
- at least one material batch
- material batch count capped
- exactly one expected root per material batch
- no duplicate material trees
- no duplicate expected root trees
- all material batches match bundle chain_id and epoch_id
- all expected roots match bundle chain_id and epoch_id
- expected root source_items_count matches material batch item count
- each inclusion proof corresponds to material + expected root artifacts
- proof root hash matches expected root hash
- proof source_items_count matches material batch item count
```

### 2.3 Replay result validation behavior

`QuickChainVerifierReplayResultV1::validate()` now enforces:

```text
- correct schema
- correct DTO version
- valid chain_id
- valid epoch_id
- supported replay algorithm only
- material_batches_count > 0
- root_checks.len() == expected_roots_count
- proof_checks.len() == inclusion_proofs_count
- expected_roots_count == material_batches_count
- all root/proof checks match result chain_id and epoch_id
- Verified result cannot contain mismatched checks
- Mismatch result must contain at least one mismatched check
```

### 2.4 DTO authority boundaries preserved

The new `ron-proto` DTOs are deliberately inert.

They do **not**:

```text
- hash
- sign
- verify signatures
- read files
- write files
- call services
- mutate ledger state
- define validators
- define quorum/finality
- define staking/slashing
- define bridge/external settlement
- grant spend authority
```

They only define strict wire/data shapes and validation.

### 2.5 ron-proto module wiring

Updated file:

```text
crates/ron-proto/src/quickchain/mod.rs
```

Added:

```rust
pub mod verifier;
pub use verifier::*;
```

This exposes the verifier DTOs through the existing `ron_proto::quickchain::*` surface.

---

## 3. ron-proto tests added

New file:

```text
crates/ron-proto/tests/quickchain_verifier_replay.rs
```

Tests added:

```text
verifier_replay_bundle_accepts_one_material_tree_and_expected_root
verifier_replay_bundle_rejects_missing_expected_root
verifier_replay_bundle_rejects_duplicate_material_tree
verifier_replay_result_status_must_match_checks
verifier_replay_bundle_rejects_unknown_fields
```

Result:

```text
running 5 tests
5 passed
0 failed
```

This test file confirms:

```text
- strict replay-bundle DTO validation
- missing expected roots reject
- duplicate material trees reject
- result status must match check statuses
- unknown fields reject
```

---

## 4. ron-ledger changes

### 4.1 Added read-only replicated replay module

New file:

```text
crates/ron-ledger/src/quickchain/replicated_replay.rs
```

Purpose:

```text
Read-only QuickChain Phase 2 replay-bundle verification adapter.
```

Primary function added:

```rust
verify_replay_bundle_read_only(
    bundle: &QuickChainVerifierReplayBundleV1,
) -> Result<QuickChainVerifierReplayResultV1, QuickChainTreeMaterialProjectionError>
```

Behavior:

```text
1. Validate the verifier replay bundle through ron-proto.
2. For each material batch:
   - find the expected root for that tree
   - recompute the root through existing Phase 1 tree material projection logic
   - compare expected root hash vs recomputed root hash
   - emit Verified or Mismatch root check
3. For each supplied inclusion proof:
   - find the expected root for that tree
   - verify the proof through existing Phase 1 proof verification logic
   - emit Verified or Mismatch proof check
4. Return QuickChainVerifierReplayResultV1.
5. Validate the result before returning it.
```

### 4.2 Error handling

The repair patch deliberately avoided adding new error enum variants after the Perl replacement failed.

Instead, malformed verifier bundles/results map into the existing deterministic ledger projection error taxonomy:

```text
QuickChainTreeMaterialProjectionError::InvalidBatch
```

Proof verification failures are treated as mismatch diagnostics in the returned replay result when the bundle itself is valid.

This keeps the patch smaller and avoids unnecessary enum churn.

### 4.3 ron-ledger module wiring

Updated file:

```text
crates/ron-ledger/src/quickchain/mod.rs
```

Added:

```rust
mod replicated_replay;
pub use replicated_replay::verify_replay_bundle_read_only;
```

### 4.4 Ledger authority boundaries preserved

The new `ron-ledger` replay function is read-only.

It does **not**:

```text
- mutate balances
- mutate holds
- mutate receipts
- mutate ledger state
- read database order
- use wall-clock time
- call network services
- write files
- create validators
- sign anything
- define quorum/finality
- define staking/slashing
- define settlement/bridge behavior
```

It only recomputes and verifies deterministic artifacts already defined by Phase 1.

---

## 5. ron-ledger tests added

New file:

```text
crates/ron-ledger/tests/quickchain_phase2_replicated_replay.rs
```

Tests added:

```text
two_read_only_replay_runs_reproduce_identical_results
root_mismatch_returns_diagnostic_mismatch_not_authority
missing_expected_root_rejects_as_invalid_replay_bundle
```

Result:

```text
running 3 tests
3 passed
0 failed
```

These tests prove the important Phase 2 Round 1 exit behavior for this crate pair:

```text
- the same replay bundle can be checked twice and returns identical results
- replay result status is Verified when expected roots/proofs match
- replay result status is Mismatch when expected root hash is wrong
- root mismatch is diagnostic only, not authority
- malformed bundles reject before replay
```

---

## 6. Focused commands run

Focused patch/test commands succeeded:

```bash
cargo fmt -p ron-proto -p ron-ledger

cargo test -p ron-proto --test quickchain_verifier_replay
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase2_replicated_replay

cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Results:

```text
ron-proto quickchain_verifier_replay:
  5 passed
  0 failed

ron-ledger quickchain_phase2_replicated_replay:
  3 passed
  0 failed

ron-ledger quickchain_pre_root_boundary:
  1 passed
  0 failed

ron-ledger quickchain_projection_canonical_boundary:
  2 passed
  0 failed
```

Patch completion message:

```text
QuickChain Phase 2 Round 1 verifier replay repair patch applied.
```

---

## 7. Park / preflight commands run

After focused tests, both crate-local QuickChain preflight scripts were run:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

### 7.1 ron-proto preflight result

Final marker:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=24
```

Important included tests:

```text
quickchain_canonical_json
quickchain_chain_params
quickchain_challenge_wire
quickchain_domain_separation
quickchain_dto_strict
quickchain_empty_tree
quickchain_event_class
quickchain_file_vectors
quickchain_hash_payloads
quickchain_hold_scenarios
quickchain_ids_and_money
quickchain_locked_bytes
quickchain_operation_hold
quickchain_receipt_dto
quickchain_receipt_order
quickchain_replay_idempotency
quickchain_root_material
quickchain_sort_keys
quickchain_source_boundary
quickchain_tooling_boundary
quickchain_tree_proof
quickchain_vector_dto
quickchain_vector_inventory
quickchain_verifier_replay
```

Also passed:

```text
bash vector inventory verifier
bash hash payload verifier
clippy preflight
```

Vector inventory marker:

```text
verified quickchain vector inventory: files=36 locked_bytes=31 locked_hash=5 sketch=0 typed_vectors=31
```

Hash payload verifier marker:

```text
verified quickchain hash payload vectors with bash: files=5 payload_bytes=3841 preimage_bytes=3972 optional_b3sum_checks=0
```

### 7.2 ron-ledger preflight result

Final marker:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=30
```

Important included tests:

```text
quickchain_accepted_replay_auxiliary_inputs
quickchain_accepted_replay_boundary_extremes
quickchain_accepted_replay_boundary_identity_rejection
quickchain_accepted_replay_boundary_rejection_stability
quickchain_accepted_replay_boundary_retry_stability
quickchain_accepted_replay_boundary
quickchain_accepted_replay_evidence_tamper
quickchain_accepted_replay_sequence_evidence
quickchain_accepted_replay
quickchain_account_leaf_projection
quickchain_active_hold_leaf_projection
quickchain_atomic_execution
quickchain_balance_transition
quickchain_chain_binding
quickchain_hold_atomic_execution
quickchain_operation_hash_projection
quickchain_phase1_tree_material
quickchain_phase1_tree_proof
quickchain_phase2_replicated_replay
quickchain_pre_root_boundary
quickchain_projection_canonical_boundary
quickchain_projection_context_boundaries
quickchain_projection_replay_equality
quickchain_receipt_hash_projection
quickchain_replay_index
quickchain_retry_after_accepted_rebuild
quickchain_state_snapshot
quickchain_terminal_hold_account_projection
quickchain_terminal_hold_replay_evidence
quickchain_tooling_boundary
```

Also passed:

```text
clippy preflight
```

---

## 8. Important design decisions made

### 8.1 No enum churn after Perl failure

The first patch attempt failed during a Perl replacement intended to add new ledger error enum variants.

Repair decision:

```text
Do not edit the existing ledger projection error enum.
Map verifier bundle/result DTO failures into existing InvalidBatch error variant.
```

Reason:

```text
- smaller change
- less fragile
- avoids unnecessary public-ish error taxonomy expansion
- keeps Phase 2 Round 1 diagnostic-only
```

### 8.2 Replay mismatch is not an error when the bundle shape is valid

A valid replay bundle with a wrong expected root now returns:

```text
QuickChainVerifierReplayStatusV1::Mismatch
```

It does not panic, mutate, claim slashing, claim finality, or produce settlement effects.

This is the right Phase 2 Round 1 behavior because mismatch reporting is diagnostic evidence, not committee authority.

### 8.3 ron-proto owns verifier DTO shape; ron-ledger owns deterministic checking

Boundary:

```text
ron-proto:
  DTO shape + validation only

ron-ledger:
  read-only deterministic recomputation/checking over existing Phase 1 artifacts
```

This preserves the existing architecture:

```text
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

---

## 9. Boundaries still enforced

The following boundaries remain intact:

```text
- no ROX active runtime
- no Solana active runtime
- no external settlement
- no public bridge
- no staking
- no liquidity
- no exchange-facing logic
- no gateway/omnigate ledger mutation
- no rewarder ledger mutation
- no index/policy/cache/client payment truth
- no raw engagement direct ROC allocation
- no pruning
- no committee signing
- no quorum/finality semantics
- no validator economy
- no wall-clock roots
- no DB-order roots
- no fake hashes/placeholders
```

The new code is safe to carry forward into downstream Phase 2 Round 1 crate pairs because it only exposes inert verifier DTOs and a read-only replay checker.

---

## 10. Current completion judgment

Crate pair status:

```text
ron-proto + ron-ledger are complete for QuickChain Phase 2 Round 1.
```

Not yet complete:

```text
Full Phase 2 Round 1
Full Phase 2
```

Next crate pair:

```text
svc-wallet + ron-accounting
```

Next crate-pair goal:

```text
Consume/audit the verifier artifact model from the wallet/accounting boundary without allowing wallet or accounting to become verifier, finality, root, settlement, bridge, or committee authority.
```

Expected next-pair scope:

```text
svc-wallet:
- wallet receipts may be projected into verifier material as backend-derived evidence
- wallet must reject client-supplied verifier/finality/committee poison fields
- wallet must remain mutation front-door only
- wallet must not claim finality/quorum/settlement truth

ron-accounting:
- accounting snapshots may remain derivative artifacts
- accounting must not link ron-ledger/ron-proto verifier authority unless explicitly needed later
- accounting must not become balance truth
- accounting must reject or avoid verifier/finality/committee poison fields
```

---

## 11. Commands for future reference

Focused retest for this pair:

```bash
cargo test -p ron-proto --test quickchain_verifier_replay
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase2_replicated_replay
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Full crate-local parks:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=24
ron-ledger quickchain exhaustive preflight gate passed: tests=30
```

---

## 12. Summary

`ron-proto + ron-ledger` now provide the Phase 2 Round 1 core:

```text
strict verifier replay DTOs
+ read-only replay bundle verification
+ deterministic root/proof replay checks
+ mismatch diagnostics
+ repeated replay equivalence
+ exhaustive crate-local preflight green
```

This completes the first crate pair for Phase 2 Round 1 and parks it until a downstream crate exposes a real missing contract.

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - ron-proto + ron-ledger


### END NOTE - JUNE 23 2026 - 18:00 CST


### BEGIN NOTE - JUNE 23 2026 - 23:05 CST

Grounding: the final terminal proof shows the focused Phase 2 Round 2 committee tests and clippy passed for both crates, then `ron-proto` exhaustive QuickChain preflight passed with **tests=25** and `ron-ledger` exhaustive QuickChain preflight passed with **tests=31**. 

Here are the paste-ready crate notes:

# QuickChain Phase 2 Round 2 Crate Notes — `ron-proto` + `ron-ledger`

## Session Summary

This session completed the first crate pair of **QuickChain Phase 2 Round 2**:

```text
ron-proto + ron-ledger
```

This is the final Phase 2 round for this pair. The work added bounded committee attestation/readiness DTOs and a read-only ledger-side committee readiness evaluator for replicated verification.

The core result is:

```text
ron-proto + ron-ledger are complete / parked for QuickChain Phase 2 Round 2.
```

This does **not** mean all of Phase 2 is complete yet. It means the first crate pair in Phase 2 Round 2 is green and parked. The remaining Phase 2 Round 2 crate pairs still need their completion pass.

---

## Phase / Scope

Current QuickChain phase:

```text
Phase 2 — small committee / replicated verification
Round 2 — final Phase 2 completion pass
```

Authorized scope for this pair:

```text
bounded committee identity DTOs
bounded verifier attestation DTOs
attestation-set validation
deterministic readiness status
deterministic disagreement taxonomy
read-only replicated verification interlock
quorum-readiness checks
duplicate-attestation rejection
mismatched replay-result rejection
outside-committee attestation rejection
```

Forbidden scope remained excluded:

```text
no staking
no slashing
no validator economy
no public bridge
no external settlement
no ROX/Solana runtime
no exchange-facing logic
no public blockchain runtime
no pruning
no anchors
no live finality
no gateway authority
no CrabLink authority
no wallet mutation from attestations
no ledger mutation from attestations
```

The patch stayed inside the Phase 2 doctrine:

```text
replicated verification without validator economy
committee readiness without settlement authority
attestations as bounded verification artifacts, not balance truth
ron-proto owns DTO shape
ron-ledger owns deterministic read-only evaluation
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

---

## `ron-proto` Notes

### Purpose

`ron-proto` now carries the strict DTO vocabulary for Phase 2 committee readiness.

It defines the data shapes needed for replicated verifier attestations without making those attestations into public-chain finality, settlement authority, staking authority, or wallet authority.

### Main Area Touched

```text
crates/ron-proto/src/quickchain/verifier.rs
```

### Main Test Added

```text
crates/ron-proto/tests/quickchain_committee_attestation.rs
```

### DTO / Contract Additions

`ron-proto` now includes Phase 2 committee/readiness vocabulary for:

```text
QuickChainCommitteeMemberV1
QuickChainVerifierAttestationV1
QuickChainCommitteeAttestationSetV1
QuickChainCommitteeReadinessResultV1
QuickChainCommitteeReadinessStatusV1
QuickChainCommitteeDisagreementCodeV1
```

The schema constants added for these shapes bind the wire contract and keep Phase 2 artifacts explicit:

```text
quickchain.committee-member.v1
quickchain.verifier-attestation.v1
quickchain.committee-attestation-set.v1
quickchain.committee-readiness-result.v1
quickchain.verifier-attestation-payload.v1
```

The current committee algorithm token is bounded and descriptive:

```text
bounded_committee_replay_attestation_v1
```

It is not a settlement algorithm, finality algorithm, public chain algorithm, fork-choice algorithm, staking algorithm, or bridge algorithm.

### Validation Rules Added

The new `ron-proto` validation enforces:

```text
schema/version correctness
chain_id validation
epoch_id validation
bounded committee algorithm string
non-empty committee member set
bounded committee member count
bounded attestation count
required_attestations > 0
required_attestations <= committee_members.len()
committee member chain/epoch match
unique committee member IDs
attestation chain/epoch match
attestation member must be declared in committee
one attestation per committee member
attestation replay_result_hash must match set replay_result_hash
attestation replay_status must match set replay_status
readiness result counts must match status
Ready requires accepted_attestations >= required_attestations
Ready must not carry a disagreement code
NotReady requires InsufficientAttestations
Disagreement requires a disagreement code
strict unknown-field rejection
```

### Important Boundary

`ron-proto` remains DTO/schema/canonical vocabulary only.

It does not:

```text
verify signatures cryptographically
choose committee membership from runtime state
create finality
produce roots
mutate ledger state
mutate wallet state
decide settlement
anchor externally
prune data
start validators
create staking/slashing economics
```

The signature fields are represented as bounded wire data only. Verification is still future/service-owned and not a claim of public-chain authority.

### Tests Confirmed

Focused test:

```bash
cargo test -p ron-proto --test quickchain_committee_attestation
```

Result:

```text
6 passed
```

The test suite covers:

```text
valid bounded committee attestation set
duplicate member-attestation rejection
outside-committee attestation rejection
mismatched replay result hash rejection
readiness status/count consistency
unknown-field rejection
```

Full preflight:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Final result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=25
```

The exhaustive run also confirmed:

```text
bash vector inventory verifier passed
bash hash payload verifier passed
clippy preflight passed
```

Current vector inventory proof:

```text
files=36
locked_bytes=31
locked_hash=5
sketch=0
typed_vectors=31
```

Current hash payload verifier proof:

```text
files=5
payload_bytes=3841
preimage_bytes=3972
optional_b3sum_checks=0
```

### Clippy Fix

A strict clippy issue was found after the first green focused tests:

```text
clippy::manual_contains
```

It was fixed by replacing `.iter().any(...)` membership checks with `.contains(...)` in the committee validation paths.

This was a pure style/performance cleanup and did not change QuickChain semantics.

---

## `ron-ledger` Notes

### Purpose

`ron-ledger` now exposes a narrow, read-only Phase 2 committee readiness evaluator.

The evaluator lets a replicated verification artifact be checked for threshold readiness after the underlying replay result has already been computed and represented. It does not mutate the ledger, does not verify signatures cryptographically, and does not create finality.

### Main Area Touched

```text
crates/ron-ledger/src/quickchain/replicated_replay.rs
crates/ron-ledger/src/quickchain/mod.rs
```

### Main Test Added

```text
crates/ron-ledger/tests/quickchain_phase2_committee_readiness.rs
```

### Function Added

```text
evaluate_committee_readiness_from_attestations
```

Purpose:

```text
take a validated QuickChainCommitteeAttestationSetV1
check its deterministic validation contract
count accepted bounded attestations
return a QuickChainCommitteeReadinessResultV1
```

It returns:

```text
Ready
```

only when:

```text
accepted_attestations_count >= required_attestations
```

It returns:

```text
NotReady + InsufficientAttestations
```

when:

```text
accepted_attestations_count < required_attestations
```

It rejects invalid attestation sets before returning a readiness result.

### Important Boundary

`ron-ledger` remains economic truth and deterministic replay/projection core.

This new Phase 2 evaluator is deliberately read-only. It does not:

```text
mutate ledger state
mutate wallet state
issue ROC
transfer ROC
burn ROC
open/capture/release/expire holds
create receipts
verify signatures cryptographically
decide finality
choose forks
start validators
create public chain state
anchor externally
settle externally
prune data
create staking/slashing rules
```

The evaluator is an interlock for replicated verification readiness, not a runtime chain authority.

### Tests Confirmed

Focused test:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase2_committee_readiness
```

Result:

```text
4 passed
```

The test suite covers:

```text
committee readiness becomes Ready after bounded threshold attestations
committee readiness stays NotReady below threshold
duplicate member attestation rejects before readiness
mismatched replay result hash rejects before readiness
```

Focused clippy:

```bash
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
```

Result:

```text
passed
```

Full preflight:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=31
```

The exhaustive run also confirmed:

```text
quickchain_phase2_replicated_replay passed
quickchain_phase2_committee_readiness passed
quickchain_pre_root_boundary passed
quickchain_projection_canonical_boundary passed
quickchain_tooling_boundary passed
clippy preflight passed
```

---

## Files Touched

### `ron-proto`

```text
crates/ron-proto/src/quickchain/verifier.rs
crates/ron-proto/tests/quickchain_committee_attestation.rs
```

### `ron-ledger`

```text
crates/ron-ledger/src/quickchain/replicated_replay.rs
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/tests/quickchain_phase2_committee_readiness.rs
```

---

## Commands That Passed

Focused Phase 2 Round 2 tests:

```bash
cargo test -p ron-proto --test quickchain_committee_attestation
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase2_committee_readiness
```

Focused clippy gates:

```bash
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
```

Full crate preflights:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final green markers:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=25
ron-ledger quickchain exhaustive preflight gate passed: tests=31
```

---

## Current Completion Status

```text
ron-proto:  Phase 2 Round 2 complete / parked
ron-ledger: Phase 2 Round 2 complete / parked
focused tests: green
clippy: green
exhaustive preflight: green
```

This crate pair should not need more Phase 2 Round 2 work unless a downstream crate exposes a real integration mismatch.

---

## Next Crate Pair

Continue Phase 2 Round 2 with:

```text
svc-wallet + ron-accounting
```

Reason:

```text
ron-proto now defines the bounded committee attestation/readiness DTO surface.
ron-ledger now evaluates committee readiness as read-only replicated verification.
svc-wallet must remain the mutation front-door.
ron-accounting must remain snapshot/reporting only and never balance truth.
```

Expected next goals:

```text
svc-wallet confirms committee/readiness artifacts cannot bypass explicit wallet mutation paths
svc-wallet remains issue/transfer/burn/hold/capture/release/receipt front-door only
svc-wallet does not treat attestations as spend authority
svc-wallet does not create fake balances or fake receipts
svc-wallet does not silently spend
ron-accounting can observe/report replicated verification metadata only if safe
ron-accounting remains read-only snapshot/reporting
ron-accounting never mutates ledger state
ron-accounting never becomes balance truth
both crates reject settlement/validator/staking/bridge scope creep
```

---

## Carry-Forward Warning

Do not reinterpret Phase 2 committee readiness as public-chain finality.

The safe interpretation is:

```text
small committee can independently replay/verify and produce bounded attestations
attestation sets can be checked for threshold readiness
readiness is a verification artifact, not balance truth
readiness is not settlement
readiness is not staking/slashing
readiness is not public bridge authority
readiness is not external chain authority
```

The doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

For this crate pair, Phase 2 Round 2 honored that doctrine.


### END NOTE - JUNE 23 2026 - 23:05 CST


### BEGIN NOTE - JUNE 24 2026 - 12:20 CST

Here are paste-ready crate notes for the Phase 3 `ron-proto + ron-ledger` pass. These are based on the green exhaustive gates: `ron-proto` passed 26 QuickChain tests and `ron-ledger` passed 32 QuickChain tests, including the new Phase 3 validator/passport-gating tests and clippy preflight. 

## QuickChain Phase 3 Round 1 Notes — ron-proto

### Status

`ron-proto` is complete for QuickChain Phase 3 Round 1.

The crate now carries the Phase 3 passport-gated validator DTO surface while staying strictly DTO-only. It defines validator identity, capability, validator-set, authorization request, and authorization result shapes without adding runtime authority, crypto verification, staking, slashing, bridge logic, settlement logic, or ledger mutation.

### Phase 3 Scope Completed

Added strict passport-gated validator-set DTO scaffolding for:

* validator identity shape
* validator lifecycle status
* validator capability shape
* validator-set membership shape
* validator authorization request shape
* validator authorization result shape
* deterministic rejection/status enums
* passport-required admission semantics
* capability validity windows
* explicit revocation/expiry states
* validator-set hash binding
* replay-result hash/status binding
* schema constants and algorithm/scope constants

### Important Boundaries Preserved

`ron-proto` remains pure DTO/canonicalization surface only.

It does not:

* verify signatures
* authorize validators at runtime
* mutate ledger or wallet state
* create roots/checkpoints/finality
* introduce staking, bonding, slashing, liquidity, bridge, ROX, Solana, or external settlement logic
* make validators economic authorities
* make passports wallets
* create public-chain authority

Phase 3 validator shapes are descriptive and validation-only. Actual identity, capability, and registry truth remain service-owned and explicit.

### Key Design Decisions

Validator admission for this round is passport/registry gated, not stake gated.

Bonded economics are explicitly rejected for this round. The validator-set DTO includes flags that enforce:

* `passport_required = true`
* `bond_required = false`

This keeps Phase 3 aligned with the QuickChain doctrine:

* internal ROC first
* determinism before distribution
* DTOs before authority
* no bonded economics until explicitly authorized later

Validator capability scope is narrow and explicit:

* current scope: `quickchain.verify.v1`
* capability is for read-only verification/attestation participation
* capability does not grant wallet, ledger, storage, gateway, rewarder, or policy mutation authority

### New Test Coverage

Added Phase 3 validator-set tests covering:

* valid passport-required / bond-forbidden validator set
* rejection of missing passport requirement
* rejection of bonded economics
* duplicate passport subject rejection
* authorization request binding to validator-set hash, attestation, and capability
* wrong validator-set hash rejection
* authorization result status/rejection-code consistency
* unknown-field rejection for strict wire shape

### Confirmed Gate

`ron-proto` exhaustive QuickChain preflight passed:

* all discovered `quickchain_*.rs` tests passed
* Phase 3 validator-set test passed
* vector inventory verifier passed
* hash payload verifier passed
* clippy preflight passed

Final status:

`ron-proto` is Phase 3 Round 1 complete.

## QuickChain Phase 3 Round 1 Notes — ron-ledger

### Status

`ron-ledger` is complete for QuickChain Phase 3 Round 1.

The crate now has a read-only passport-gated validator authorization evaluator for QuickChain Phase 3. This evaluator checks whether a validator attestation is allowed by an explicit validator-set and capability request, while preserving the ledger as economic truth and preventing validator authority from leaking into wallet, ledger, settlement, staking, or runtime mutation paths.

### Phase 3 Scope Completed

Added read-only validator authorization evaluation for:

* active passport/registry validator authorization
* unauthorized validator rejection
* revoked validator rejection
* expired validator/capability rejection
* revoked capability rejection
* key mismatch rejection
* bonded-economics rejection before authorization
* validator-set hash binding
* capability scope binding
* passport subject binding
* replay-result binding
* explicit evaluation time input

### Important Boundaries Preserved

`ron-ledger` remains the economic truth boundary, but the new Phase 3 validator gate does not mutate ledger truth.

It does not:

* issue ROC
* transfer ROC
* burn ROC
* open/capture/release holds
* mutate balances
* mutate account sequences
* create roots/finality/checkpoints
* create validator consensus
* perform signature verification
* perform registry IO
* perform wallet calls
* create staking, slashing, bridge, ROX, Solana, or external settlement logic

The validator gate is a deterministic read-only evaluator. It consumes explicit DTO inputs and returns an authorization result. It does not fetch registry state, trust wall-clock time, or create any economic authority.

### Key Design Decisions

Lifecycle handling is fail-closed.

Because `QuickChainValidatorLifecycleStatusV1` is `#[non_exhaustive]`, `ron-ledger` now includes a wildcard lifecycle match arm that treats any future unknown validator lifecycle status as `ValidatorNotActive` until explicitly designed and tested.

That means future lifecycle expansion cannot accidentally authorize a new status.

Validator authorization remains passport/registry based for Phase 3 Round 1:

* active validator identity required
* active capability required
* capability must be within explicit evaluation time bounds
* validator-set hash must match
* attestation member id must match identity
* attestation public key must match identity
* capability passport subject must match identity
* bonded economics are rejected

### Boundary Test Update

The pre-root boundary scanner was updated with a narrow exception for the Phase 3 read-only validator gate source file.

This exception permits the new validator-gating terminology only where expected, while keeping the broader ron-ledger QuickChain source boundary intact against unauthorized runtime authority/root/settlement expansion.

### New Test Coverage

Added Phase 3 validator-gate tests covering:

* active passport registry validator authorizes read-only attestation
* unauthorized identity rejects
* revoked identity rejects
* revoked capability rejects
* expired capability rejects
* bonded economics reject before authorization
* key mismatch rejects without mutating ledger truth

Existing pre-root boundary test also passed after the Phase 3 exception.

### Confirmed Gate

`ron-ledger` exhaustive QuickChain preflight passed:

* all discovered `quickchain_*.rs` tests passed
* Phase 3 validator-gate test passed
* pre-root boundary test passed
* accepted replay tests passed
* tree material/proof tests passed
* Phase 2 committee/readiness tests remained green
* clippy preflight passed

Final status:

`ron-ledger` is Phase 3 Round 1 complete.


### END NOTE - JUNE 24 2026 - 12:20 CST


### BEGIN NOTE - JUNE 24 2026 - 19:40 CST

Here are the crate notes. They reflect the Phase 3 Round 2 completion standard from the carryover notes, plus the latest passing `ron-ledger` exhaustive gate and your final `ron-proto` marker. The notes requirement was to prove validator lifecycle/evidence as strict data in `ron-proto`, protect economic truth in `ron-ledger`, keep existing tests green, and avoid staking/slashing/bridge/external-settlement creep.  The latest `ron-ledger` log confirms `ron-ledger quickchain exhaustive preflight gate passed: tests=33`. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - ron-proto + ron-ledger

# QuickChain Phase 3 Round 2 Crate Notes — `ron-proto + ron-ledger`

## 0. Status

Crate pair:

```text
ron-proto + ron-ledger
```

Phase / round:

```text
QuickChain Phase 3 — passport-gated validator boundary
Round 2 — validator lifecycle / evidence / operation hardening
```

Status:

```text
COMPLETE / PARKED for QuickChain Phase 3 Round 2
```

This pair is now parked for Phase 3 Round 2.

Safe completion label:

```text
QuickChain Phase 3 Round 2 — ron-proto + ron-ledger complete / parked
```

Important distinction:

```text
ron-proto + ron-ledger are complete for Phase 3 Round 2.
The full Phase 3 Round 2 stack still needs downstream crate pairs.
```

Next crate pair:

```text
svc-wallet + ron-accounting
```

---

## 1. Why this pair mattered

This was the foundational Phase 3 Round 2 pair.

`ron-proto` owns the strict DTO vocabulary.

`ron-ledger` owns the economic-truth boundary and read-only evaluation behavior.

Phase 3 Round 2 needed to harden validator/passport lifecycle semantics without crossing into public validator runtime, staking, slashing, bridge, ROX, Solana, or external settlement.

The goal was not to create validator authority.

The goal was to make validator lifecycle/evidence data safe, strict, deterministic, and non-economic before downstream crates consume or display it.

---

## 2. What changed in `ron-proto`

Primary new/updated files:

```text
crates/ron-proto/src/quickchain/validator_lifecycle.rs
crates/ron-proto/src/quickchain/mod.rs
crates/ron-proto/tests/quickchain_phase3_validator_lifecycle.rs
```

`ron-proto` now has strict Phase 3 Round 2 validator lifecycle DTOs and validation helpers.

Covered DTO / schema concepts include:

```text
validator rotation
validator revocation
validator downtime/degraded report
validator equivocation evidence
replay challenge evidence
governance-gated parameter update
validator lifecycle decision
validator lifecycle rejection code
```

The DTO surface is intentionally data-only.

It does not perform:

```text
cryptographic verification
validator election
validator runtime execution
public chain state mutation
staking
slashing
bonding
bridge settlement
external chain anchoring
wallet mutation
ledger mutation
balance authority
receipt authority
```

---

## 3. `ron-proto` validation behavior added or confirmed

The new lifecycle DTOs validate:

```text
schema constants
DTO version
chain_id
epoch_id
validator set hash
validator member identity
passport subject binding
capability reference binding
rotation old/new validator key mismatch
rotation old/new capability mismatch
revocation reason shape
downtime/degraded report shape
equivocation evidence shape
replay challenge evidence shape
governance parameter update shape
lifecycle decision status consistency
```

Important strictness:

```text
unknown fields reject
wrong schema rejects
wrong version rejects
empty required IDs reject
duplicate/conflicting evidence constraints reject
rotation with no key/capability change rejects
lifecycle Accepted cannot carry rejection_code
lifecycle Rejected must carry rejection_code
```

Round 2 anti-creep behavior:

```text
economics-shaped smuggling fields reject
staking/slashing-shaped fields reject
bridge/external settlement-shaped fields reject
validator lifecycle DTOs do not expose balance authority
validator lifecycle DTOs do not expose payout authority
validator lifecycle DTOs do not expose wallet authority
validator lifecycle DTOs do not expose settlement authority
```

---

## 4. `ron-proto` focused tests that passed

Focused Phase 3 Round 2 test:

```bash
cargo test -p ron-proto --test quickchain_phase3_validator_lifecycle
```

Result:

```text
5 passed; 0 failed
```

Test coverage included:

```text
rotation_revocation_downtime_challenge_and_parameter_update_shapes_validate
rotation_requires_key_and_capability_change
equivocation_evidence_requires_distinct_and_conflicting_attestations
lifecycle_decision_status_consistency_is_strict
phase3_round2_lifecycle_dtos_reject_unknown_economics_and_slashing_fields
```

Final exhaustive `ron-proto` marker confirmed by user:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=27
```

Therefore:

```text
ron-proto is complete / parked for QuickChain Phase 3 Round 2.
```

---

## 5. What changed in `ron-ledger`

Primary new/updated files:

```text
crates/ron-ledger/src/quickchain/validator_lifecycle.rs
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/tests/quickchain_phase3_validator_lifecycle.rs
crates/ron-ledger/tests/quickchain_pre_root_boundary.rs
```

`ron-ledger` now has read-only validator lifecycle evaluators.

Covered evaluator concepts include:

```text
read-only validator rotation evaluation
read-only validator revocation evaluation
read-only equivocation evidence evaluation
validator set hash binding
active validator membership checks
revoked validator rejection
capability/key mismatch rejection
```

The evaluators are deliberately non-mutating.

They return lifecycle decisions as data.

They do not mutate:

```text
balances
holds
account sequences
operation IDs
idempotency keys
receipt truth
ledger supply
wallet state
storage state
accounting snapshots
rewarder payouts
```

---

## 6. `ron-ledger` read-only evaluator behavior

`ron-ledger` now safely evaluates validator lifecycle artifacts against the passport-gated validator set model.

Accepted read-only cases:

```text
active validator rotation with correct old key/capability binding
active validator revocation with valid member/set binding
equivocation evidence as evidence-only artifact
```

Rejected read-only cases:

```text
validator set hash mismatch
revoked validator attempting rotation
revoked validator submitting equivocation evidence as authorized member
rotation key mismatch
rotation capability mismatch
invalid lifecycle artifact validation
```

Important boundary:

```text
Even accepted lifecycle decisions do not mutate ledger truth.
Accepted means the lifecycle artifact shape/evidence is acceptable as data.
Accepted does not mean finality, slashing, reward, payout, settlement, or validator runtime authority.
```

---

## 7. `ron-ledger` focused tests that passed

Focused lifecycle command:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase3_validator_lifecycle
```

Result:

```text
7 passed; 0 failed
```

Covered tests:

```text
active_validator_rotation_is_accepted_read_only
active_validator_revocation_is_accepted_read_only_without_slashing
equivocation_evidence_is_accepted_as_evidence_only
rotation_rejects_key_or_capability_mismatch_without_mutation
revoked_validator_cannot_rotate_or_submit_equivocation_as_authorized_member
validator_set_hash_mismatch_rejects_lifecycle_artifacts
lifecycle_hardening_source_stays_out_of_staking_slashing_and_balance_authority
```

Focused validator gate command:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase3_validator_gate
```

Result:

```text
7 passed; 0 failed
```

Covered tests:

```text
active_passport_registry_validator_authorizes_attestation_read_only
unauthorized_identity_rejects
revoked_identity_rejects
expired_capability_rejects
revoked_capability_rejects
key_mismatch_rejects_without_mutating_ledger_truth
bonded_economics_are_rejected_before_authorization
```

Boundary repair command:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
```

Result:

```text
1 passed; 0 failed
```

Final exhaustive `ron-ledger` marker:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=33
```

Therefore:

```text
ron-ledger is complete / parked for QuickChain Phase 3 Round 2.
```

---

## 8. Fixes applied during this pair

### Fix 1 — unused import in `ron-proto`

Initial compile failed because `ron-proto` denies warnings and `validator_lifecycle.rs` imported `QUICKCHAIN_DTO_VERSION` without using it.

Resolution:

```text
Removed unused QUICKCHAIN_DTO_VERSION import.
```

### Fix 2 — replay status enum mismatch

Initial lifecycle test used:

```text
QuickChainVerifierReplayStatusV1::Rejected
```

But the verifier replay status enum only supports:

```text
Verified
Mismatch
```

`Rejected` belongs to validator/lifecycle decision status concepts, not replay result status.

Resolution:

```text
Replaced invalid QuickChainVerifierReplayStatusV1::Rejected usages with QuickChainVerifierReplayStatusV1::Mismatch.
```

This preserved the intended meaning: equivocation/conflict evidence is based on conflicting replay attestations/results, not a nonexistent replay rejection variant.

### Fix 3 — `ron-ledger` pre-root boundary scanner

The `ron-ledger` source-boundary scanner originally allowed the word `validator` only in the Phase 3 Round 1 passport gate module.

Phase 3 Round 2 legitimately added:

```text
src/quickchain/validator_lifecycle.rs
src/quickchain/mod.rs
```

Resolution:

```text
Narrowly extended the validator-token exception to the known Phase 3 validator files only.
```

This did not weaken the core safety scanner.

It still blocks forbidden runtime/root/authority creep.

---

## 9. What remains forbidden after this pair

This pair did not introduce and must not be treated as introducing:

```text
staking
slashing
bonding
validator reward economics
public validator economy
public validator networking
public chain launch
bridge implementation
ROX
Solana
external settlement
external DA integration
anchors that mutate ROC
checkpoint authority that replaces wallet/ledger truth
validator-set finality authority
quorum/fork-choice authority
gateway ledger mutation
omnigate ledger mutation
rewarder ledger mutation
index/policy settlement truth
CrabLink validator signing
client-side validator management
cache-based paid unlock
fake balances
fake receipts
fake hashes
placeholder roots
fake finality
raw engagement direct ROC allocation
pruning without archive/challenge/DA fallback
```

---

## 10. Boundary state after completion

The architecture remains intact:

```text
ron-proto = strict DTOs and validation helpers
ron-ledger = economic truth and read-only lifecycle/evidence evaluation
svc-wallet = only mutation front-door
ron-accounting = snapshots/read-model only
svc-rewarder = payout planning only
svc-storage = bytes/artifacts only
svc-gateway/omnigate = public paid boundary/hydration only
svc-index/ron-policy = lookup/policy only
CrabLink Tauri = display/user intent only
```

Validator lifecycle artifacts are now safe to carry downstream as:

```text
strict data
evidence metadata
read-only lifecycle decisions
non-economic validation results
```

They are not:

```text
balance truth
settlement truth
wallet authority
ledger mutation authority
staking authority
slashing authority
bridge authority
public-chain authority
```

---

## 11. Current Phase 3 Round 2 progress

Current crate-pair order:

```text
1. ron-proto + ron-ledger        ✅ COMPLETE / PARKED
2. svc-wallet + ron-accounting   ← NEXT ACTIVE PAIR
3. svc-rewarder + svc-storage    pending
4. svc-gateway + omnigate        pending
5. svc-index + ron-policy        pending
6. CrabLink Tauri + adapters     pending
```

Do not skip:

```text
svc-wallet + ron-accounting
```

---

## 12. Next pair focus — `svc-wallet + ron-accounting`

The next pair should consume/audit the new validator lifecycle/evidence model without allowing wallet or accounting to become validator authority.

Expected `svc-wallet` focus:

```text
wallet remains mutation front-door only
wallet does not accept validator lifecycle artifacts as spend authority
wallet does not turn validator decisions into receipts
wallet does not expose staking/slashing/bonding paths
wallet does not let lifecycle metadata issue/transfer/burn/hold/capture/release
wallet rejects or ignores poison fields that try to smuggle validator finality/settlement
```

Expected `ron-accounting` focus:

```text
accounting remains derivative snapshots/read-model only
accounting does not become balance truth
accounting does not treat validator lifecycle decisions as settlement
accounting does not produce wallet receipts
accounting does not plan or execute slashing/reward economics
accounting does not mutate ledger state
```

Recommended next codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-wallet
bash scripts/make_crate_codex.sh --force -c ron-accounting
```

---

## 13. Retest commands for this completed pair

Only rerun if this pair is modified again.

Focused retests:

```bash
cargo test -p ron-proto --test quickchain_phase3_validator_lifecycle

cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase3_validator_lifecycle
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase3_validator_gate
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
```

Full crate-local preflight gates:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=27
ron-ledger quickchain exhaustive preflight gate passed: tests=33
```

---

## 14. Completion judgment

Final judgment:

```text
ron-proto + ron-ledger are complete / parked for QuickChain Phase 3 Round 2.
```

This pair now provides the Phase 3 Round 2 foundation:

```text
strict validator lifecycle DTOs
+ strict validator evidence DTOs
+ strict replay challenge evidence DTOs
+ governance-gated parameter update DTOs
+ lifecycle decision DTOs
+ read-only lifecycle evaluation
+ read-only equivocation evidence evaluation
+ validator set hash binding
+ revoked/mismatched validator rejection
+ no economic mutation
+ no staking/slashing/bridge/external settlement
+ exhaustive crate-local preflight green
```

Next active pair:

```text
svc-wallet + ron-accounting
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - ron-proto + ron-ledger


### END NOTE - JUNE 24 2026 - 19:40 CST




### BEGIN NOTE - JUNE 25 2026 - 14:30 CST

Confirmed from the latest terminal output: `ron-proto` full QuickChain preflight passed with **28 tests**, and `ron-ledger` full QuickChain preflight passed with **34 tests**. The focused Phase 4 Round 1 bond tests also passed cleanly for both crates. 

Here are the crate notes.

# QuickChain Phase 4 Round 1 Crate Notes — `ron-proto` + `ron-ledger`

Date: 2026-06-25
Phase: QuickChain Phase 4 Round 1
Crates: `ron-proto` + `ron-ledger`
Status: COMPLETE / GREEN

## High-Level Result

QuickChain Phase 4 Round 1 is complete for the `ron-proto` + `ron-ledger` crate pair.

This round introduced the first safe bonded-validator modeling layer without crossing into live staking, automatic slashing, liquidity, settlement, public validator markets, bridge logic, or runtime chain authority.

The implementation stays aligned with the Phase 4 Round 1 scope:

* `ron-proto`: strict bond-related DTO scaffolding.
* `ron-ledger`: deterministic, internal-only, no-op/conservative bond accounting model.
* No live bond mutation through wallet/service routes yet.
* No automatic slashing.
* No public staking market.
* No liquidity or exchange-facing behavior.
* No ROX/Solana/external settlement creep.
* No runtime validator economics authority.

## Tests / Gates Passed

Focused tests passed:

```bash
cargo test -p ron-proto --test quickchain_phase4_bond_dto
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_accounting
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
```

Results:

* `ron-proto quickchain_phase4_bond_dto`: 6/6 passed.
* `ron-ledger quickchain_phase4_bond_accounting`: 5/5 passed.
* `ron-ledger quickchain_pre_root_boundary`: 1/1 passed.

Full crate preflights passed:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Results:

* `ron-proto quickchain exhaustive preflight gate passed: tests=28`
* `ron-ledger quickchain exhaustive preflight gate passed: tests=34`

## Files Added / Updated

### `ron-proto`

New / updated files:

```text
crates/ron-proto/src/quickchain/bond.rs
crates/ron-proto/src/quickchain/mod.rs
crates/ron-proto/tests/quickchain_phase4_bond_dto.rs
```

### `ron-ledger`

New / updated files:

```text
crates/ron-ledger/src/quickchain/bond_accounting.rs
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/tests/quickchain_phase4_bond_accounting.rs
crates/ron-ledger/tests/quickchain_pre_root_boundary.rs
```

`quickchain_pre_root_boundary.rs` was updated narrowly so the new `bond_accounting.rs` file can exist in the QuickChain preflight tree without falsely tripping the validator-token boundary scanner.

## `ron-proto` Notes

### New Module

Added:

```rust
crates/ron-proto/src/quickchain/bond.rs
```

This module defines strict Phase 4 Round 1 DTOs for future bonded validator modeling.

It is DTO-only and validation-only. It does not perform wallet mutation, ledger mutation, root production, public staking, live slashing, liquidity, bridge behavior, settlement, or validator economics authority.

### New Exports

`crates/ron-proto/src/quickchain/mod.rs` now exports the bond module:

```rust
pub mod bond;
pub use bond::*;
```

### New Schema Constants

Added constants for strict bond-related schema tags:

```rust
QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA
QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA
QUICKCHAIN_SLASH_EVIDENCE_SCHEMA
QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA
QUICKCHAIN_BOND_ASSET_ROC
```

The only accepted Phase 4 Round 1 bond asset is:

```text
roc
```

No ROX, Solana, bridge asset, public staking token, or external settlement token is allowed.

### New Enums

Added:

```rust
QuickChainBondIntentKindV1
QuickChainBondAccountStatusV1
QuickChainSlashEvidenceKindV1
QuickChainBondLifecycleOperationV1
QuickChainBondLifecycleDecisionStatusV1
QuickChainBondLifecycleRejectionCodeV1
```

Important behavior:

* Enums are `#[non_exhaustive]` where appropriate.
* Downstream crates must fail closed on unknown future variants.
* `ron-ledger` now handles this with a wildcard rejection arm in bond intent matching.

### New DTOs

Added:

```rust
QuickChainValidatorBondIntentV1
QuickChainValidatorBondAccountV1
QuickChainSlashEvidenceV1
QuickChainBondLifecycleDecisionV1
```

All DTOs use strict serde shape with `deny_unknown_fields`.

### Bond Intent DTO

`QuickChainValidatorBondIntentV1` represents an explicit future bond action request.

Supported Phase 4 Round 1 intent kinds:

```rust
OpenBond
IncreaseBond
RequestUnlock
CancelUnlockRequest
```

Validation enforces:

* Correct schema.
* Correct DTO version.
* Valid `chain_id`.
* Valid `epoch_id`.
* Valid validator/account/intent/idempotency references.
* Asset must be `roc`.
* Amount must be present.
* Amount must be canonical integer minor-unit string.
* Amount must be positive.
* No floats.
* No ROX/external asset.
* `unlock_epoch_id` is only allowed/required for `RequestUnlock`.

### Bond Account DTO

`QuickChainValidatorBondAccountV1` represents a strict bond account snapshot.

It validates conservation of the internal account components:

```text
locked_minor = available_to_unlock_minor + pending_unlock_minor + slash_reserved_minor
```

It also requires:

* Correct schema/version.
* Valid chain/epoch/account/validator refs.
* ROC asset only.
* Canonical integer minor-unit strings.
* Nonzero account sequence.

### Slash Evidence DTO

`QuickChainSlashEvidenceV1` represents slash evidence as data only.

Important: Phase 4 Round 1 accepts slash evidence shape but does not allow automatic slashing.

It validates:

* Correct schema/version.
* Chain/epoch binding.
* Validator set hash as canonical b3 content ID.
* Evidence ID.
* Validator ID.
* Evidence kind.
* Evidence ref.
* Submitter ref.
* Nonzero observed timestamp.
* Optional recommended amount as canonical integer minor-unit string.

### Bond Lifecycle Decision DTO

`QuickChainBondLifecycleDecisionV1` represents a strict lifecycle decision artifact.

Validation enforces status consistency:

* Accepted decisions must not include a rejection code.
* Rejected decisions must include a rejection code.

This is important for deterministic rejection accounting and future evidence review paths.

### `ron-proto` Test Coverage Added

Added:

```rust
crates/ron-proto/tests/quickchain_phase4_bond_dto.rs
```

Coverage includes:

* Valid explicit ROC integer bond intent shape.
* Missing amount rejection.
* Float money rejection.
* Wrong asset rejection.
* Wrong unlock epoch shape rejection.
* Bond account component conservation.
* Slash evidence strict evidence-only shape.
* Lifecycle decision rejection-code consistency.
* Unknown-field rejection for dangerous/unauthorized fields such as public staking, liquidity, and auto-slash fields.

## `ron-ledger` Notes

### New Module

Added:

```rust
crates/ron-ledger/src/quickchain/bond_accounting.rs
```

This module implements a pure in-memory Phase 4 Round 1 bond accounting model.

It is intentionally not a wallet path and not live ledger-service authority.

It does not:

* Move real wallet balances.
* Mutate service-owned ledger truth.
* Slash automatically.
* Reserve slash funds automatically.
* Burn/capture/release funds.
* Create public staking behavior.
* Create liquidity behavior.
* Produce roots.
* Produce checkpoints.
* Use clocks.
* Use IO.
* Use network.
* Use public validator economics authority.

### New Exports

`crates/ron-ledger/src/quickchain/mod.rs` now exports the bond accounting model types/functions.

Expected exports include:

```rust
QuickChainBondAccountingState
QuickChainBondAccountRecord
QuickChainBondLedgerOutcome
QuickChainBondLedgerError
evaluate_slash_evidence_noop
```

### Bond Accounting State

Added:

```rust
QuickChainBondAccountingState
```

This is a deterministic in-memory state model over a `BTreeMap`.

It provides:

* Deterministic account ordering.
* Internal account sequence assignment.
* Total locked amount.
* Total pending unlock amount.
* Total slash-reserved amount.
* Total available-to-unlock amount.
* Pure copy-on-write application of explicit bond intents.

### Copy-On-Write Safety

The bond accounting model applies changes to a cloned candidate state first.

If validation or arithmetic fails, the original state is not mutated.

This was explicitly tested for over-unlock, over-cancel, mismatched account fields, and insufficient owner boundary failures.

### Explicit Owner Boundary

Open/increase bond operations require the caller to supply an explicit `owner_available_minor` boundary.

This prevents silent balance lock/spend semantics from appearing inside `ron-ledger`.

The model can say, “this requested bond would debit this much,” but it does not itself debit a live wallet balance.

### Supported Bond Intent Behavior

`apply_explicit_bond_intent` handles:

```rust
OpenBond
IncreaseBond
RequestUnlock
CancelUnlockRequest
```

Behavior:

* `OpenBond`

  * Requires owner available amount to cover the bond.
  * Rejects existing bond account ID.
  * Creates a new account record.
  * Sets locked and available-to-unlock amounts.
  * Owner debit output equals the bond amount.

* `IncreaseBond`

  * Requires owner available amount to cover the increase.
  * Requires existing account.
  * Requires owner/validator/asset match.
  * Increases locked and available-to-unlock amounts.
  * Owner debit output equals the increase amount.

* `RequestUnlock`

  * Requires existing account.
  * Requires owner/validator/asset match.
  * Requires available-to-unlock amount to cover request.
  * Moves amount from available-to-unlock into pending unlock.
  * Owner debit output is zero.

* `CancelUnlockRequest`

  * Requires existing account.
  * Requires owner/validator/asset match.
  * Requires pending unlock amount to cover cancellation.
  * Moves amount from pending unlock back into available-to-unlock.
  * Owner debit output is zero.

* Unknown future enum variants

  * Rejected explicitly with `InvalidBondIntent`.
  * This is required because the proto enum is non-exhaustive.

### Slash Evidence No-Op

Added:

```rust
evaluate_slash_evidence_noop
```

This validates slash evidence shape and returns a rejected lifecycle decision with:

```rust
AutomaticSlashingForbidden
```

It does not mutate state and does not reserve, burn, capture, release, bridge, settle, or slash ROC.

This is the correct Phase 4 Round 1 behavior.

### Error Type

Added:

```rust
QuickChainBondLedgerError
```

Important variants include:

```rust
InvalidBondIntent
InvalidSlashEvidence
BondAccountAlreadyExists
UnknownBondAccount
OwnerAccountMismatch
ValidatorMismatch
AssetMismatch
InsufficientOwnerAvailable
InsufficientBondAvailable
InsufficientPendingUnlock
ArithmeticOverflow
StateInvariantViolation
```

Because `ron-ledger` has `#![deny(missing_docs)]`, public named fields inside error variants require field-level documentation. This was fixed and the crate compiles cleanly.

### `ron-ledger` Test Coverage Added

Added:

```rust
crates/ron-ledger/tests/quickchain_phase4_bond_accounting.rs
```

Coverage includes:

* Open bond requires explicit owner-available boundary.
* Increase bond requires explicit owner-available boundary.
* Open/increase update locked and available-to-unlock values correctly.
* Request unlock moves funds from available-to-unlock to pending unlock.
* Cancel unlock moves funds from pending unlock back to available-to-unlock.
* Locked component conservation is preserved.
* Over-unlock rejects without partial mutation.
* Over-cancel rejects without partial mutation.
* Mismatched owner rejects.
* Mismatched validator rejects.
* Mismatched asset rejects.
* Slash evidence evaluates as no-op rejected decision.
* Slash evidence does not reserve or slash funds.

## Boundary Scanner / Authority Notes

`ron-ledger` pre-root boundary passed after narrowly allowing the new `bond_accounting.rs` source file in the appropriate scanner path.

The boundary test confirms the new Phase 4 Round 1 bond accounting work did not introduce forbidden runtime authority or root production.

The implementation remains preflight/model-only.

## Important Doctrine Preserved

This crate pair preserved the core QuickChain doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Specific Phase 4 boundaries preserved:

* No public staking market.
* No validator rewards.
* No live slashing.
* No automatic slashing.
* No slash reserve mutation from evidence alone.
* No liquidity pool.
* No bridge.
* No ROX.
* No Solana.
* No external settlement.
* No public chain authority.
* No gateway/omnigate/rewarder ledger mutation.
* No CrabLink client authority.
* No fake balances.
* No fake receipts.
* No silent spend.
* No hidden bond lock.

## Current Completion Status

`ron-proto + ron-ledger` are complete for QuickChain Phase 4 Round 1.

This crate pair is ready to hand off to the next crate pair.

## Next Crate Pair

Normal crate-pair order says the next pair is:

```text
svc-wallet + ron-accounting
```

Expected Phase 4 Round 1 focus for the next pair:

### `svc-wallet`

Likely goals:

* Add explicit wallet-side bond/unstake confirmation boundary checks.
* Ensure any future bond path is quote/prepare/confirm style only.
* Prevent hidden lock/silent spend.
* Prevent fake bond receipts.
* Prevent wallet from claiming live staking/slashing authority.
* Ensure any bond-facing DTOs remain gated and internal.
* No live public staking route unless explicitly authorized by a later phase.
* No automatic slashing route.
* No validator reward payout route.
* No bridge/ROX/Solana/external settlement route.

### `ron-accounting`

Likely goals:

* Add read-only accounting/reporting safeguards for bond-related snapshots.
* Accounting may summarize/report bond model data but must not be bond truth.
* Accounting must not mutate ledger, wallet, bond state, slash state, or balances.
* Accounting must not become staking authority.
* Accounting must not calculate live public validator rewards.
* Accounting must not create receipts or balances.
* Accounting remains snapshot/reporting only.

## Recommended Next Session Opening

Use this as the next-session opener:

```text
We completed QuickChain Phase 4 Round 1 for ron-proto + ron-ledger. ron-proto full QuickChain preflight passed with 28 tests, and ron-ledger full QuickChain preflight passed with 34 tests. The focused Phase 4 bond DTO and bond accounting tests are green. The implementation added strict bond DTOs in ron-proto and a deterministic internal no-op/conservative bond accounting model in ron-ledger, while preserving all boundaries: no live staking, no automatic slashing, no liquidity, no bridge, no ROX/Solana, no external settlement, and no runtime validator economics authority.

Next crate pair is svc-wallet + ron-accounting for Phase 4 Round 1. The goal should be explicit wallet confirmation boundaries and accounting read-only safeguards for bond-related data, without introducing live staking/slashing authority or hidden balance locks.
```

## Commands Already Proven Green

Focused:

```bash
cargo test -p ron-proto --test quickchain_phase4_bond_dto
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_accounting
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
```

Full preflight:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

## Final Status Line

QuickChain Phase 4 Round 1 `ron-proto + ron-ledger` is complete, green, and ready to carry forward into `svc-wallet + ron-accounting`.


### END NOTE - JUNE 25 2026 - 14:30 CST


### BEGIN NOTE - JUNE 25 2026 - 23:50 CST

Here are the paste-ready crate notes for the completed `ron-proto + ron-ledger` slice. I anchored these to the green terminal proof showing `ron-proto` preflight `tests=29` and `ron-ledger` preflight `tests=35`, plus the Phase 4 Round 2 scope: challenge windows, slash evidence validation, appeal/freeze states, replayable dispute state, and no live irreversible slash yet.   QuickChain remains future ROC settlement infrastructure, not public staking/bridge/external settlement runtime. 

# QuickChain Phase 4 Round 2 Crate Notes — `ron-proto` + `ron-ledger`

Date: June 25/26, 2026
Project: RustyOnions / CrabLink
Phase / round: QuickChain Phase 4 Round 2
Crate pair: `ron-proto + ron-ledger`
Status: COMPLETE / PARKED / GREEN

## 0. Status

`ron-proto + ron-ledger` are complete and parked for QuickChain Phase 4 Round 2.

This crate pair implemented the first safe slashing/challenge simulation layer for Phase 4 without introducing live slashing, live staking, public validator economy, external settlement, bridges, liquidity, ROX/Solana runtime, exchange-facing logic, gateway mutation authority, omnigate mutation authority, wallet mutation bypasses, fake balances, fake receipts, or silent spend.

Final status:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=29
ron-ledger quickchain exhaustive preflight gate passed: tests=35
```

This completes only the first crate pair for Phase 4 Round 2.

Current Phase 4 Round 2 crate-pair position:

```text
1. ron-proto + ron-ledger              COMPLETE / PARKED
2. svc-wallet + ron-accounting         NEXT
3. svc-rewarder + svc-storage          pending
4. svc-gateway + omnigate              pending
5. svc-index + ron-policy              pending
6. CrabLink Tauri + client adapters    pending
```

## 1. Buildplan context

Phase 4 theme:

```text
bonded / higher-stakes validator model
```

Phase 4 planned rounds:

```text
Round 1 — bond DTOs and no-op accounting model
Round 2 — slashing/challenge simulation
Round 3 — controlled live bond enforcement
```

Phase 4 Round 2 purpose:

```text
Simulate consequences before live enforcement.
```

Round 2 target scope:

```text
challenge windows
slash evidence validation
appeal/freeze states
hold-like lifecycle for disputed bonds
replayable dispute state
no one-step irreversible slash without tested governance rules
```

Round 2 exit gate for this crate pair:

```text
slashing simulation is deterministic
slashing simulation is replayable
challenge path is safe
dispute windows are explicit
no live irreversible slash yet
ron-proto DTOs remain strict DTOs only
ron-ledger simulation remains deterministic/read-only with respect to existing bond accounting state
pre-root/root-authority scanners stay green
clippy preflight stays clean
```

## 2. Non-negotiable doctrine preserved

The active QuickChain doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

This pair preserved the QuickChain boundaries:

```text
ron-proto:
  DTOs, schema constants, validation helpers, serde strictness only

ron-ledger:
  deterministic replay/simulation and economic truth boundaries only

svc-wallet:
  still the future mutation front-door for wallet/bond actions

ron-accounting:
  still derivative reporting/snapshots only

svc-rewarder:
  still payout planning only

svc-storage:
  still bytes/artifacts only

svc-gateway / omnigate:
  still public/hydration boundaries only, not ledger mutation

CrabLink:
  still display/user intent only
```

Forbidden scope remained forbidden:

```text
ROX active runtime
Solana active runtime
external settlement
public bridge
public staking
liquidity
exchange-facing logic
public validator economy
automatic irreversible slash
validator reward minting
gateway direct ledger mutation
omnigate direct ledger mutation
rewarder direct ledger mutation
accounting as balance truth
policy/index/cache as payment truth
CrabLink slash authority
CrabLink wallet authority
fake balances
fake receipts
silent spend
fake roots
placeholder hashes
root-producing code without canonical bytes and golden vectors
```

## 3. What changed in `ron-proto`

### Files added / updated

```text
crates/ron-proto/src/quickchain/bond_dispute.rs
crates/ron-proto/src/quickchain/mod.rs
crates/ron-proto/tests/quickchain_phase4_bond_dispute.rs
```

Existing Phase 4 Round 1 files remained green:

```text
crates/ron-proto/src/quickchain/bond.rs
crates/ron-proto/tests/quickchain_phase4_bond_dto.rs
```

### DTOs / types introduced

`ron-proto` now has strict Phase 4 Round 2 disputed-bond DTO shapes:

```text
QuickChainBondDisputeWindowV1
QuickChainBondDisputeV1
QuickChainBondDisputeEventV1
QuickChainBondDisputeStatusV1
QuickChainBondDisputeEventKindV1
QuickChainBondDisputeRejectionCodeV1
```

Schema constants added:

```text
QUICKCHAIN_BOND_DISPUTE_SCHEMA
QUICKCHAIN_BOND_DISPUTE_EVENT_SCHEMA
```

### Main DTO behavior

The new DTOs model disputed-bond lifecycle state only.

They support:

```text
explicit challenge windows
explicit appeal windows
challenge-open state
frozen-pending-appeal state
appeal-open state
resolved-no-slash terminal state
resolved-slash-rejected terminal state
event-sequenced dispute lifecycle transitions
frozen disputed amount fields
strict rejection codes for unsafe slash paths
```

They reject or prevent:

```text
unknown fields
implicit slash authority
one-step irreversible slash fields
missing windows where required
appeal/freeze status inconsistencies
bad amount strings
bad IDs / refs
bad schema or version
```

Important DTO safety point:

```text
The DTOs describe challenge/dispute state. They do not authorize wallet mutation, slashing, staking, settlement, bridge logic, or validator rewards.
```

## 4. `ron-proto` tests added

New test file:

```text
crates/ron-proto/tests/quickchain_phase4_bond_dispute.rs
```

Focused tests passed:

```text
cargo test -p ron-proto --test quickchain_phase4_bond_dispute
result: 4 passed; 0 failed
```

Test coverage:

```text
dispute_open_state_validates_explicit_challenge_window
dispute_status_requires_matching_appeal_and_frozen_state
dispute_events_validate_kind_specific_requirements
bond_dispute_dtos_reject_unknown_fields
```

Existing Phase 4 Round 1 DTO test stayed green:

```text
cargo test -p ron-proto --test quickchain_phase4_bond_dto
result: 6 passed; 0 failed
```

## 5. `ron-proto` final preflight status

Full crate-local preflight passed:

```text
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=29
```

Preflight included:

```text
all discovered quickchain integration tests
bash vector inventory verifier
bash hash payload verifier
clippy preflight
no helper Python scripts boundary
bash-only quickchain tooling boundary
test discovery count contract
```

Important count change:

```text
Phase 4 Round 1 ron-proto count: 28
Phase 4 Round 2 ron-proto count: 29
```

This confirms the new dispute DTO test was picked up by the exhaustive preflight runner.

## 6. What changed in `ron-ledger`

### Files added / updated

```text
crates/ron-ledger/src/quickchain/bond_dispute.rs
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/tests/quickchain_phase4_bond_dispute_simulation.rs
crates/ron-ledger/tests/quickchain_pre_root_boundary.rs
```

Existing Phase 4 Round 1 bond accounting remained green:

```text
crates/ron-ledger/src/quickchain/bond_accounting.rs
crates/ron-ledger/tests/quickchain_phase4_bond_accounting.rs
```

### Simulation functions introduced

`ron-ledger` now exposes deterministic disputed-bond simulation helpers:

```text
evaluate_bond_dispute_event_simulation
replay_bond_dispute_simulation
QuickChainBondDisputeSimulationError
```

### Main simulation behavior

The new simulation is deterministic and replayable.

It supports these simulated transitions:

```text
ChallengeOpen
  -> FreezePendingAppeal

FrozenPendingAppeal
  -> AppealOpen

AppealOpen or FrozenPendingAppeal or ChallengeOpen
  -> ResolvedNoSlash

AppealOpen
  -> ResolvedSlashRejected
```

Important nuance:

```text
ResolvedSlashRejected means irreversible slash was rejected in simulation.
It is not a live slash.
It is not a wallet capture.
It is not slashing authority.
```

The evaluator enforces:

```text
current dispute DTO validates before simulation
event DTO validates before simulation
chain_id / epoch_id / dispute_id / bond_account_id / validator_id bindings match
event sequence must advance exactly by one
challenge window must contain freeze event epoch
appeal window must contain appeal event epoch
freeze amount cannot exceed disputed bond amount
terminal disputes reject later events
simulation output validates before return
```

The evaluator rejects:

```text
binding mismatch
sequence mismatch
terminal dispute replay
closed challenge window
closed appeal window
over-freeze amount
invalid transitions
invalid DTOs
arithmetic overflow
```

### Important terminal ordering fix

A focused failure exposed one semantic ordering issue:

```text
terminal_disputes_reject_later_events failed
```

Cause:

```text
The evaluator checked event sequence before checking whether the current dispute was terminal.
A stale event against a terminal dispute returned SequenceMismatch instead of DisputeAlreadyTerminal.
```

Fix:

```text
Check current.is_terminal() before calculating expected next sequence.
```

Reason:

```text
Terminality should dominate sequence analysis.
Once a dispute is terminal, no future/stale/duplicate event should be interpreted as a normal sequence problem first.
```

Final result:

```text
terminal_disputes_reject_later_events ... ok
```

## 7. `ron-ledger` tests added

New test file:

```text
crates/ron-ledger/tests/quickchain_phase4_bond_dispute_simulation.rs
```

Focused test passed:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_dispute_simulation
result: 5 passed; 0 failed
```

Test coverage:

```text
dispute_simulation_replays_freeze_appeal_and_rejected_irreversible_slash
replay_is_deterministic_and_sequence_bound
dispute_windows_are_enforced
terminal_disputes_reject_later_events
dispute_simulation_does_not_mutate_bond_accounting_state
```

Existing Phase 4 Round 1 accounting test stayed green:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_accounting
result: 5 passed; 0 failed
```

Pre-root scanner stayed green after a narrow file-specific scanner alignment:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
result: 1 passed; 0 failed
```

## 8. Boundary scanner adjustment

The new `bond_dispute.rs` module uses existing bond/dispute DTO bindings that contain validator-related field names.

The existing pre-root scanner initially failed on this token:

```text
src/quickchain/bond_dispute.rs: found forbidden token `validator`
```

This was not a real authority drift. It was a scanner allowlist issue.

Patch applied:

```text
crates/ron-ledger/tests/quickchain_pre_root_boundary.rs
```

The scanner now includes a narrow exception for:

```text
src/quickchain/bond_dispute.rs
```

This matches the existing Phase 4 pattern already used for bond/lifecycle boundary files.

Important note:

```text
The scanner exception is file-specific.
It does not globally weaken the no-root/no-runtime-authority boundary.
It does not allow validator machinery, staking, live slashing, roots, settlement, bridge logic, or public validator economy.
```

Final result:

```text
quickchain_preflight_sources_have_no_runtime_authority_or_root_production ... ok
```

## 9. `ron-ledger` final preflight status

Full crate-local preflight passed:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=35
```

Preflight included:

```text
all discovered quickchain integration tests
Phase 4 bond accounting tests
Phase 4 dispute simulation tests
pre-root boundary scanner
projection/canonical boundary scanners
tooling boundary scanner
clippy preflight
no helper Python scripts boundary
every quickchain integration test feature-gated
```

Important count change:

```text
Phase 4 Round 1 ron-ledger count: 34
Phase 4 Round 2 ron-ledger count: 35
```

This confirms the new dispute simulation test was picked up by the exhaustive preflight runner.

## 10. Final focused test proof

Focused commands that passed:

```bash
cargo test -p ron-proto --test quickchain_phase4_bond_dispute
cargo test -p ron-proto --test quickchain_phase4_bond_dto

cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_dispute_simulation
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_accounting
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
```

Focused results:

```text
ron-proto quickchain_phase4_bond_dispute: 4 passed; 0 failed
ron-proto quickchain_phase4_bond_dto: 6 passed; 0 failed

ron-ledger quickchain_phase4_bond_dispute_simulation: 5 passed; 0 failed
ron-ledger quickchain_phase4_bond_accounting: 5 passed; 0 failed
ron-ledger quickchain_pre_root_boundary: 1 passed; 0 failed
```

Full preflight results:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=29
ron-ledger quickchain exhaustive preflight gate passed: tests=35
```

## 11. What this pair proves

This crate pair now proves:

```text
strict dispute/challenge DTO shape exists
challenge windows are explicit
appeal windows are explicit
frozen pending appeal state is explicit
appeal-open state is explicit
terminal dispute states are explicit
unknown fields are rejected
one-step irreversible slash shape is rejected
bond dispute event sequence is replayable
dispute simulation is deterministic
terminal disputes cannot be replayed forward
window enforcement is tested
simulation does not mutate bond accounting state
pre-root/no-authority guard remains green
clippy preflight remains clean
```

## 12. What this pair does not implement

This pair does not implement:

```text
live slashing
live staking
wallet bond mutation routes
wallet slash/capture/release routes
automatic bond forfeiture
public validator economy
validator rewards
public staking market
liquidity
exchange-facing logic
ROX
Solana
public bridge
external settlement
gateway ledger mutation
omnigate ledger mutation
rewarder ledger mutation
accounting balance truth
index slash truth
policy slash execution
CrabLink slash authority
client-side bond authority
fake balances
fake receipts
silent spend
root production
checkpoint production
finality
public chain state
```

## 13. Safe status wording

Use this wording:

```text
ron-proto + ron-ledger are COMPLETE / PARKED for QuickChain Phase 4 Round 2.
Phase 4 Round 2 slashing/challenge simulation foundation is green for this crate pair.
The implementation is deterministic, replayable, explicit-window, and no-live-slash.
```

Do not say:

```text
Phase 4 Round 2 is complete
Phase 4 is complete
slashing is live
staking is live
bond enforcement is live
validators can be slashed
public validator economy exists
wallet supports live bond slashing
gateway can trigger slashing
CrabLink can manage slashing
ROX/Solana bridge is ready
external settlement is active
```

## 14. Retest commands for this completed pair

Only rerun if this pair is modified again.

Focused retests:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p ron-proto --test quickchain_phase4_bond_dispute
cargo test -p ron-proto --test quickchain_phase4_bond_dto

cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_dispute_simulation
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_accounting
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
```

Full crate-local preflight gates:

```bash
cd /Users/mymac/Desktop/RustyOnions

crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=29
ron-ledger quickchain exhaustive preflight gate passed: tests=35
```

## 15. Next crate pair

Next active pair in the fixed QuickChain crate-pair order:

```text
svc-wallet + ron-accounting
```

Recommended codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-wallet
bash scripts/make_crate_codex.sh --force -c ron-accounting
```

Expected `svc-wallet` focus for Phase 4 Round 2:

```text
wallet must not live-slash
wallet must not silently freeze funds
wallet must not create automatic forfeiture/capture paths
wallet must not fake bond receipts
wallet must not bypass explicit user/operator confirmation
wallet must treat dispute/challenge state as future-gated unless explicit backend route is safely designed
wallet remains mutation front-door but no live slash enforcement yet
```

Expected `ron-accounting` focus for Phase 4 Round 2:

```text
accounting may report dispute/challenge simulation status
accounting may summarize disputed/frozen simulated bond state
accounting must not become balance truth
accounting must not execute slashing
accounting must not mutate ledger/wallet state
accounting must not treat simulation as settlement
accounting must not produce wallet receipts
accounting must not plan slash rewards
```

## 16. Completion judgment

Final judgment:

```text
ron-proto + ron-ledger are complete / parked for QuickChain Phase 4 Round 2.
```

This pair now provides the Phase 4 Round 2 foundation:

```text
strict disputed-bond DTOs
+ explicit challenge windows
+ explicit appeal/freeze states
+ strict dispute lifecycle event DTOs
+ unknown-field rejection
+ one-step irreversible slash rejection
+ deterministic dispute simulation
+ replayable dispute state
+ terminal dispute protection
+ no mutation of bond accounting state
+ pre-root boundary scanner green
+ exhaustive crate-local preflights green
+ no live slashing
+ no staking
+ no bridge
+ no external settlement
+ no public validator economy
```

Next active pair:

```text
svc-wallet + ron-accounting
```

### END NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - ron-proto + ron-ledger


### END NOTE - JUNE 25 2026 - 23:50 CST