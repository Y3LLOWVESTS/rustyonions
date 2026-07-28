Yep—here’s a tight carryover pack from **svc-dht → ron-naming** so we don’t re-learn anything the hard way.

### BEGIN NOTE - OCTOBER 28 2025 - 14:52 CST

# Carryover notes for `ron-naming` (from `svc-dht`)

## 1) Interfaces & types to reuse

* **Content IDs:** Use the same `B3Cid` (from `ron-proto`) everywhere. Don’t re-introduce a parallel CID type in ron-naming. Parsing/validation rules should be identical.
* **Node/Provider URIs:** Keep the `local://…` and future `oap://…`/`tcp[s]://…` shape. Reuse the URI validator used by `POST /provide` in `svc-dht`.
* **Readiness/Health:** Mirror the `/healthz` + `/readyz` contract (won’t be “ready” until bootstrap/min-fill or cache warm is satisfied).

## 2) How naming should sit on top of DHT

* **Lookup path:** `name → NameRecord → B3Cid → DHT find_providers`. Naming performs the **name → record** step; DHT supplies **location**.
* **Publish path:** `NameOwner` signs a `NameRecord` (see §4) → naming validates & stores → optionally **provides** the resolved CID into DHT (or lets the app do it).
* **Hedging:** Reuse `race_hedged` policy knobs:

  * Default **β=2**, **stagger≈2–5ms**, **budget≈150–250ms** for user-facing resolves (adjust per SLO).
  * Naming should hedge both **record fetches** (if distributed) and **provider lookups** (via DHT).

## 3) TTLs, caching, and negative caching

* **Respect TTL** on `NameRecord` and on DHT provider entries.
* **Revalidation window:** early-refresh at ~80–90% of TTL to keep hot names hot.
* **Negative cache:** cache NXDOMAIN-equivalent results with a small TTL (e.g., 5–15s) to cut thundering herds.

## 4) Record model (proposed, keep small)

* `NameRecord { name, target: B3Cid, kind, ttl, seq, owner_pk, sig, issued_at, not_after, meta? }`

  * **kind**: e.g., `Content`, `Redirect`, `BundleIndex`, etc.
  * **seq**: monotonic to prevent rollback.
  * **sig**: detached signature over a canonical form; use `ron-kms` keys & algorithms.
  * **size budget**: keep records tiny (≪ 1 MiB OAP frame); aim < 1–2 KiB.
* **Mutability/IPNS-like behavior:** newer `seq` wins; reject stale or future-dated beyond grace.

## 5) Security & anti-abuse

* **Signature verification mandatory** for all mutable records; pin allowed algs to `ron-kms`.
* **Rate-limits:** per source IP/ASN and per `owner_pk` for publish/update.
* **Anti-poison:** on conflict, prefer highest `seq` with valid signature from the **same** `owner_pk`; otherwise treat as a fork and require policy resolution.
* **Diversity on resolve:** use the **ASN diversity helper** from `svc-dht` when choosing provider candidates to avoid single-ASN eclipse.

## 6) Metrics/SLOs to add (mirror DHT style)

* `naming_lookups_total`, `naming_lookup_latency_seconds{class="cache|fresh"}`, `naming_records_total{state="valid|expired|revoked"}`,
  `naming_cache_hits_total`, `naming_cache_hit_ratio`, `naming_publish_total{result="ok|reject|bad_sig"}`,
  `naming_seq_rollback_total`, `naming_sig_verify_seconds`.
* **Correlate with DHT metrics**: propagate a correlation ID so we can join `naming_*` with `dht_lookup_*` in traces.

## 7) Readiness gates (copy the pattern)

* Do **not** report ready until:

  * config loaded, metrics bound, admin bound,
  * **seeded/bootstrapped** (if records are distributed),
  * optional: **warm-cache** for critical names (if configured).

## 8) Failure modes to design for

* **Expired/soon-to-expire records:** serve + kick revalidation or require strict freshness per policy.
* **DHT miss:** return NXDOMAIN-equivalent with negative cache TTL; **don’t** loop indefinitely.
* **Clock skew:** verify timestamps with tolerance; log and downgrade trust if outside bounds.
* **Partial results:** if DHT yields providers but fetch fails, expose a distinct status for observability.

## 9) Testing patterns to port (all green in svc-dht)

* **Handler tests without sockets** (fast CI): mirror `api_smoke.rs` style to validate `/publish_name` + `/resolve/:name`.
* **Property tests** for:

  * sequence/rollback rejection,
  * signature acceptance/rejection,
  * cache TTL correctness and negative cache,
  * diversity floor satisfied when picking providers.
* **Hedger tests**: reuse `deadline_hedge.rs` pattern to prove budget/stagger are enforced.
* **E2E smoke scripts** mirroring `run-local.sh` and `smoke_svc_dht.sh`.

## 10) Performance expectations (baseline guidance)

* **In-process resolve (cache hit):** sub-millisecond target on laptop; aim well under **200–300µs** on servers.
* **Fresh resolve with β=2 (no network DHT):** expect low-single-digit **µs** for the hedger + selection layer (bench proved ~3–9 µs per β step).
* **End-to-end (with DHT over loopback/transport):** dominated by transport; hedge to pull P95/P99 down.

## 11) Config surface (keep familiar)

* `beta`, `hedge_stagger_ms`, `hedge_budget_ms`, `asn_floor`, `min_fill` (bootstrap), `cache_ttl_bounds`, `negative_ttl`, `max_record_bytes`, `kms_keyset`, `publish_rate_limits`.

## 12) What to **not** redo

* **Don’t fork** the CID type or distance math. Import from `ron-proto` / reuse `NodeId` distance code when you need XOR ordering.
* **Don’t re-register Prometheus collectors** per test—share a registry (we already hit this once).
* **Don’t use non-Send RNG** in any async/hedged benchmarks (use deterministic mixers like we did).

## 13) “Later” parking lot for ron-naming

* **Persistent index** for records (sled/rocks); background compaction & revalidation queue.
* **Gossip/backfill** for hot names.
* **Revocation lists** (owner rotates keys; signed revocation).
* **Wildcard / subname delegation** with policy boundaries.
* **Client SDK** helpers (`resolve()`, `publish()`, `watch()`).

---

### Minimal cross-crate checklist

* [ ] Reuse `B3Cid` and URI parsing.
* [ ] Port hedger defaults & metrics conventions.
* [ ] Implement signed `NameRecord` with `seq` and TTL; verify via `ron-kms`.
* [ ] Add `/publish_name`, `/resolve/:name`, `/metrics`, `/readyz`, `/healthz`.
* [ ] Copy the CI-fast handler tests approach; add diversity/TTL/rollback properties.
* [ ] Ship a `smoke_ron_naming.sh` mirroring the DHT smoke (publish→resolve→metrics).

