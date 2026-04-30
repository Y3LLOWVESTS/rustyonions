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