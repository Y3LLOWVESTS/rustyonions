

---

# 🧪 TESTS.md — ron-proto

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible; loom not required for this lib)*

---

## 0) Purpose

Define the **testing contract** for `ron-proto`:

* Unit, integration, property, fuzz, perf (for serialization/hashing).
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Invocation commands for devs & CI.

This crate is **types-only** (Serde/DAG-CBOR, OAP/1 envelope header, `ContentId`), so tests focus on parsing/encoding determinism, content-address integrity, and schema evolution (Vn→Vn+1) without runtime I/O.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Individual modules/functions: `ContentId` parser/formatter, OAP/1 header encode/decode, `ObjectManifestV2` encode/decode, ErrorEnvelope/Capability schemas.
**Location:** Inline in `src/**` with `#[cfg(test)]`.
**Must cover (invariants):**

* `ContentId`: rejects non-`b3:` prefixes; hex must be 64 lowercase chars; round-trip `Display/FromStr`.
* `ObjectManifestV2`: chunk size ≤ 65536; offsets strictly increasing; digest must match size/chunks in positive-path tests.
* OAP/1 header: `len` bounds; unknown flag bits are zero; correct serialization length for given payload.
* ErrorEnvelope/Capability: `deny_unknown_fields` enforced.

**Run:**

```bash
cargo test -p ron-proto --lib
```

---

### 1.2 Integration Tests (library-level)

**Scope:** Crate surface via `tests/*.rs` using **canonical vectors** in `tests/vectors/`.
**Must include:**

* **Vector parity:** decode → encode → **byte-identical** to on-disk CBOR for each vector (deterministic DAG-CBOR).
* **Cross-version decode:** Vn vectors must decode under Vn+1 DTOs where additive; negative vectors must fail with typed errors.
* **Hash truth:** recompute BLAKE3 over payloads referenced by vectors; `ContentId` must match.
* **Interop envelopes:** OAP/1 HELLO echo vector: REQ header ↔ RESP header correlation id preserved.

> Note: Config reload, live concurrency/backpressure are **N/A** for a lib.

**Run:**

```bash
cargo test -p ron-proto --test '*'
```

---

### 1.3 Property-Based Tests

**Scope:** Parsers/codecs where input space is large (manifest, OAP/1 header, capability caveats).
**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants:**

* **No panics** (decode returns typed error).
* **Round-trip**: `decode(encode(x)) == x` for valid `x`; and for **canonical** cases, `encode(decode(bytes)) == canonical_bytes`.
* **Idempotency**: Re-encoding already-canonical CBOR yields identical bytes.
* **Bounds**: Generated OAP `len` never exceeds 1 MiB − header_size; generated chunk sizes ≤ 64 KiB; total offsets + sizes ≤ `size`.

**Run:**

```bash
cargo test -p ron-proto --features proptest -- --nocapture
```

---

### 1.4 Fuzz Tests

**Scope:** Defensive decode paths (no panics):

* `fuzz_targets/decode_manifest.rs`
* `fuzz_targets/decode_oap_header.rs`
* `fuzz_targets/parse_contentid.rs`
* `fuzz_targets/decode_capability.rs`

**Corpus:** Seed from `tests/vectors/*` + crashers/minimized inputs from CI.
**Acceptance:** **No crashes**; ASan/UBSan clean.

**Run (developer quick pass):**

```bash
cargo fuzz run decode_manifest -- -max_total_time=60
```

**CI nightly (silver/gold gates):**

```bash
cargo fuzz run decode_manifest -- -max_total_time=3600
cargo fuzz run decode_oap_header -- -max_total_time=3600
```

---

### 1.5 Chaos/Soak Tests

**Scope:** Service crates (N/A for `ron-proto`).
**Adaptation for lib:** **Parser soak**—millions of decode/encode iterations across vectors and random inputs to detect leaks/alloc spikes.

**Run:**

```bash
cargo test -p ron-proto soak_roundtrip -- --ignored --nocapture
```

---

### 1.6 Performance/Load Tests

**Scope:** Serialization/hashing perf bound to **size bins** (see PERFORMANCE.md).
**Tools:** `criterion` benches in `benches/`:

* `contentid.rs` (tiny: ≤1 KiB, streaming: 64 KiB, large: 1 MiB)
* `manifest.rs` (2/16/1024-chunk shapes)
* `oap1.rs` (header encode/decode micro-bench)

**Run:**

```bash
cargo bench -p ron-proto
```

---

## 2) Coverage & Gates

> Coverage measured with `grcov` (Linux) or `tarpaulin` (fallback).
> Perf regressions measured vs **baselines** per host profile (x86_64-AVX2, arm64-NEON).

### 2.1 Bronze (MVP)