### END NOTE - OCTOBER 28 2025 - 14:52 CST

### BEGIN NOTE - OCTOBER 28 2025 - 16:15 CST

# NOTES — `ron-naming`

**Profile:** RON-CORE (schema & hygiene only; no runtime lookups)
**Status:** ~92% complete (Beta-ready)
**Last updated:** 2025-10-28 (America/Chicago)

---

## 1) What this crate is

`ron-naming` defines **types and hygiene** for names and addresses in RON-CORE:

* **DTOs/Types:** `Fqdn`, `ContentId`, `NameRecord`, `Address` (`Content{b3:…}` | `Name{fqdn, version?}`), `NameVersion`.
* **Normalization:** Unicode → ASCII via UTS-46/IDNA + NFC; lowercase; no trailing dot.
* **Wire helpers:** JSON/CBOR encode/decode + round-trip functions.
* **CLI (feature-gated):** `tldctl` for normalize/parse/encode dev-workflows.

> No network or persistence here — resolution/lookup happens in downstream services (e.g., `svc-index`, `svc-dht`).

---

## 2) What we accomplished

### 2.1 Types & Invariants

* `ContentId` strictly enforces **`"b3:" + 64 lowercase hex`**.
* `Fqdn` holds normalized ASCII domain names.
* `NameRecord { name: Fqdn, version: Option<NameVersion>, content: ContentId }`.
* `Address`:

  * `Content { id: ContentId }`
  * `Name { fqdn: Fqdn, version: Option<NameVersion> }`
* `NameVersion` wraps `semver::Version` with parse/serde.

### 2.2 Normalization

* `normalize_fqdn_ascii(&str) -> NormalizedFqdn(Fqdn)`:

  * Applies UTS-46/IDNA (punycode) + NFC normalization.
  * Enforces lowercase, no trailing dot.
  * Rejects invalid input up front.

### 2.3 Wire helpers

* `wire::json::{to_json_bytes, from_json_bytes, roundtrip_*_json}`
* `wire::cbor::{to_cbor_bytes, from_cbor_bytes, roundtrip_*_cbor}`

### 2.4 CLI (feature `cli`)

* `tldctl` subcommands:

  * `normalize <name>` → ASCII FQDN (punycoded).
  * `parse <addr>` → JSON `Address` DTO (`b3:…` or `name[@semver]`).
  * `json --version <semver> --content <b3:…> <name>` → JSON `NameRecord`.
  * `cbor --version <semver> --content <b3:…> <name>` → CBOR (base64 to stdout).

### 2.5 Tests & benches

* **Integration tests**:

  * `tests/address_hygiene.rs` (3 passing).
  * `tests/normalize_idempotence.rs` (2 passing).
  * `tests/dto_wire_vectors.rs` (placeholder passing).
  * `tests/cli_contract.rs` (2 passing; requires `--features cli`).
* **Criterion benches**:

  * `benches/normalize_bench.rs`
  * `benches/encode_bench.rs`

---

## 3) How to run things

### 3.1 Format & lint

```
cargo fmt -p ron-naming
cargo clippy -p ron-naming --no-deps -- -D warnings
```

### 3.2 Unit & integration tests (library)

```
cargo test -p ron-naming
```

Notes:

* You’ll see “0 tests” for `src/lib.rs` unless unit tests are added inside the lib (integration tests still run and pass).

### 3.3 CLI contract tests

The CLI is feature-gated and the tests build/run the `tldctl` binary with `assert_cmd`:

```
cargo test -p ron-naming --features cli --test cli_contract
```

To run other tests alongside CLI tests:

```
cargo test -p ron-naming --features cli
```

### 3.4 CLI examples (manual)

```
cargo run -p ron-naming --example normalize_roundtrip -- "Café.Example"
cargo run -p ron-naming --features cli -- normalize "Café.Example"
cargo run -p ron-naming --features cli -- parse "files.example@1.2.3"
```

### 3.5 Benches (Criterion)

Run all benches:

```
cargo bench -p ron-naming
```

**Save a baseline** (important): pass the baseline flag to each **bench binary**, not the lib test harness. Use `--bench` selectors:

```
cargo bench -p ron-naming --bench normalize_bench -- --save-baseline naming-2025-10-28
cargo bench -p ron-naming --bench encode_bench    -- --save-baseline naming-2025-10-28
```

Compare against a saved baseline:

```
cargo bench -p ron-naming --bench normalize_bench -- --baseline naming-2025-10-28
cargo bench -p ron-naming --bench encode_bench    -- --baseline naming-2025-10-28
```

> **Why your earlier “Unrecognized option: 'save-baseline'” happened:**
> You passed `-- --save-baseline` without scoping to a specific bench binary. Cargo also runs the lib test harness (0 tests) under the bench profile, and *that* harness doesn’t understand Criterion’s flags. The fix is to scope with `--bench <name>` as above.

---

## 4) Current benchmark snapshot (your machine)

* JSON:

  * `json/address_roundtrip`: ~**0.946 µs**
  * `json/record_roundtrip`: ~**1.086 µs**
* CBOR:

  * `cbor/address_roundtrip`: ~**1.124 µs**
  * `cbor/record_roundtrip`: ~**1.366 µs**
* Normalize:

  * `normalize_fqdn_ascii/mixed`: ~**23.7 µs** per batch-iteration
  * `normalize_fqdn_ascii/hot_ascii`: ~**1.41 µs**

Interpretation:

* Encode/decode is sub-2 µs → excellent for control-plane DTO usage.
* Unicode normalization is intentionally more expensive than ASCII fast-path; this is a one-time hygiene step at the edge, not per-packet.

---

## 5) Contracts for downstream crates (carry-over)

* **svc-index** (name → content resolution)

  * **Require normalized** ASCII FQDN at the API boundary (lowercase; no trailing dot). Reject raw Unicode input with 400 and a hint to normalize.
  * Accept `Address::Name{ fqdn, version? }`. Return `ContentId` (strict `"b3:<64hex>"` lowercase).
* **svc-dht** (provider discovery)

  * Operate **only** on `ContentId` (`"b3:<64hex>"`). Keep the exact shape check.
* **svc-gateway/omnigate**

  * If exposing public “resolve” endpoints, **normalize first** using `ron-naming` before calling `svc-index`.
* **ron-proto**

  * Treat `ron-naming` DTOs as schema truth. Add round-trip tests through `wire::{json,cbor}`.

---

## 6) Troubleshooting

* **Doc lint failures** (`#![deny(missing_docs)]`):
  Ensure public modules/functions/enum variants have rustdoc comments.
