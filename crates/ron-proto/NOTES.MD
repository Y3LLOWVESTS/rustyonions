

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