* ✅ Unit + integration tests pass (vectors parity green).
* ✅ Fuzz harness builds with at least one target enabled.
* ✅ Line coverage ≥ **70%** (excludes benches/fuzz).

### 2.2 Silver (Useful Substrate)

* ✅ Property tests present for manifest, OAP/1 header, `ContentId`.
* ✅ Nightly fuzz ≥ **1h/target**, zero crashes.
* ✅ Line coverage ≥ **85%**; branch coverage reported.
* ✅ Parser soak test runs ≥ **10M** decode+encode cycles (no leaks).
* ✅ Perf CI runs; **no size-bin regression >10%** vs baselines.

### 2.3 Gold (Ops-Ready)

* ✅ Fuzz ≥ **4h/night** across all targets; coverage corpora preserved.
* ✅ Soak ≥ **24h** weekly (no FD/memory growth).
* ✅ Line coverage ≥ **90%**; critical modules (parsers) ≥ **95%**.
* ✅ Perf: regression ≤ **10%** in all bins; allocations/op within spec (≤3 enc/dec; ≤5 for 1024-chunk manifest).
* ✅ Public API diff enforced (non-major releases have **no** breaking change).

---

## 3) Invocation Examples

### 3.1 All Tests (unit + integration)

```bash
cargo test -p ron-proto --all-targets -- --nocapture
```

### 3.2 Property Tests only

```bash
cargo test -p ron-proto --features proptest -- --ignored proptest
```

### 3.3 Fuzz Target (quick)

```bash
cargo fuzz run decode_oap_header -- -max_total_time=60
```

### 3.4 Benches (Criterion)

```bash
cargo bench -p ron-proto
```

### 3.5 Coverage (grcov example)

```bash
cargo clean
CARGO_INCREMENTAL=0 RUSTFLAGS="-Zinstrument-coverage" LLVM_PROFILE_FILE="prof-%p-%m.profraw" \
  cargo test -p ron-proto
grcov . --binary-path ./target/debug/ -s . -t lcov --branch --ignore-not-existing -o lcov.info
```

---

## 4) Observability Hooks

* **Structured logs on failure** (see `OBSERVABILITY.md`): tests emit JSON with keys `{ "test": "...", "corr_id": u64, "case": "...", "error": "...", "seed": ... }`.
* **Correlation IDs** propagate through helper macros so cross-crate failures (e.g., SDK parity) can be traced.
* **Vector diffs**: integration tests print **byte-diff hexdumps** (first mismatch offset, expected vs actual) for quick diagnosis.

---

## 5) CI Enforcement

**GitHub Actions jobs (minimum):**

* **Build & Test:** `cargo test --workspace --all-targets`
* **Lint:** `cargo fmt -- --check` and `cargo clippy -D warnings`
* **Public API:** `cargo public-api -p ron-proto --diff-git-checkouts "HEAD^" "HEAD"`
* **Coverage:** grcov/tarpaulin report; enforce gates per Bronze/Silver/Gold
* **Fuzz (nightly):** `cargo fuzz run <target> -- -max_total_time=3600` (Silver) / 4h (Gold)
* **Perf:** run `cargo bench -p ron-proto`; compare against `testing/performance/baselines/ron-proto.json` per **size bin**
* **Deny:** `cargo deny check` (advisories/licenses)
* **SDK Matrix (post-release):** ensure vectors parity across Rust/TS/Py/Swift

---

## 6) Open Questions (crate-specific commitments)

* **Loom:** N/A (no async/stateful concurrency); if helper types gain interior mutability later, add loom plan here.
* **Mandatory fuzz targets:** `decode_manifest`, `decode_oap_header`, `parse_contentid`, `decode_capability`.
* **Perf SLOs:** Measured per **size bin** (≤1 KiB ops/sec, 64 KiB bytes/sec, 1 MiB bytes/sec). Thresholds: ≤10% regression vs baseline; allocations/op ceilings as in PERFORMANCE.md.

---

## 7) Directory & File Conventions (for test hygiene)

* `tests/vectors/` — canonical CBOR/JSON vectors (do **not** hand-edit).
* `tests/*.rs` — integration tests (parity, hash truth, cross-version).
* `benches/*.rs` — Criterion benches (size-binned).
* `fuzz/fuzz_targets/*.rs` — decode/parse fuzzers.
* `testing/performance/baselines/ron-proto.json` — perf baselines (per host profile).

All new vectors/fixtures must land atomically with code changes; SDK fixtures updated in a coordinated series.

---

✅ With this testing contract, `ron-proto` guarantees deterministic encoding, strict schema hygiene (`deny_unknown_fields`), robust decode defense (fuzz/property), and **CI-enforced** perf/coverage gates—eliminating interop drift across crates and SDKs.