* **CLI unresolved imports** (`clap`, `anyhow`, `base64`):
  The `tldctl` bin is feature-gated. Build or test it with `--features cli`. Cargo.toml includes:

  ```
  [[bin]]
  name = "tldctl"
  path = "src/bin/tldctl.rs"
  required-features = ["cli"]
  ```
* **base64 `.encode` method not found**:
  Import the trait `use base64::Engine;` (already present in `tldctl.rs`).
* **CLI test can’t find binary (`CARGO_BIN_EXE_tldctl` not set)**:
  We use `assert_cmd::Command::cargo_bin("tldctl")` — it builds and locates the bin automatically (fixed).
* **Criterion `--save-baseline` not recognized**:
  Scope to a bench binary with `--bench <name>` (see §3.5).

---

## 7) What’s left to “Gold” (~8%)

1. **Docs polish**

   * **README.md**: add Quickstart, CLI examples, and Benchmarks table (with your numbers).
   * **docs/TESTS.md**: list commands for unit/integration/CLI tests (with `--features cli` note).
   * **docs/PERFORMANCE.md**: describe benches, how to save/compare baselines, known performance characteristics.

2. **Vector attestation (optional, nice to have)**

   * Add `scripts/hash_vectors.sh` to BLAKE3-hash `testdata/vectors/*` and write to `testdata/signatures/vectors.b3.txt`.
   * Add a `verify` feature test that loads vectors, verifies hashes, and round-trips selected DTOs.

3. **Crate hygiene**

   * Confirm `deny.toml` matches workspace baseline (licenses/bans/advisories).
   * Ensure `CODEOWNERS`, dual licenses, and README badges finalized.
   * (Optional) `#[doc = include_str!("../README.md")]` at the top of `lib.rs` so rustdoc mirrors README.

4. **Forward-compat**

   * Keep `idna = "0.5"` as default; consider an experimental `idna_v1` feature for CI A/B later (off by default).

---

## 8) Suggested finalization workflow

```
# polish docs (README + docs/TESTS.md + docs/PERFORMANCE.md)
# optional: add verify vectors & script

cargo fmt -p ron-naming
cargo clippy -p ron-naming --no-deps -- -D warnings
cargo test -p ron-naming
cargo test -p ron-naming --features cli --test cli_contract
cargo bench -p ron-naming --bench normalize_bench -- --save-baseline naming-2025-10-28
cargo bench -p ron-naming --bench encode_bench    -- --save-baseline naming-2025-10-28

git add .
git commit -m "ron-naming: finalize docs; benches baseline; CLI contract tests green"
```

---

## 9) Quick reference (CLI)

```
# Normalize Unicode → ASCII (punycode)
cargo run -p ron-naming --features cli -- normalize "Café.Example"
# => xn--caf-dma.example

# Parse address string → JSON DTO
cargo run -p ron-naming --features cli -- parse "files.example@1.2.3"
# {
#   "kind": "name",
#   "fqdn": "files.example",
#   "version": "1.2.3"
# }
```

---

**Verdict:** `ron-naming` is **Beta-ready**. Finish docs (and optional vector attestation) and we can move on to the next crate with confidence.


### END NOTE - OCTOBER 28 2025 - 16:15 CST


### BEGIN NOTE - APRIL 29 2026 - 13:00 CST

Here are the **ron-naming changelog/carry-over notes only**.

---

# `ron-naming` — WEB3_2 Crab Asset URL Foundation Notes

**Session date:** 2026-04-29
**Crate:** `crates/ron-naming`
**Status:** Batch complete / green
**Purpose of this session:** Add the canonical public `crab://` asset URL grammar for WEB3_2 product-proof asset pages.

---

## 0. Summary

In this session, `ron-naming` was extended to support the new canonical public `crab://` asset URL format:

```text
crab://<64-lowercase-hex>.<asset_kind>
```

Example:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Important project decision confirmed during the session:

```text
Public crab asset URLs must NOT include a b3/ path prefix.
```

So this old shape is now intentionally noncanonical:

```text
crab://b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

The internal content ID remains canonical:

```text
b3:<64-lowercase-hex>
```

This keeps user-facing links clean while preserving RustyOnions’ internal BLAKE3 content-addressing invariant.

---

## 1. Files added or updated

### Updated

```text
crates/ron-naming/src/lib.rs
crates/ron-naming/src/crab.rs
crates/ron-naming/tests/dto_wire_vectors.rs
```

### Added

```text
crates/ron-naming/src/asset.rs
crates/ron-naming/tests/asset_kind.rs
crates/ron-naming/tests/crab_links.rs
```

---

## 2. Major functionality added

### 2.1 Added `AssetKind`

A new `AssetKind` enum was added to `ron-naming` to support typed asset suffix parsing.

Current supported kinds in `ron-naming`:

```text
.image
.song
.article
.video
.comment
.page
.manifest
```

Later discussion clarified that the broader project should support all typed asset suffixes moving forward, such as:

```text
.image
.video
.music
.article
.post
.comment
```

But we did **not** force-renormalize `.song` to `.music` in this session because the user clarified those were examples, not a required immediate change.

### 2.2 Added pure `crab://` parser

A new `crab` module was added with:

```rust
CrabLink
CrabRoute
CrabNamespace
CrabParseError
CRAB_SCHEME
```

The parser supports:

```text
crab://<64hex>.image
crab://<64hex>.song
crab://<64hex>.article
crab://<64hex>.video
crab://<64hex>.comment
crab://<64hex>.page
crab://<64hex>.manifest
```

The parser also supports named routes:

```text
crab://site/<name>
crab://name/<name>
crab://<name>
```

These are kept for later `crab://site` and named-site work.

### 2.3 Canonical string rendering

The parser now canonicalizes asset URLs into the clean public shape:

```text
crab://<64-lowercase-hex>.<asset_kind>
```

If the input hash uses uppercase hex, the canonical output lowercases it.

Example:

```text
Input:
crab://0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF.IMAGE

Canonical:
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

The internal CID returned by the parser remains:

```text
b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

### 2.4 Old `b3/` path prefix rejected

The old draft shape is intentionally rejected:

```text
crab://b3/<hash>.image
```

This prevents two public URL forms from competing.

Canonical public URL:

```text
crab://<hash>.image
```

Internal CID:

```text
b3:<hash>
```

### 2.5 Safe query parsing

The `CrabLink` parser supports safe query parameters and canonicalizes query key ordering.

Example:

```text
crab://<hash>.image?z=2&a=1&receipt=tx:abc
```

Canonical output:

```text
crab://<hash>.image?a=1&receipt=tx:abc&z=2
```

Duplicate query keys are rejected.

Unsafe query characters and bad percent encoding are rejected.

### 2.6 Security/hygiene checks

The parser rejects:

```text
control characters
embedded credentials / userinfo-like syntax
path traversal
encoded traversal patterns
unknown asset kinds
bad hash length
bad hash characters
missing asset kind suffix
unsupported namespaces with paths
duplicate query keys
fragments
```

This keeps `ron-naming` fail-closed and safe for later browser-extension and gateway usage.

---

## 3. DTO / wire-vector cleanup

The old placeholder test in:

```text
crates/ron-naming/tests/dto_wire_vectors.rs
```

contained:

```rust
assert!(true);
```

This failed under strict clippy:

```text
-D warnings
clippy::assertions_on_constants
```

That placeholder was replaced with real JSON/CBOR round-trip tests covering:

```text
Address JSON round-trip for b3 content IDs
Address JSON round-trip for versioned names
Address CBOR round-trip for b3 content IDs
NameRecord JSON round-trip
NameRecord CBOR round-trip
JSON byte validity and content-address preservation
```

This moved `dto_wire_vectors.rs` from placeholder status to meaningful coverage.

---

## 4. Tests added

### `tests/asset_kind.rs`

Covers:

```text
stable asset kind suffixes
case-insensitive parsing
empty suffix rejection
unknown suffix rejection
```

### `tests/crab_links.rs`

Covers:

```text
canonical typed b3 asset links without b3/ prefix
uppercase hash/kind canonicalization
site links
explicit name links
root name links
query parameter canonicalization
percent encoding normalization
invalid hash rejection
unknown asset kind rejection
missing asset kind rejection
embedded credential rejection
path traversal rejection
encoded traversal rejection
control character rejection
unsupported namespace rejection
old crab://b3/<hash>.image rejection
duplicate query key rejection
stable parser error codes
```

### `tests/dto_wire_vectors.rs`

Covers:

```text
Address JSON round-trip
Address CBOR round-trip
NameRecord JSON round-trip
NameRecord CBOR round-trip
valid JSON bytes
b3 content address preservation
```

---

## 5. Validation gates passed

The final `ron-naming` gates passed:

```text
cargo fmt
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

Final observed result:

```text
ron-naming clippy: clean
ron-naming tests: all passing
```

Specific passing test groups included:

```text
address_hygiene.rs
asset_kind.rs
crab_links.rs
dto_wire_vectors.rs
normalize_idempotence.rs
encode_bench.rs
normalize_bench.rs
examples/encode_decode.rs
examples/normalize_roundtrip.rs
```

---

## 6. Invariants preserved

This work preserved the WEB3 / WEB3_2 invariants:

```text
ron-naming remains pure schema/parsing/normalization logic.
No IO was added.
No async was added.
No storage calls were added.
No wallet calls were added.
No ledger calls were added.
No economics logic was added.
No raw asset bytes are stored here.
Internal content IDs remain canonical b3:<64 lowercase hex>.
Public crab asset URLs are canonical crab://<64hex>.<asset_kind>.
Names remain optional human pointers.
Hashes remain canonical content addresses.
```

---

## 7. Architectural boundary notes

`ron-naming` now owns:

```text
crab:// parsing
asset suffix parsing
canonical crab string formatting
b3 hash normalization for public crab links
named-site/name route syntax checks
safe query normalization
deterministic parse errors
```

`ron-naming` still does **not** own:

```text
storage
asset bytes
manifest storage
manifest hydration
wallet mutation
ledger mutation
paid access
receipt verification
browser extension logic
gateway routing
index persistence
```

Those responsibilities remain in their proper crates:

```text
svc-storage    — raw content bytes by b3 CID
svc-index      — manifest pointers / name lookups
omnigate       — hydrated views / asset-page composition
svc-gateway    — ingress proxying
svc-wallet     — wallet mutation / receipts
ron-ledger     — durable economic truth
```

---

## 8. Completion assessment for `ron-naming`

For the WEB3_2 product-proof TODO, `ron-naming` Batch 1 is:

```text
100% complete for this session’s scope
```

Current `ron-naming` WEB3_2 readiness:

```text
Canonical crab asset URL grammar         complete
Typed asset suffix parsing               complete for initial beta set
Old b3/ public prefix rejection           complete
Internal b3 CID preservation              complete
Safe parsing / deterministic errors       complete
Wire-vector placeholder cleanup           complete
Targeted gates                            green
```

---

## 9. Remaining future work for `ron-naming`

No blocker remains from this session.

Future enhancements may include:

```text
Add broader asset suffix vocabulary if/when product needs it:
  .music
  .post
  .app
  .site

Potentially align ron-naming AssetKind exactly with ron-proto AssetKind later.

Add explicit parser support for future crab://site flows if the grammar expands.

Add route helpers for browser-extension translation:
  crab://<hash>.image
  → /v1/b3/<hash>.image

Add stricter public docs/examples once the route contract is finalized.
```

Important note: after the user clarified examples, we did **not** force `.song` → `.music` in `ron-naming`. That can be handled later if the product vocabulary is standardized across crates.

---

## 10. Session takeaway

`ron-naming` now provides the clean public URL foundation for WEB3_2 typed asset pages:

```text
crab://<64hex>.<asset_kind>
```

This is the first critical product-proof primitive for RustyOnions’ creator-facing asset system. It gives the rest of the stack a stable, user-friendly address format while preserving internal `b3:<hash>` canonical identity.


### END NOTE - APRIL 29 2026 - 13:00 CST




### BEGIN NOTE - MAY 6 2026 - 17:50 CST


## Set 2 — `ron-naming` Crate Notes

Drop this into the `ron-naming` crate notes.

---

# `ron-naming` Crate Carryover Notes

## NEXT_LEVEL / WEB3_2 Username + Crab Link Parser Foundation

## 1. Crate role

`ron-naming` is the pure naming/addressing crate for RustyOnions. It should stay DTO/parser/normalization-only.

Its job is to define and validate canonical naming grammar for:

```text
b3:<64 lowercase hex>
crab://<64 lowercase hex>.<asset_kind>
crab://site/<name>
crab://name/<name>
crab://<root-name>
@username
crab://@username
crab://profile/@username
```

It must **not** do runtime lookup, storage, network calls, wallet logic, ledger mutation, or profile persistence.

Correct layering:

```text
ron-naming:
- pure parsing
- pure normalization
- pure validation
- deterministic errors

svc-index / omnigate / svc-passport:
- runtime lookup
- persistence
- profile resolution
- site/profile hydration
```

Important invariant:

```text
Names and usernames are human pointers.
b3 hashes remain canonical content identity.
```

---

## 2. Session summary

In this session, `ron-naming` was upgraded to support the NEXT_LEVEL identity/profile grammar and expanded asset suffix vocabulary.

Main additions:

```text
RonUsername parser
@username normalization
crab://@username parsing
crab://profile/@username parsing
CrabNamespace::Profile
CrabRoute::Profile
expanded AssetKind suffixes
new username tests
new asset kind tests
expanded crab link tests
```

This crate now provides the canonical parser foundation needed by the later backend chain:

```text
ron-naming
→ ron-proto
→ svc-passport
→ omnigate
→ svc-gateway
→ CrabLink
```

---

## 3. Files changed / added

Primary files touched:

```text
crates/ron-naming/src/lib.rs
crates/ron-naming/src/username.rs
crates/ron-naming/src/asset.rs
crates/ron-naming/src/crab.rs
crates/ron-naming/tests/username.rs
crates/ron-naming/tests/asset_kind.rs
crates/ron-naming/tests/crab_links.rs
```

---

## 4. Public API additions

### `RonUsername`

Added canonical public username type:

```rust
RonUsername
```

Purpose:

```text
Represents a normalized RON username without the leading @.
```

Examples:

```text
@Skinny.Crabby  → skinny.crabby
Skinny-Crabby   → skinny-crabby
skinny_crabby   → skinny_crabby
```

Important methods:

```rust
RonUsername::parse(input)
RonUsername::as_str()
RonUsername::handle()
RonUsername::crab_url()
RonUsername::into_string()
```

Expected outputs:

```text
username.as_str()  → skinnycrabby
username.handle()  → @skinnycrabby
username.crab_url() → crab://@skinnycrabby
```

### Helper functions

Added:

```rust
normalize_username(input: &str) -> Result<String, UsernameParseError>
normalize_handle(input: &str) -> Result<String, UsernameParseError>
```

Usage:

```text
normalize_username("@SkinnyCrabby") → "skinnycrabby"
normalize_handle("SkinnyCrabby")    → "@skinnycrabby"
```

---

## 5. Username validation rules

The normalized username:

```text
must be lowercase ASCII after normalization
must be 3–32 bytes
may contain:
  a-z
  0-9
  _
  -
  .
must start with ASCII letter or digit
must not end with '.', '-', or '_'
must not contain consecutive dots
must not contain spaces
must not contain Unicode
must not contain reserved system names
```

Reserved examples:

```text
admin
alt
api
app
article
asset
assets
b3
comment
crab
gateway
image
mail
manifest
mod
moderator
music
passport
podcast
post
profile
root
ron
site
sites
stream
support
sys
system
video
wallet
```

Important product rule:

```text
Reserved names are rejected in ron-naming before svc-passport claim persistence.
```

This prevents user-facing usernames from colliding with protocol/product namespaces.

---

## 6. Username error taxonomy

Added deterministic error enum:

```rust
UsernameParseError
```

Known variants:

```text
Empty
TooShort
TooLong
InvalidStart
InvalidTrailingPunctuation
InvalidCharacter
ConsecutiveDots
Reserved
```

Each has a stable code via:

```rust
UsernameParseError::code()
```

Example codes:

```text
empty
too_short
too_long
invalid_start
invalid_trailing_punctuation
invalid_character
consecutive_dots
reserved
```

Important test lesson from this session:

When matching errors with owned `String` fields, borrow the error before calling `.code()`:

```rust
assert_eq!(err.code(), "reserved");
assert!(matches!(
    &err,
    UsernameParseError::Reserved { name } if name == "site"
));
```

Do **not** move the string field out of the error and then call `err.code()` afterward.

---

## 7. Crab link parser additions

`crab.rs` now supports public profile routes.

New namespace:

```rust
CrabNamespace::Profile
```

New route:

```rust
CrabRoute::Profile { username: RonUsername }
```

Accepted forms:

```text
crab://@skinnycrabby
crab://@SkinnyCrabby
crab://profile/@skinnycrabby
crab://profile/@SkinnyCrabby
```

Canonical output:

```text
crab://@skinnycrabby
```

Important behavior:

```text
crab://profile/@SkinnyCrabby parses successfully.
canonical_string() emits crab://@skinnycrabby.
```

That means `crab://profile/@...` is accepted as an input form, but canonical display should collapse to:

```text
crab://@username
```

---

## 8. Typed asset kind expansion

`AssetKind` was expanded to cover the NEXT_LEVEL / WEB3_2 public asset vocabulary.

Current supported suffixes include:

```text
.image
.song
.music
.podcast
.article
.post
.comment
.video
.stream
.profile
.passport
.alt
.page
.site
.app
.manifest
```

The public typed crab URL format remains:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Examples:

```text
crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
crab://1111111111111111111111111111111111111111111111111111111111111111.profile
crab://2222222222222222222222222222222222222222222222222222222222222222.article
```

Internal canonical content ID remains:

```text
b3:<64 lowercase hex>
```

Important invariant:

```text
The old public form crab://b3/<hash>.image remains noncanonical and should be rejected.
```

---

## 9. Crab link security / hygiene behavior

The parser rejects:

```text
wrong scheme
empty target
unsafe control characters
fragment identifiers
embedded credentials
path traversal
encoded path traversal in target
unsupported namespaces with paths
old b3 slash prefix
bare hash with no asset kind
unknown asset kind
invalid b3 hash length
invalid hash characters
bad profile usernames
duplicate query keys
```

Query behavior:

```text
safe query parameters are preserved
query parameters are sorted by key in canonical output
percent-encoding is normalized to uppercase hex
raw '/' and '\' are rejected in query components
percent-encoded slash can remain encoded and canonicalized
```

Important bug fixed during this session:

The query sanitizer originally rejected percent-encoded `/`, but the existing test expected:

```text
hello%2fworld → hello%2Fworld
```

Final behavior:

```text
raw slash rejected
encoded slash preserved as encoded uppercase %2F
```

---

## 10. Tests added / updated

### `tests/username.rs`

Covers:

```text
username normalizes handle and username
reserved names reject
confusing shapes reject
```

Important cases:

```text
@Skinny.Crabby → skinny.crabby / @skinny.crabby
@site rejects as reserved
ab rejects as too short
-alice rejects invalid start
alice_ rejects trailing punctuation
alice..bob rejects consecutive dots
alice bob rejects invalid character
```

### `tests/asset_kind.rs`

Covers:

```text
all suffixes are stable
case-insensitive parsing
empty suffix rejects
unknown suffix rejects
```

### `tests/crab_links.rs`

Expanded to cover:

```text
parses_profile_username_links
parses_profile_path_username_links
rejects_bad_profile_usernames
```

Also confirms existing behavior:

```text
typed b3 asset links without b3 path prefix
lowercase canonical hash output
safe query sorting
percent-encoding normalization
old b3 slash prefix rejection
missing asset kind rejection
unsupported namespace rejection
```

---

## 11. Green proof from this session

`ron-naming` passed:

```bash
cargo fmt -p ron-naming
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

Important green results:

```text
asset_kind.rs: passed
username.rs: passed
crab_links.rs: 22 passed
dto_wire_vectors.rs: passed
normalize_idempotence.rs: passed
benches/examples smoke: passed
```

`crab_links.rs` reached:

```text
22 passed
0 failed
```

---

## 12. Current role in the NEXT_LEVEL chain

`ron-naming` is now ready for the next layers.

It provides the grammar for:

```text
svc-passport username claim validation
omnigate crab://@username route parsing
svc-gateway profile proxy path validation
CrabLink address bar support for crab://@username
CrabLink profile page rendering
site creator profile links
```

The current backend chain now has profile routes through gateway, but `ron-naming` still needs to be used in future resolver work for:

```text
GET /crab/resolve?url=crab://@skinnycrabby
```

That is not fully implemented yet.

---

## 13. Remaining work for `ron-naming`

### A. Use `CrabLink::parse` in Omnigate profile resolver

Future Omnigate crab resolver should parse:

```text
crab://@username
crab://profile/@username
```

using `ron-naming`, then call profile lookup through the proper service route.

Target route:

```text
GET /crab/resolve?url=crab://@skinnycrabby
```

Expected resolution chain:

```text
svc-gateway /crab/resolve?url=crab://@skinnycrabby
→ omnigate /v1/crab/resolve?url=crab://@skinnycrabby
→ ron-naming parses CrabRoute::Profile
→ omnigate calls svc-passport profile lookup
→ returns public profile page/DTO
```

### B. Avoid duplicating username logic long-term

Right now username normalization logic exists in:

```text
ron-naming
svc-passport profile core
omnigate identity bootstrap helper
```

This is acceptable during staged implementation, but long-term the project should converge on `ron-naming` as the canonical parser.

Future cleanup:

```text
Make svc-passport use ron-naming::RonUsername if dependency boundaries allow.
Make omnigate identity bootstrap username parser follow ron-naming rules exactly.
Avoid drift between reserved username lists.
```

Do this carefully to avoid dependency cycles or broad workspace churn.

### C. Add test vectors for profile crab links

Potential future tests:

```text
crab://@SkinnyCrabby?view=public canonicalizes query
crab://profile/@SkinnyCrabby?tab=assets canonicalizes
crab://@site rejects reserved
crab://profile/site probably should reject because missing @? decide explicitly
```

### D. Add route kind for future feed/comment objects if needed

Asset suffixes exist for:

```text
.post
.comment
.article
.video
.stream
.podcast
```

But actual runtime object semantics are not in `ron-naming`.

If future grammar requires specific name routes like:

```text
crab://post/<id>
crab://comment/<id>
```

that should be designed separately and not conflated with typed b3 asset URLs.

### E. Document canonical public profile forms

Add crate docs explaining:

```text
crab://@username is canonical public profile link
crab://profile/@username is accepted input compatibility form
canonical_string emits crab://@username
```

---

## 14. Important invariants to preserve

Do not add any of the following to `ron-naming`:

```text
network calls
HTTP clients
database access
sled usage
wallet calls
ledger calls
storage calls
index mutation
profile persistence
username claiming
signature verification
policy gates
async runtime requirements
locks
```

`ron-naming` should remain:

```text
pure
deterministic
fast
strict
serde-friendly
test-vector-friendly
```

Every parser should fail closed.

---

## 15. Known good examples

### Username

```text
Input:  @SkinnyCrabby
Output username: skinnycrabby
Output handle:   @skinnycrabby
Output crab URL: crab://@skinnycrabby
```

### Profile link

```text
Input:  crab://profile/@SkinnyCrabby
Route:  CrabRoute::Profile
Output: crab://@skinnycrabby
```

### Typed image asset

```text
Input:
crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image

Internal CID:
b3:984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4

Kind:
image
```

### Rejected old form

```text
crab://b3/984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

Reason:

```text
Old b3 slash prefix is noncanonical.
```

---

## 16. Suggested next `ron-naming` acceptance gate

Before any future changes to this crate, run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-naming
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

If future work changes the public grammar, also rerun downstream tests:

```bash
cargo test -p omnigate --test passport_profile_routes
cargo test -p svc-gateway --test identity_routes_proxy
```

---

## 17. Status at end of this session

Current `ron-naming` status:

```text
GREEN
```

Feature status:

```text
@username parser: complete for current phase
crab://@username parser: complete for current phase
crab://profile/@username parser: complete for current phase
expanded asset kind suffixes: complete for current phase
runtime profile resolution: not in this crate / pending Omnigate resolver work
username persistence/claiming: not in this crate / implemented in svc-passport current in-memory phase
```

Next relevant project step involving `ron-naming`:

```text
Use CrabLink/Omnigate resolver path to make crab://@username open a public profile page.
```


### END NOTE - MAY 6 2026 - 17:50 CST




### BEGIN NOTE - JULY 22 2026 - 22:25 CST


Last changed crate: **`crates/ron-naming`**. This crate was touched in Phase 2B/2C to become the canonical native Passport `@username` handle policy source, so `svc-passport` would not duplicate naming rules. The overall session ended with Phase 7A green. 

# Changelog Notes — `crates/ron-naming`

## Session Scope

This was the first Passport Native Buildplan implementation session involving `ron-naming`.

`ron-naming` was changed during **Phase 2B** and **Phase 2C**. Its role in the Passport Native Buildplan is to own canonical `@username` / handle naming rules so that `svc-passport` does not invent or maintain duplicate handle validation logic.

Current status for this crate:

```text id="lgv2cm"
PHASE_2B_RON_NAMING_USERNAME_HANDLES=GREEN
PHASE_2C_PROFILE_USERNAME_RULE_RECONCILIATION=GREEN
RON_NAMING_NATIVE_PASSPORT_HANDLES=ADDED
SVC_PASSPORT_NATIVE_HANDLE_REUSE=GREEN
```

The key distinction locked in this crate:

```text id="1ikq5l"
@username is a handle.
@username is not a legal name.
@username is not a recovery authority.
@username is not a wallet authority.
@username must not become a signed mutable identity pointer.
```

---

# Why `ron-naming` Was Changed

Native Passport needs user-facing handles, but those handles must not be treated as real names, legal names, key authority, wallet authority, or recovery authority.

Before this work, `svc-passport` profile username rules existed, but the native Passport build needed a cleaner shared naming source that could be reused by `svc-passport` and future surfaces.

The Phase 2B/2C change made `ron-naming` the canonical owner of native Passport handle syntax and reserved-name behavior.

This prevents:

* duplicate username parsers
* duplicate reserved-name lists
* handle/profile drift
* services treating handles as identity authority
* user-facing `@username` labels becoming recovery or wallet authority
* legal/real-name semantics leaking into Passport identity

Correct layering:

```text id="6k87xb"
ron-naming:
  owns canonical native Passport handle rules

svc-passport:
  consumes ron-naming handle parsing
  applies service/profile behavior around it

ron-proto:
  owns protocol-level Passport/Device/Challenge IDs
```

Incorrect layering:

```text id="67x44h"
svc-passport:
  invents a separate native Passport @username parser

ron-proto:
  owns human-facing username/profile policy

ron-naming:
  treats @username as real-name identity authority
```

---

# Files Changed in `ron-naming`

## Added

```text id="6zmqgf"
crates/ron-naming/src/passport_username.rs
```

This file was added to define native Passport username/handle parsing and reserved-name behavior.

## Changed

```text id="xlncyf"
crates/ron-naming/src/lib.rs
```

This module was updated to expose the new Passport username/handle module.

## Added Tests

```text id="bbwmx9"
crates/ron-naming/tests/native_passport_phase2b_username_handles.rs
```

This test locks the native Passport handle behavior in `ron-naming`.

---

# New Native Passport Handle Surface

Phase 2B added a native Passport username/handle surface.

The exact type names may be checked in the current source, but the intended public surface includes:

```text id="m5w5v1"
PassportUsername
PassportUsernameParseError
```

The important behavior is that `ron-naming` now provides a parser/validator for native Passport handles that `svc-passport` reuses.

During implementation, there was a naming conflict because `ron-naming` already had a `UsernameParseError` from another username module. The new native Passport error type was renamed to avoid collision.

Final direction:

```text id="8d1h6b"
Native Passport handle parser error:
PassportUsernameParseError
```

In `svc-passport`, this may be re-exported or aliased locally as needed, but `ron-naming` owns the canonical native Passport handle parser.

---

# Handle Syntax and Rule Intent

The native Passport handle rules are intentionally stricter than loose display-name rules.

A native Passport handle should be:

* deterministic
* local to handle semantics
* safe for profile and routing surfaces
* separate from real/legal names
* separate from recovery/root identity
* separate from wallet authority
* safe to reuse across `svc-passport` and future CrabLink surfaces

The parser is expected to reject malformed handles and reserved labels.

Expected valid handle shape is approximately:

```text id="wmrkni"
@alice
@creator_one
@crablink
```

Expected invalid handle categories include:

```text id="arvjt0"
empty handle
missing @ when @ is required
collapsed separators
double underscore
leading separator
trailing separator
unsupported punctuation
reserved labels
uppercase if lowercase canonicalization is required
ambiguous legal-name-like authority semantics
```

The exact accepted/rejected syntax should be read from `passport_username.rs` and its tests before editing.

---

# Reserved Name Behavior

Phase 2B/2C added or reconciled reserved native Passport handle labels.

Known reserved labels from the session include:

```text id="p6sgz4"
site
ledger
```

Other reserved labels may already exist in `ron-naming` or adjacent username modules.

The important Phase 2C change was adding/reserving:

```text id="j51qcu"
site
```

and ensuring profile/native reconciliation around reserved names, including `ledger`.

Reserved names should prevent ambiguous authority or namespace confusion.

Examples of why these are reserved:

```text id="jcl770"
@site
```

Could be confused with site namespace authority.

```text id="gv8hga"
@ledger
```

Could be confused with ledger/accounting/wallet authority.

These should not be allowed as native Passport handles.

---

# Relationship to `svc-passport`

`svc-passport` was updated to consume the `ron-naming` native Passport handle parser.

Added in `svc-passport`:

```text id="zypyer"
crates/svc-passport/src/native/username.rs
```

This layer reuses `ron-naming` rather than duplicating native handle rules.

The important integration rule:

```text id="xkzb8w"
svc-passport must call into ron-naming for native Passport handle validation.
svc-passport must not create a separate native Passport handle parser.
```

This allows `ron-naming` to remain the single naming-policy source.

---

# Profile Rule Reconciliation

Phase 2C also reconciled some existing `svc-passport` profile username behavior with the new native Passport handle rules.

Although the profile code lives in `svc-passport`, the naming authority work was rooted in `ron-naming`.

Reconciled behavior included:

* double underscore rejection
* reserved `site` behavior
* reserved `ledger` behavior
* explicit documentation of legacy profile hyphen divergence

Important note:

Legacy profile route behavior was **not fully migrated** in this phase.

Known documented divergence:

```text id="ghdsdx"
Legacy profile may still allow a hyphenated profile username such as crab-link.
Native ron-naming Passport handle rules may reject that until route migration is explicitly scoped.
```

This was intentional. The session avoided breaking existing profile routes while establishing native Passport handle rules for future use.

---

# Important Design Distinction: Handle vs Name

This crate must preserve the distinction between:

```text id="p1mfey"
handle
```

and:

```text id="z81cy7"
real name / legal name / display name
```

Native Passport `@username` is a handle.

It is not:

* a real name
* a legal name
* a government identity
* a wallet authority
* a recovery/root authority
* a signed mutable pointer to a human identity
* a source of Passport finality

This distinction matters because Passport is wallet-like and local-custody-oriented. A mutable `@username` must never become the cryptographic root of identity, recovery, wallet, or capability authority.

---

# What `ron-naming` Did Not Add

The `ron-naming` changes were intentionally narrow.

It did **not** add:

* Passport routes
* profile routes
* profile storage
* real-name storage
* legal-name identity
* recovery-root behavior
* device-key behavior
* KMS behavior
* signature behavior
* proof behavior
* vault behavior
* PIN behavior
* restore behavior
* device enrollment behavior
* capability issuance
* wallet mutation
* ledger mutation
* runtime I/O
* secret custody

`ron-naming` stayed in its lane: naming and handle policy.

---

# Tests Added in `ron-naming`

## `native_passport_phase2b_username_handles.rs`

Purpose:

Lock native Passport handle parsing and reserved-name behavior.

The test should cover:

* valid native Passport handles
* malformed handles
* reserved handles
* handle normalization/canonicalization behavior
* rejection of ambiguous or unsafe labels
* no legal-name authority semantics

Expected Phase 2B result:

```text id="p0g9nu"
RON_NAMING_NATIVE_PASSPORT_USERNAME_HANDLES=GREEN
```

---

# Verification Used During Session

Phase 2B/2C verification included `ron-naming` focused tests and downstream `svc-passport` reuse tests.

Representative commands:

```bash id="89rb9s"
cargo test -p ron-naming --test native_passport_phase2b_username_handles
cargo test -p svc-passport --features native-passport --test native_passport_phase2b_ron_naming_username_reuse
cargo test -p svc-passport --features native-passport --test native_passport_phase2c_profile_username_rule_reconciliation
cargo check -p ron-naming
cargo check -p svc-passport --features native-passport
```

Later workspace checks also compiled `ron-naming` as part of the broader workspace.

The latest full workspace check was green at Phase 6C, after the `ron-naming` changes were already in place.

---

# Phase 2B Repair Notes

Initial Phase 2B failed because the new error type name collided with an existing naming module error.

Failure shape:

```text id="hgbhob"
UsernameParseError is defined multiple times
```

Cause:

`ron-naming` already had a `username::UsernameParseError`.

Repair:

The native Passport handle parser error was renamed to:

```text id="mqfocf"
PassportUsernameParseError
```

This avoided collision and made the native Passport handle error more explicit.

Final result:

```text id="b540cv"
PHASE2B_RON_NAMING_USERNAME_HANDLES=GREEN
```

---

# Phase 2C Notes

Phase 2C reconciled native handle rules with existing `svc-passport` profile username behavior where safe.

`ron-naming` side:

* added/reserved `site`
* maintained canonical native handle policy
* provided the source of truth consumed by `svc-passport`

`svc-passport` side:

* rejected double underscores in profile username rules
* reserved `ledger`
* documented legacy hyphen behavior rather than forcing immediate migration
* tested profile/native naming compatibility

Final result:

```text id="x2a36v"
PHASE2C_PROFILE_USERNAME_RULE_RECONCILIATION=GREEN
SITE_RESERVED_PARITY=GREEN
PROFILE_DOUBLE_UNDERSCORE_REJECTION=GREEN
PROFILE_LEDGER_RESERVED=GREEN
LEGACY_PROFILE_HYPHEN_CONTRACT=DOCUMENTED
PROFILE_ROUTES_MIGRATED=NO
```

---

# Design Invariants for `ron-naming`

Going forward, preserve these invariants:

1. `ron-naming` owns human-facing handle/name syntax policy.
2. Native Passport `@username` is a handle, not a legal name.
3. Native Passport handles must not become root/recovery authority.
4. Native Passport handles must not become wallet authority.
5. Reserved names must protect system namespaces.
6. Reserved names must protect ledger/wallet/accounting semantics.
7. `svc-passport` must reuse `ron-naming` for native handle parsing.
8. `svc-passport` must not duplicate native Passport handle rules.
9. `ron-naming` must not add Passport runtime behavior.
10. `ron-naming` must not add secret storage or wallet/ledger mutation.
11. `ron-naming` should keep syntax deterministic and testable.

---

# Future Considerations for `ron-naming`

Possible future additions should stay naming-related.

Good candidates:

```text id="m95iod"
additional reserved labels
site namespace handle rules
creator namespace handle rules
public profile handle normalization helpers
display-name vs handle distinction helpers
migration helpers for legacy profile route names
```

Bad candidates:

```text id="71cqw0"
restore_passport()
derive_device_key()
unlock_vault()
issue_capability()
sign_proof()
verify_proof()
store_secret()
mutate_ledger()
```

Those belong outside `ron-naming`.

---

# Possible Future Reserved Labels

Do not add these preemptively without design/test need, but likely future reserved labels may include names such as:

```text id="j5du3s"
admin
root
system
passport
device
wallet
ledger
site
profile
creator
api
gateway
omnigate
roc
rox
crab
crablink
```

Each reserved label should be added with tests explaining why it is reserved.

---

# Relationship to Later Phases

## Phase 7B and 7C

No immediate `ron-naming` changes are expected for Phase 7B or 7C unless new-device enrollment introduces user-facing invitation names or handle-bound labels.

Phase 7B should likely stay in `svc-passport`.

## Phase 12 — Private-Beta `@username` Registry

`ron-naming` will probably become important again in Phase 12.

Expected Phase 12 work may include:

* private-beta handle registry policy
* reserved namespace finalization
* public handle validation
* handle/profile route migration
* collision rules
* canonical lowercasing rules
* migration rules for legacy profile usernames
* better separation between display name and handle
* tests proving `@username` is not recovery/wallet/root authority

## Phase 13 — Public Profile and Site Namespace

`ron-naming` may also be touched in Phase 13 for:

* public profile namespace
* site namespace
* creator namespace
* route-safe handle rules
* reserved labels
* profile display metadata separation

---

# Current `ron-naming` Role in the Buildplan

The current division of responsibility is:

```text id="q4rh70"
ron-proto:
  canonical protocol IDs

ron-naming:
  canonical handle/name policy

svc-passport:
  native Passport service DTO/review foundations
```

This division should be preserved.

---

# What To Check Before Editing `ron-naming` Again

Before the next `ron-naming` edit, inspect:

```text id="g5s5ed"
crates/ron-naming/src/passport_username.rs
crates/ron-naming/src/lib.rs
crates/ron-naming/tests/native_passport_phase2b_username_handles.rs
```

Also inspect downstream usage:

```text id="2m6v02"
crates/svc-passport/src/native/username.rs
crates/svc-passport/tests/native_passport_phase2b_ron_naming_username_reuse.rs
crates/svc-passport/tests/native_passport_phase2c_profile_username_rule_reconciliation.rs
```

Then run:

```bash id="698f73"
cargo test -p ron-naming --test native_passport_phase2b_username_handles
cargo test -p svc-passport --features native-passport --test native_passport_phase2b_ron_naming_username_reuse
cargo test -p svc-passport --features native-passport --test native_passport_phase2c_profile_username_rule_reconciliation
cargo check -p ron-naming
cargo check -p svc-passport --features native-passport
```

---

# Summary for `ron-naming`

`ron-naming` was changed narrowly but importantly.

It now owns the native Passport handle policy used by `svc-passport`.

Current status:

```text id="isorr6"
Native Passport handle parser added.
PassportUsernameParseError naming collision repaired.
Reserved handle behavior updated, including site.
svc-passport native username reuse green.
profile username reconciliation green.
No runtime behavior added.
No identity authority added.
No wallet/ledger mutation added.
```

The crate should remain focused on deterministic naming and handle policy. Future Passport phases should only touch `ron-naming` when handle, profile, site namespace, creator namespace, or reserved-name behavior needs to change.



### END NOTE - JULY 22 2026 - 22:25 CST