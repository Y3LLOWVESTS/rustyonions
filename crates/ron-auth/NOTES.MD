### BEGIN NOTE - NOVEMBER 8 2025 - 10:10 CST

# ron-auth — Carry-Over Notes from **svc-passport** (Blueprint for Cohesive Build)


## 0) TL;DR (what to inherit + why)

* **Keep the contract tiny and boring.** svc-passport proved that a small, stable JSON API with strict limits, solid metrics, and fast negative paths is enough to get God-tier reliability and perf.
* **Reuse invariants verbatim.** Ed25519 only, audience gating, body caps, concurrency caps, structured errors, `/metrics`, `/healthz`, optional `/readyz`.
* **Delegate crypto.** ron-auth should **never** hand-roll crypto; **verify via svc-passport** (or ron-kms via the same trait boundary later). Batch whenever possible.

---

## 1) ron-auth’s role in RON-CORE (scope & boundaries)

**AuthN:** turn a request (passports, API keys, or future login artifacts) into a **Subject** with stable **Claims**.
**AuthZ:** evaluate **Subject × Action × Resource × Context** against policy (via `ron-policy`) and return **Permit | Deny** (with reasons).

**Not in scope (for first beta):**

* Full user directory/IdP, password flows, SSO/OIDC provider.
* Long-term session store with refresh tokens.
* Web3 identities (defer per pivot).

**In scope (beta):**

* **Passport-based AuthN** (single and batch).
* **Policy-driven AuthZ** (call `ron-policy` locally; no remote net I/O).
* **Decision endpoints**: `/v1/auth/whoami`, `/v1/auth/authorize`, `/v1/auth/authorize_batch`, `/v1/auth/introspect`.
* **Fast path caching** (optional, bounded TTL on positive verifies).
* **Ops plane**: `/healthz`, `/readyz`, `/metrics`.
* **Strict limits** and **consistent problems** (same style as svc-passport).

---

## 2) Hard requirements we should copy from **svc-passport**

* **Algorithms:** accept only `Ed25519` envelopes; reject everything else early.
* **Audience:** gated when `[security].require_aud=true` and aud must match configured audience(s) for ron-auth.
* **Body size:** enforce `limits.max_msg_bytes` (413).
* **Batch size:** enforce `limits.max_batch` with 413 on `/authorize_batch`.
* **Concurrency:** `ConcurrencyLimitLayer` on hot paths (env-tunable).
* **Error model:** small set of stable problem codes (`BadAlg`, `BadKid`, `BadAudience`, `Malformed`, `MsgTooLarge`, `BatchTooLarge`, `Denied`, `Internal`).
* **Metrics:** op counters + histograms (names below).
* **Config precedence:** `RON_AUTH_CONFIG` (inline TOML) > `RON_AUTH_CONFIG_FILE` > repo default.
* **JWKS handling:** don’t do crypto key logic in ron-auth—**call svc-passport `/v1/keys`** if you need to hydrate a local VK cache (optional). Prefer calling **svc-passport `/verify(_batch)`** instead of verifying locally.

---

## 3) Proposed API surface (beta)

```
GET  /healthz
GET  /readyz
GET  /metrics

POST /v1/auth/whoami
  -> Input: Envelope (same as svc-passport)
  -> Output: { sub, aud?, iat?, exp?, scopes?, roles?, device?, tenant? }
     (Claims derived from signed msg; if msg is arbitrary JSON, we map known fields conservatively.)

POST /v1/auth/introspect
  -> Input: Envelope or { token: <opaque> } (alias to Envelope for now)
  -> Output: { active: bool, alg: "Ed25519", kid, aud?, sub?, warnings?[] }

POST /v1/auth/authorize
  -> Input: { envelope: Envelope, action: "read|write|...", resource: "uri-or-id", context?: {…} }
  -> Output: { decision: "Permit"|"Deny", reason?: "PolicyRuleId", obligations?: {…} }

POST /v1/auth/authorize_batch
  -> Input: { items: [ { envelope, action, resource, context? }, ... ] }  OR bare array
  -> Output: [ { decision, reason?, obligations? }, ... ]
```

**Why this set:** Minimal surface that many services need immediately (identify subject, check access). Keeps envelopes as the single auth artifact (no second token layer yet).

---

## 4) DTOs (keep them boring & explicit)

* **Envelope**: identical to svc-passport (`{ alg, kid, msg_b64, sig_b64, aud? }`).

* **Subject/Claims** (internal struct):

  ```
  Subject { sub: String, aud: Option<String>, scopes: Vec<String>, roles: Vec<String>,
            tenant: Option<String>, device: Option<String>, iat: Option<i64>, exp: Option<i64> }
  ```

  Only populate fields that are **present in the signed message**. No synthesis.

* **Decision**: `{ decision: "Permit"|"Deny", reason?: String, obligations?: Map }`

---

## 5) Config (mirror svc-passport style)

```toml
[server]
bind = "127.0.0.1:5310"
admin_bind = "127.0.0.1:5311"

[limits]
max_msg_bytes = 4096
max_batch = 512

[security]
require_aud = true
accepted_audiences = ["ron-auth"]  # Default: service logical name

[verify]
# When true, call svc-passport /verify(_batch); when false (future), verify locally via ron-kms.
delegate_to_passport = true
passport_base = "http://127.0.0.1:5307"

[caching]
verify_positive_ttl_ms = 250     # Optional micro-cache for (kid,msg,sig) → true
verify_negative_ttl_ms = 50      # Optional micro-cache for stable negatives
jwks_ttl_s = 60                  # If we ever hydrate VKs locally
```

**Env precedence:**

1. `RON_AUTH_CONFIG` (inline TOML)
2. `RON_AUTH_CONFIG_FILE` (path)
3. Default file in crate

**Tunable envs** (fast overrides):
`RON_AUTH_VERIFY_CONCURRENCY`, `RON_AUTH_AUTHORIZE_CONCURRENCY`, `RON_AUTH_MAX_MSG_BYTES`, `RON_AUTH_MAX_BATCH`.

---

## 6) Inter-crate call patterns (keep cohesive)

* **ron-auth → svc-passport:**

  * Prefer `POST /v1/passport/verify_batch` with the **array form** for best throughput.
  * On startup (optional) hit `/v1/keys` once to warm any VK cache; respect `ETag` and `Cache-Control`.

* **ron-auth ↔ ron-policy:**

  * Build a small in-proc evaluator boundary: `fn authorize(subject:&Subject, action:&str, resource:&str, ctx:&Value) -> Decision`.
  * Keep policy bundles hot-reloaded if `ron-policy` supports it; otherwise load at boot.

* **svc-gateway / omnigate:**

  * They call ron-auth `/authorize(_batch)` before routing/dispatch, or annotate upstream request context with a `Permit`/`Deny` decision and reason ID.

---

## 7) Negative-path & limits (test matrix to port over)

* `alg != Ed25519` → 400 `BadAlg`
* empty `kid` → 400 `BadKid`
* missing required `aud` when `security.require_aud=true` → 400 `BadAudience`
* `issue` doesn’t exist here; **authorize** oversize request → 413 `MsgTooLarge`
* `authorize_batch` length > `limits.max_batch` → 413 `BatchTooLarge`
* malformed base64 in envelope → 400 `Malformed`
* valid envelope but **policy Deny** → 200 with `{ decision:"Deny", reason:"<RuleId>" }` (Deny is not an error)

---

## 8) Metrics (names & cardinalities)

**Counters**

* `auth_ops_total{op="whoami|introspect|authorize|authorize_batch", result="ok|deny|fail", alg="Ed25519|n/a"}`

**Histograms**

* `auth_op_latency_seconds{op}`
* `auth_batch_len` (for `/authorize_batch`)
* Optional: `auth_verify_delegate_seconds` (latency of svc-passport call), `auth_policy_eval_seconds`

**Gauges**

* `auth_ready` (0/1)
* Optional caches: `auth_verify_cache_items`

**Smoke sanity:** ensure at least one increment per op; verify `deny` shows when a synthetic Deny case is tested.

---

## 9) Observability & ops plane

* `/healthz` (liveness): always cheap and fast.
* `/readyz` (readiness): flip **true** after listeners are bound and (optionally) after first successful call to svc-passport `/v1/keys` or `/verify`.
* `/metrics`: Prometheus exposition.

**Logs:** structured with route, decision, rule id (on Deny), `kid`, and a request id. **No** secrets.

---

## 10) Caching (safe, minimal, optional)

* **Positive verify micro-cache** keyed by `(kid, sig_b64, msg_hash)` with **very small TTL** (100–250 ms).

  * Avoids repeat verifies within bursts; bounded memory (LRU).
* **Negative micro-cache** (optional, 50 ms) for obviously bad alg/kid/aud to shed load.
* **JWKS cache**: only if we add local verification later; for Beta keep **delegation** to svc-passport as the default.

---

## 11) Performance expectations (on laptop baseline)

* Delegated verify adds one local HTTP hop when auth sits next to passport. With loopback and small JSON the added latency is typically **<10–20 µs** beyond verify cost.
* Batch authorize should approach **verify_batch** per-item cost + policy eval (µs-level if policy is local and simple).
* Target (laptop, per core):

  * `authorize` ≈ **50–70 µs** typical (verify + trivial policy).
  * `authorize_batch_64` ≈ **40–50 µs/item**.

---

## 12) Test & bench plan (copy svc-passport patterns)

**Unit tests**

* Envelope validation: alg/kid/aud/base64.
* Subject extraction from `msg_b64` (known fields only).
* Policy decisions: Permit and Deny with clear reasons.

**Integration tests**

* **Delegate happy path**: valid envelope → `true` from svc-passport → `Permit` when policy allows.
* **Delegate deny**: force a policy Deny; keep HTTP 200 with `{decision:"Deny"}`.
* **Batch**: `{items:[…]}` and array forms; over-batch → 413.
* **Readiness flip** on boot; shutdown drains.

**Benches (Criterion)**

* `authorize_single` (with real delegate to local svc-passport).
* `authorize_batch_64`.
* Save baselines with `critcmp` compatibility as we did for svc-passport.

**Scripts**

* Mirror `scripts/smoke_passport.sh` as `scripts/smoke_auth.sh`:

  * spawn on fixed/dynamic port, probe `/healthz`, hit `/metrics`, exercise whoami/authorize/authorize_batch, include a Deny case, and stop the process.
  * Accept `RON_AUTH_CONFIG(_FILE)`; optional `AUTH_DYNAMIC_PORT=1`.

---

## 13) Security posture (carry-over)

* **Zero ambient authority:** decisions must be derived from **signed envelopes** (and policy), not from headers you don’t trust.
* **Strict rejects before crypto:** early checks for alg/aud/kid/size; only then call delegate verify.
* **Deny is not error:** keep Deny as a 200 with a clear reason; use HTTP errors for malformed or policy/engine failures only.
* **No secret leakage** in errors or logs.
* **Amnesia mode ready:** be able to run with caches disabled/ephemeral; don’t persist subject info by default.

---

## 14) Nice-to-haves beyond Beta

* **Session facade** (stateless): mint a short-lived **auth decision cookie** (HMAC’d) for web UIs to cut repeat authorizes—documented and opt-in.
* **PEP/PDP split:** optional mode where ron-auth (PDP) is called remotely by gateways (PEP) via gRPC/HTTP with batch.
* **Richer policy context:** geo/time/device risk signals; expose a `context` schema section.
* **Rate limiting/quotas** pre-auth (middleware) for abusive clients.
* **Admin plane**: `/admin/policy/reload`, `/admin/decision_trace` for debugging.

---

## 15) Reusable code/style conventions (keep consistency)

* **Axum 0.7** patterns exactly as in svc-passport: Router<()> + `Extension(Arc<_>)`; `DefaultBodyLimit::max` per route; `ConcurrencyLimitLayer`.
* **Problem envelopes**: same struct shape and code strings.
* **Base64:** `use base64::Engine;` with `general_purpose::STANDARD` (0.22 API).
* **ETag practice:** if any list endpoints appear (e.g., policy list), use **BLAKE3** over exact bytes + `If-None-Match`.
* **Docs:** each file uses `CODECOMMENTS.MD` style headers (RO:WHAT/WHY/INTERACTS/INVARIANTS).

---

## 16) Rollout order (small, safe steps)

1. Scaffold crate with ops plane, config loader, metrics registry, problem types, limits/concurrency, and empty handlers.
2. Implement **delegate verify** client (svc-passport) and whoami.
3. Add policy bridge and `authorize`.
4. Add `authorize_batch` + tests/benches.
5. Smoke script + metrics sanity.
6. `/readyz` + graceful shutdown.
7. README/runbook + baselines.

---

## 17) What to copy directly from **svc-passport**

* Config precedence loader logic (env inline/file/default) and test harness pattern.
* Problem type + helper `problem(StatusCode, code, msg)`.
* Body limit and concurrency env knobs.
* `/metrics` exporter binding and counter/histogram registration.
* The **smoke script** structure (spawn, wait_for_up, hit endpoints, teardown).

---

## 18) Integration expectations with the rest of RON-CORE

* **svc-gateway / omnigate:** call `/v1/auth/authorize(_batch)` early; cache **Permit** decisions briefly at the edge.
* **ron-policy:** load bundle on boot; provide a version in `/admin/attest` so operators can correlate policy rollouts.
* **ron-kms:** future switch (feature flag) to local verification—**do not change the public API** when enabling this.

---

## 19) Acceptance criteria for **ron-auth Beta**

* `cargo fmt && cargo clippy -D warnings && cargo test` green on macOS/Linux.
* `scripts/smoke_auth.sh --spawn` passes (whoami, authorize ok + deny case, batch ok, metrics present).
* Strict limits and negative tests pass.
* Metrics show all ops counters > 0 after smoke; histograms populated.
* Delegation path to svc-passport is resilient (timeouts, clear errors, small retries; no thundering herd).

---

## 20) Quick commands (reference)

**Dev sweep**

```
cargo fmt -p ron-auth
cargo clippy -p ron-auth --no-deps -- -D warnings
cargo test -p ron-auth
```

**Smoke (fixed)**

```
RON_AUTH_CONFIG_FILE=crates/ron-auth/config/default.toml \
crates/ron-auth/scripts/smoke_auth.sh --spawn
```

**Smoke (dynamic)**

```
AUTH_DYNAMIC_PORT=1 RON_AUTH_CONFIG_FILE=crates/ron-auth/config/default.toml \
crates/ron-auth/scripts/smoke_auth.sh --spawn
```

**Benches**

```
cargo bench -p ron-auth -- --measurement-time 10 --sample-size 100
```

---

### Final note

svc-passport gave us the template: tiny surface, strict invariants, solid ops, and speed. ron-auth should look and feel the same—just swapping **“sign/verify”** for **“whoami/authorize”** while **delegating verification to svc-passport** and **localizing policy**. This keeps the core cohesive and makes future productionization (KMS, richer policy, PDP/PEP split) incremental and low-risk.

### END NOTE - NOVEMBER 8 2025 - 10:10 CST

### CURRENT BENCH RESULTS NOTE:


Here’s the quick, plug-and-play rundown of each benchmark and what the numbers mean.

### Bench suite (what each test does)

* **verify_single** — Verifies one token end-to-end (Base64URL→CBOR decode, MAC check, caveat eval). Measures pure single-item latency.
* **verify_batch_64_loop** — Verifies 64 identical tokens by calling `verify_token` in a loop. Shows amortization from shared set-up but still pays per-call overhead.
* **verify_many_64** — Verifies 64 tokens via the amortized/buffer-reusing path (`verify_many`/`*_into`). This is our optimized batch evaluator.
* **verify_single_heavy** — Like `verify_single`, but with a “heavy” token (many caveats / costlier eval). Stresses evaluator and CBOR decode.
* **verify_many_64_heavy** — 64 heavy tokens through the amortized path. Stresses columnar/streaming hybrid, buffer reuse, and cache behavior.

### Your results (central estimate ≈ median of the bracket)

* **verify_single:** **3.80 µs** (3.776–3.819 µs)
  Rough throughput ≈ **263k tokens/sec**.

* **verify_batch_64_loop:** **239.7 µs** for 64 ⇒ **3.75 µs/token**
  Rough throughput ≈ **267k tokens/sec**; loop batching removes a tiny bit of overhead.

* **verify_many_64:** **235.1 µs** for 64 ⇒ **3.67 µs/token**
  Rough throughput ≈ **272k tokens/sec**; amortized/buffer-reusing path is **~1.9–3.3% faster** than the simple loop (and ~3% better than `verify_single` per token).

* **verify_single_heavy:** **16.72 µs**
  Rough throughput ≈ **59.8k tokens/sec**; the extra caveat/CBOR work dominates.

* **verify_many_64_heavy:** **1.033 ms** for 64 ⇒ **16.14 µs/token**
  Rough throughput ≈ **62k tokens/sec**; amortized batch wins **~3.5%** vs heavy single per-token.

### Takeaways

* The batch evaluator (`verify_many_*`) is doing its job: modest win for light tokens (~2–3%) and a bigger relative win on heavy tokens (~3–4%) thanks to buffer reuse and fewer per-item inits.
* Absolute latencies are stable and tight; outliers (5–13%) are typical Criterion noise on a dev laptop.
* For production throughput estimates, you can quote:

  * **~270k verifications/sec** for typical (“light”) tokens on a single thread.
  * **~60–62k verifications/sec** for “heavy” tokens on a single thread.
* Any regression threshold to flag: >5–7% change vs this baseline.



### END CURRENT BENCH RESULTS NOTE


### BEGIN NOTE - NOVEMBER 8 2025 - 14:23 CST

Here’s a paste-ready, comprehensive carry-over for **ron-auth** so we can drop into the next instance and keep sprinting. I folded in Grok’s analysis and pushed further on God-tier optimizations (SIMD, parallel, cache-line, alloc, algorithmic). This is meant to live at:

`crates/ron-auth/NOTES.CARRYOVER.md`

---

# ron-auth — Carry-Over Notes (Nov 8, 2025)

## 0) TL;DR (State of the World)

* ✅ **Clean build & tests** (all green).
* ✅ **Hybrid evaluator**: streaming (tiny tokens) + SoA (large tokens).
* ✅ **Refactor complete**: `verify/pipeline.rs` split into `streaming.rs`, `soa.rs`, `soa_eval.rs`, and small `pipeline.rs` (~230 LOC).
* ✅ **Bench baselines** (single-thread, 2019 MBP i5-8257U):

  * `verify_single` ≈ **3.8 µs** (≈263k/sec)
  * `verify_many_64` ≈ **235 µs** total ⇒ **3.67 µs/token** (≈272k/sec)
  * Heavy tokens: **~16–16.7 µs/token** (≈60–62k/sec)
* ✅ `verify_many` amortizes ~2–4% vs loop; heavy paths show bigger relative win.
* 🔒 **Invariants**: pure lib (no I/O), bounded work, CBOR canonical, URL-safe Base64 (no pad), keyed **BLAKE3** MAC, constant-time compare, **no `unsafe`**.
* 🟨 **Bench-only toggles** exist (`*_streaming_only`, `*_soa_only`). Currently public; we’ll gate them to avoid dead-code warnings.
* 🎯 **Goal next**: push below **3.0 µs** per light token on dev hw; larger gains on modern server cores. Batch win target **10–20%** via deeper amortization & SIMD.

---

## 1) What we finished this session

* **Architecture cleanup**

  * Introduced **hybrid verification pipeline** with a **threshold** (`SOA_THRESHOLD = 8`).
  * Split 450+ LOC monolith into:

    * `verify/pipeline.rs` — orchestration (decode + MAC + choose streaming vs SoA + collect `Decision`).
    * `verify/streaming.rs` — fast path, early short-circuits for `Exp`/`Nbf`, minimal allocations.
    * `verify/soa.rs` — **struct-of-arrays** converter (`CaveatsSoA`) packing caveats into columns (e.g., `exp`, `nbf`, `aud`, `method`, `path_prefix`, `ip_cidr`, `bytes_le`, `tenant`, `amnesia`, `gov_policy_digest`, `custom`, `rate`).
    * `verify/soa_eval.rs` — columnar scan evaluator; hard-errors early for time bounds.
    * `verify/parse.rs` — Base64URL→CBOR decode helpers with scratch buffer; fixed clippy (`div_ceil`) and kept **serde_cbor**.
  * Eliminated redundant imports + fixed type mismatches (`Rate { burst: u64, per_s: u32 }` → typed push).
  * **Removed** accidental `#[cfg(feature = "bench-eval-modes")]` guards that weren’t in `Cargo.toml`.
  * **Made the bench-only entrypoints work** and stopped toolchain complaining by (temporarily) keeping them public; plan to gate.

* **Bench suite**

  * `benches/verify_bench.rs` covers:

    * `verify_single`, `verify_batch_64_loop`, `verify_many_64`
    * `verify_single_heavy`, `verify_many_64_heavy`
  * Stable results with tight brackets; baselines saved via Criterion CLI.

---

## 2) Current file map (relevant bits)

```
crates/ron-auth/
  src/
    lib.rs
    bounds.rs
    builder.rs        # CapabilityBuilder + sign_and_encode_b64url
    cbor.rs           # canonical CBOR helpers (encode, etc.)
    errors.rs         # AuthError, DenyReason
    mac.rs            # BLAKE3 keyed MAC + constant-time compare
    types.rs          # Capability, Caveat, Decision, RequestCtx, VerifierConfig, MacKey, MacKeyProvider
    verify/
      mod.rs          # pub use verify_token / verify_many / verify_many_into
      pipeline.rs     # decode + MAC + hybrid dispatch + output assembly
      streaming.rs    # streaming evaluator
      soa.rs          # CaveatsSoA (columnar) + from_slice
      soa_eval.rs     # evaluator over SoA columns
      parse.rs        # decode_b64url_cbor_capability_with_buf + helpers
  benches/
    verify_bench.rs
  tests/
    allow_deny_vectors.rs
    ip_cidr.rs
    ...
```

---

## 3) How to run (repeatable commands)

**Unit + doctests**

```bash
cargo fmt -p ron-auth
cargo clippy -p ron-auth --no-deps -- -D warnings
cargo test -p ron-auth
```

**Benches (save baseline with timestamp)**

```bash
cargo bench -p ron-auth --bench verify_bench -- \
  --warm-up-time 2 --measurement-time 6 \
  --save-baseline auth-$(date +%Y%m%d-%H%M%S)
```

**Compare to a previous baseline** (from Criterion target dir):

```bash
# Example: compare latest vs. a named baseline
cargo bench -p ron-auth --bench verify_bench -- --baseline <BASELINE_NAME>
```

**(Optional) Flamegraph hotspotting** (install cargo-flamegraph first):

```bash
cargo flamegraph -p ron-auth --bench verify_bench -- verify_many_64
```

---

## 4) Benchmark snapshot (to track regressions)

* `verify_single`: **3.776–3.819 µs** (≈ **3.80 µs** median).
* `verify_batch_64_loop`: **238.7–240.7 µs** (≈ **3.75 µs/token**).
* `verify_many_64`: **234.4–235.9 µs** (≈ **3.67 µs/token**).
* `verify_single_heavy`: **16.65–16.80 µs**.
* `verify_many_64_heavy`: **1.028–1.038 ms** (≈ **16.14 µs/token**).

**Policy**: Treat > **±5–7%** change as a perf regression/upgrade that must be explained.

---

## 5) Immediate cleanup (quick wins, safe)

1. **Avoid `reasons.clone()` on deny**
   In `pipeline.rs`, where we do:

   ```rust
   out.push(Decision::Deny { reasons: reasons.clone() });
   ```

   switch to:

   ```rust
   out.push(Decision::Deny { reasons: std::mem::take(&mut reasons) });
   ```

   Then reuse the same buffer. (We already clear `reasons`; `take` avoids clone.)

2. **Use `SmallVec` for deny reasons**
   Typical mismatches are small (≤4). Change `out: &mut Vec<DenyReason>` → `&mut SmallVec<[DenyReason; 8]>` in evaluators & pipeline stack var. This eliminates heap for the hot path.

   * Add dep: `smallvec = "1.13"`
   * Keep public API returning `Vec<Decision>` unchanged.

3. **Normalize once, compare fast**

   * Pre-normalize `ctx.method` to **upper ASCII** at construction; store token methods normalized during signing; then use `==` (no `eq_ignore_ascii_case` in loops).
   * Precompute `ctx.path.as_bytes()` and use `starts_with` on bytes.

4. **Streaming IpCidr parse**
   In streaming fast path we currently do `IpNet::from_str(s)` per caveat. Either:

   * (A) teach `cbor::decode_*` to build `Caveat::IpCidr` as **parsed** `IpNet` (breaking serialized shape → opt-in feature), or
   * (B) add a tiny **parse cache** (last N strings → IpNet), e.g., `lru = 16`. On realistic traffic with repeated CIDRs, this is a free win.

5. **Gating bench-only fns**
   To silence `dead_code` in prod builds:

   * Add a **feature**:

     ```toml
     [features]
     benches = []
     ```
   * Wrap the 4 bench toggles in `#[cfg(feature = "benches")]`.
   * In benches: `cargo bench -p ron-auth --features benches ...`

---

## 6) Bigger perf levers (God-tier plan)

### A) SIMD & zero-copy decode

1. **SIMD Base64URL**

   * Swap to `base64-simd` or `fastbase64` behind a feature flag:

     * Expect **1.3–1.8×** faster decode vs `base64` on x86/ARM.
     * Plumb into `decode_b64url_cbor_capability_with_buf`, decoding directly into the **scratch** buffer (no intermediate `String`), then CBOR-decode from that slice.

2. **CBOR faster path**

   * Today: `serde_cbor`. Options:

     * **minicbor** with borrowing where possible (zero-copy strings & byte slices).
       Expect **10–25%** speed-up on decode, lower alloc.
     * Keep serde for ergonomics; introduce feature `fast-cbor` to switch.
   * For **SoA**: parse directly into **column buffers** (skip constructing a full `Vec<Caveat>` when caveat count is large). That cuts allocations & branches.

3. **BLAKE3 MAC**

   * BLAKE3 already SIMD-accelerated. Ensure we feed it in **one shot** from the canonical CBOR bytes (no extra concat).
   * If the CBOR build step during signing is stable, consider **prehashing** immutable token parts and MAC only the tunneled payload (requires a domain-separated construction that we already have—verify).

### B) Batch-level amortization (10–20% target)

1. **Two-phase batch**:
   For `verify_many`:

   * Phase 1 (linear pass): **Base64URL decode all** into per-item scratch slices, collect lengths.
   * Phase 2: **CBOR decode + MAC** each.
   * Phase 3: **Evaluate** (streaming vs SoA) per token, but reuse **shared small buffers** (e.g., `SmallVec` for reasons).
     This reduces interleaving of different working sets → better cache locality.

2. **SIMD-friendly micro-batches**:
   Group items by **caveat count bucket** (e.g., 0–8 → streaming, 9–32 → SoA-S, 33+ → SoA-L). Avoid branchy mixed loops. Expect wins in heavy distributions.

3. **Tune `SOA_THRESHOLD` adaptively**:

   * Make it a field in `VerifierConfig` (default 8).
   * Add bench sweep (`--features benches`) for threshold ∈ {4, 8, 12, 16, 24}.
   * Pick per-deployment sweet spot based on real tokens.

### C) Parallelism

1. **Rayon for `verify_many`**

   * Add a `parallel` feature: if enabled and batch size ≥ `P_MIN` (e.g., 128), run tokens via `par_iter()` with a **ScopedThreadPool** and **per-thread scratch**.
   * BLAKE3 and CBOR are CPU bound and thread-safe → expect near **linear scaling** to 3–4 cores on laptops, more on servers.
   * Preserve determinism of outputs (keep input order). Use `par_iter().enumerate()` and write into `out[index]`.

2. **BLAKE3 tree mode (advanced)**

   * Not a drop-in for MAC (each token uses a separate key), but **per-thread** BLAKE3 will scale well. We just need to ensure we never share state (we don’t).

### D) Alloc & cache discipline

1. **Scratch buffers**

   * Keep **one `Vec<u8>` scratch** per worker; grow-but-never-shrink.
   * If using parallel mode: **thread-local** scratch via `thread_local!` or `rayon::ThreadLocal`.

2. **Method matching**

   * Replace `Vec<String>` with **`SmallVec<[Method; 4]>`** where `Method` is an enum (`GET`, `POST`, …, `OTHER(String)`), or **bitmask** if the set is finite. That turns an O(k) string scan into **O(1)** bit-test.

3. **Deny reasons encoding**

   * Convert common reasons to **bitflags** and map to vector only if needed for error reporting. That lets the hot path be **branch- and alloc-light**.

4. **Path prefix**

   * Store prefixes as **bytes**; perform `starts_with` on `[u8]` (we already hold UTF-8). Avoids repeated Unicode handling.

5. **IpNet**

   * In **SoA** we already store parsed `Option<IpNet>`. For streaming, either **parse once in decode** (feature) or use an **L1-sized cache** (see §5-4).

### E) Observability for perf

* Add a `scripts/perf_sweep.sh` that:

  * Sweeps batch sizes (1, 8, 64, 256, 1024)
  * Sweeps threshold (4, 8, 12, 16, 24)
  * Optionally enables `--features simd,fast-cbor,parallel`
  * Emits a CSV of per-token µs + stddev for CI trend charts.
* Add **alloc counters** via `jemalloc-ctl`/`tikv-jemallocator` feature to report allocation rate per verify.

---

## 7) API + config nits to line up now

* **Make `SOA_THRESHOLD` configurable**:

  ```rust
  pub struct VerifierConfig {
      pub max_token_bytes: usize,
      pub max_caveats: usize,
      pub clock_skew_secs: i64,
      pub soa_threshold: usize, // NEW (default 8)
  }
  ```
* **Bench-only entrypoints**
  Wrap in `#[cfg(feature = "benches")]` and expose `pub(crate)` instead of `pub`:

  * `verify_token_streaming_only`
  * `verify_token_soa_only`
  * `verify_many_streaming_only`
  * `verify_many_soa_only`
* **Context normalization**
  Add a helper to construct `RequestCtx` that uppercases method and interns policy digest if provided.

---

## 8) Safety + correctness gates (don’t regress)

* Time semantics: `Exp` and `Nbf` are **hard errors** (we return `AuthError::Expired` / `NotYetValid`) — keep this consistent in both streaming & SoA.
* MAC domain separation: confirm we MAC over **exact canonical CBOR** (scope + caveats + kid + tid + …) in a stable order; never re-serialize in verification. We already compute over deserialized fields, but ensure it reflects the original canonical bytes or a rigorously equivalent domain—if in doubt, store and MAC the raw CBOR bytes included in the token and verify against them.
* Bounds: keep `max_token_bytes`, `max_caveats` checks **before** heavy work.
* Constant-time compare for MAC — we already use `macs_equal`.
* No I/O or global state in verification path; parallel path must keep **thread-local** scratch.

---

## 9) Forecast (realistic)

* **SIMD Base64 + faster CBOR** alone should get **~10–25%** win for light tokens (decode-dominated).
* **Two-phase batch** should add **~5–10%** (cache locality + fewer branches).
* **Parallel** (4 cores) yields **~3–3.5×** throughput on dev laptop; servers go higher.
* **Method bitmask + SmallVec + take()**: another **~3–8%** in evaluator micro-hotspots.
* Combined, sub-**3.0 µs** per light token on dev laptop is plausible; on modern servers, **≤2.0 µs**.

---

## 10) Action list (next instance)

1. **Micro-patches (fast, safe)**

   * [ ] Replace `reasons.clone()` with `mem::take`.
   * [ ] Switch evaluator temp vectors to `SmallVec<[DenyReason; 8]>`.
   * [ ] Normalize `RequestCtx.method` once; use case-insensitive normalization at source.
   * [ ] Add `VerifierConfig.soa_threshold` (default 8).

2. **Feature flags**

   * [ ] `benches` (gates bench-only fns).
   * [ ] `simd-b64` (use `base64-simd`).
   * [ ] `fast-cbor` (try `minicbor` zero-copy).
   * [ ] `parallel` (rayon batch path).

3. **Batch re-plumb**

   * [ ] Implement two-phase pipeline in `verify_many_into` (decode-all → cbor+mac → eval).
   * [ ] Group by caveat bucket to reduce divergence.

4. **Streaming IpCidr cost**

   * [ ] Option A (feature): decode `IpCidr` as parsed `IpNet`.
   * [ ] Option B: tiny LRU for `IpCidr` strings (16 entries).

5. **Perf sweeps + flamegraph**

   * [ ] Add `scripts/perf_sweep.sh` and CSV output.
   * [ ] Run flamegraphs to confirm hotspots (expect Base64/CBOR/MAC+method scan).

6. **Docs**

   * [ ] Update README with perf numbers, features, and tuning knobs.
   * [ ] Add `PERFNOTES.md` summarizing the thresholds and hardware deltas.

---

## 11) How this fits svc-passport & ron-kms

* **ron-kms** remains the source for signing/rotation; ron-auth verifies. Keep **KID/TID versioning** and strict verify.
* **svc-passport** issues tokens; align **method normalization** and **IpCidr** encoding with ron-auth’s fast paths (e.g., uppercase methods, pre-parsed network literals behind a feature if we standardize CBOR shapes).
* For **large tokens** (gov caveats, audit scopes), prefer **SoA path**; consider splitting non-authoritative caveats into policy refs to avoid payload bloat.

---

## 12) Known open nits

* Bench-only functions still public unless we add `features.benches`.
* Streaming evaluator still does per-caveat `IpNet::from_str`; we want to avoid that in hot path.
* Methods are `Vec<String>` in tokens; after normalization work we can flip to a compact enum/bitmask without breaking external format (store as strings; map to enum at decode).

---

## 13) Drop-in talking points (Grok & team)

* **Numbers**: 3.8 µs/token (light), 16.7 µs (heavy), 64-batch amortization 2–4% (light) / ~3–4% (heavy).
* **Throughput**: ~270k/sec/thread (light); ~60–62k/sec/thread (heavy).
* **God-tier path to 10/10**: SIMD Base64 + zero-copy CBOR + batch two-phase + method bitmask + SmallVec + optional parallel.
* **SLO confidence**: Low variance (1–4%); deterministic; no alloc spikes.

---

### END NOTE - NOVEMBER 8 2025 - 14:23 CST




### BEGIN NOTE - NOVEMBER 8 2025 - 19:48 CST


# CARRY-OVER NOTES — `ron-auth` God-Tier Push (2025-11-08)

## 0) TL;DR (where we are)

* **Build + tests are green.** SIMD Base64 path integrated; constant-time MAC compare fixed; benches runnable; `critcmp` installed and working.
* **Best results (solo-thread):** the **`simd-b64`** feature is consistently best; **`fast-cbor` (ciborium)** regressed on this laptop — keep it **off** for now.
* **Not maxed:** There’s still meaningful headroom via **parallel batch verify**, **target-CPU/bench profile flags**, micro-tuning thresholds, and (later) **true zero-copy CBOR** (`minicbor`).

---

## 1) What we shipped in this instance (accomplishments)

### Functional / API

* Stable verification pipeline split: `streaming` (small caveat sets) and `soa` (columnar) with auto crossover.
* Public exports wired: `verify_token`, `verify_many`, `verify_many_into`.

### Performance

* **SIMD Base64 decode** behind `simd-b64` (via `base64-simd`), measurable wins on heavy tokens (decoding-bound segments).
* **Constant-time MAC compare** fixed: replaced `subtle` dependency with a pure constant-time loop (no allocations, no early exit).
* **SmallVec + scratch reuse** throughout hot paths to limit allocs.
* Bench harness (`criterion`) + **baseline naming** and **`critcmp`** usage validated.

### Build & hygiene

* `Cargo.toml` features organized:

  * `simd-b64` ✅
  * `parallel` (Rayon) 🔒 **ready to turn on** with one small bound change (below)
  * `fast-cbor` currently maps to `ciborium` (kept optional; **do not use** — slower on this machine).
* Errors from earlier long CLI flags sorted; we’re using criterion’s `--save-baseline` flow that the binary understands.

---

## 2) Benchmarks we ran (what each means + your observed numbers)

> Machine: 2019 MBP 13" i5; ordinary dev laptop; single-thread benches unless noted.

* **`verify_single`** — end-to-end verify of a *small* token (few caveats).

  * Purpose: latency of the common case.
  * Best: ~**3.7–3.9 µs** (SIMD on); comparable baselines within noise.

* **`verify_batch_64_loop`** — loop 64× `verify_token` (no shared amortization).

  * Purpose: per-call overhead when called repeatedly, no vectorized path.
  * Best: **~247–249 µs** for 64 tokens (≈3.9 µs/token).

* **`verify_many_64`** — the amortized vector path (shared scratch, fewer temporaries).

  * Purpose: batch efficiency and crossover vs. the loop.
  * Best: **~236–260 µs** for 64 tokens (≈3.7–4.1 µs/token).
  * On your runs, `simd-b64` tends to edge out `plain`.

* **`verify_single_heavy`** — single *heavy* token (~30 caveats; methods, prefixes, CIDRs, bounds).

  * Purpose: worst-case single-token latency.
  * Best: **~16.0–16.3 µs** (SIMD on).

* **`verify_many_64_heavy`** — batch 64 heavy tokens (decode + MAC + caveat eval).

  * Purpose: sustained throughput under heavy load.
  * Best: **~1.01–1.06 ms** / 64 (≈16–17 µs/token).
  * With `fast-cbor` (ciborium): **regressed** (~1.38–1.41 ms) → keep it off.

**`critcmp` snapshot (your latest):**

```
group                   auth-cbor                      auth-packA                    auth-plain                     auth-simd (best)
verify_batch_64_loop    1.29   320.0±10.19µs          1.27   314.7±9.48µs           1.00   248.3±6.60µs             1.00   247.7±17.97µs
verify_many_64          1.31   314.7±10.37µs          1.31   315.0±9.21µs           1.00   241.0±6.84µs             1.03   247.2±8.85µs
verify_many_64_heavy    1.39  1410.9±46.08µs          1.36  1387.9±40.89µs          1.04  1059.5±27.07µs            1.00  1018.4±28.88µs
verify_single           1.32      5.0±0.16µs          1.30      4.9±0.14µs           1.02      3.9±0.16µs            1.00      3.8±0.13µs
verify_single_heavy     1.38     22.6±0.83µs          1.35     22.1±0.77µs           1.05     17.2±0.52µs            1.00     16.3±0.47µs
```

**Interpretation:**

* `simd-b64` is the current champ, especially on heavy; `fast-cbor` is slower across the board (disable).
* Batch path (`verify_many_64`) is slightly better than the loop — amortization works.
* We are **not** parallel yet; multi-core scaling is the next big lever.

---

## 3) Immediate next steps to hit “God-tier” on this box

### A) Turn on **parallel** batches (order-preserving) — small code tweak

**Why:** `verify_many_*` for N≥64 is throughput-bound and trivially parallel. Expect ~2–3× on a 4-thread laptop for heavy tokens.

**What to change (paste-ready deltas):**

1. Ensure feature:

```toml
# crates/ron-auth/Cargo.toml
[features]
parallel = ["rayon"]
```

2. Add `Sync` bound and Rayon path in `verify/pipeline.rs` (keep scalar fallback when `parallel` is off):

```rust
// signature change to satisfy Rayon sharing:
pub fn verify_many_into<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError> {
    out.clear();
    out.reserve(tokens_b64url.len());

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        let decisions: Result<Vec<Decision>, AuthError> = tokens_b64url
            .par_iter() // IndexedParallelIterator → order preserved on collect()
            .map(|tok| {
                let mut scratch = Vec::with_capacity(1024);
                verify_one_with_buf(cfg, tok, ctx, keys, &mut scratch)
            })
            .collect();
        out.extend(decisions?);
        return Ok(());
    }

    // Non-parallel fallback (unchanged)
    let mut scratch = Vec::with_capacity(1024);
    let mut reasons = smallvec::SmallVec::<[DenyReason; 8]>::new();
    for tok in tokens_b64url {
        // ... existing scalar loop (unchanged) ...
        // Note: reasons.clear(); reuse scratch
    }
    Ok(())
}
```

> If the compiler later asks for `RequestCtx: Sync`, add `+ Sync` to its impl type or wrap `extras` in a Sync-friendly form in benches. In practice your current `RequestCtx` passed earlier without issue.

3. Rebuild with:

```
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par
```

**Expected:** `verify_many_64_heavy` should drop from ~1.02–1.06 ms → **~0.35–0.55 ms** on this laptop (2–3×), with order preserved.

---

### B) Bench profile + CPU tuning (free wins)

At the **workspace root** `Cargo.toml` (not per-crate; your warnings showed per-crate profiles were ignored):

```toml
[profile.bench]
opt-level = 3
lto = "thin"
codegen-units = 1
debug = false
strip = "debuginfo"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

Then run benches with CPU-specific codegen:

```
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "simd-b64 parallel" --bench verify_bench -- --save-baseline=auth-native
```

---

### C) Micro-tuning (low risk)

* **`soa_threshold`:** Default 8 works; try 12–16. On your ~30-caveat heavy set, streaming was still winning; a slightly higher threshold may improve mixed batches.
* **Scratch pre-reserve:** bump to 2048 bytes for token decode scratch to avoid intermediate growth on heavier tokens:

  ```rust
  let mut scratch = Vec::with_capacity(2048);
  ```
* **MAC init cache (optional, feature-gated):** tiny LRU keyed by `(tid,kid,scope)` → cached `tag0`. Helps repetitive issuer patterns. Keep bounded; no correctness impact.

---

## 4) “Pack B” (deeper work, do next instance)

* **Zero-copy CBOR via `minicbor`** with borrowed types (`Capability<'a>`, `Caveat<'a>`, `Scope<'a>`).
  *Rationale:* Real wins show up only with **borrowed decoding**, not `ciborium`. Expect +5–15% on decode-heavy paths.
* **Batch MAC / signature** (if/when we add signatures). For MAC (BLAKE3) we already stream; for ed25519 (future), dalek batch verify gives ~2× on large sets.

---

## 5) How to run & compare (canonical commands)

### One-shot baselines

```bash
# Plain (no features)
cargo bench -p ron-auth --bench verify_bench -- --save-baseline=auth-plain

# SIMD base64 only
cargo bench -p ron-auth --features "simd-b64" --bench verify_bench -- --save-baseline=auth-simd

# Parallel + SIMD (after the K: Sync patch)
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par
```

### Compare with `critcmp`

```bash
critcmp auth-plain auth-simd auth-par
# Add more: auth-cbor, auth-packA, auth-native, etc.
```

### Optional criterion knobs (supported by our bench binary)

```bash
cargo bench -p ron-auth --bench verify_bench -- \
  --sample-size=100 --warm-up-time=2 --measurement-time=6 \
  --save-baseline=auth-tuned
```

> If you see “Unrecognized option”, the bench binary you’re running was built from an older target; rebuild the bench (or omit flags and just use `--save-baseline`).

---

## 6) Quality gates & diagnostics to keep us honest

* **CI Bench Gate (manual for now):** fail if `verify_single` > **4.2 µs** or `verify_many_64_heavy` > **1.10 ms** (solo-thread) on the reference box.
* **Flamegraphs:** run `cargo install flamegraph` and capture on `verify_single_heavy` to confirm decode→MAC→eval proportions after changes.
* **Repro flags in logs:** always print active features and `RUSTFLAGS` value at bench start (add a tiny `println!` in the bench main if desired).

---

## 7) Known issues / decisions

* **`fast-cbor` (ciborium) regressed** on this laptop — leave **OFF**. Revisit only with **`minicbor` zero-copy**.
* Parallel batches needed the `K: Sync` bound; we’ll keep `MacKeyProvider` blanket-object safe for read-only access.
* Criterion long options occasionally mis-parsed when reusing old bench binaries — a clean rebuild cures it.

---

## 8) Paste-ready snippets (so future us can pick up instantly)

### A) `verify/pipeline.rs` — add `Sync` & parallel path

```rust
// Change signature:
pub fn verify_many_into<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError> {
    out.clear();
    out.reserve(tokens_b64url.len());

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        let decisions: Result<Vec<Decision>, AuthError> = tokens_b64url
            .par_iter()
            .map(|tok| {
                let mut scratch = Vec::with_capacity(2048);
                verify_one_with_buf(cfg, tok, ctx, keys, &mut scratch)
            })
            .collect();
        out.extend(decisions?);
        return Ok(());
    }

    // ... existing scalar loop (leave as-is) ...
}
```

### B) Workspace `Cargo.toml` — bench profile

```toml
[profile.bench]
opt-level = 3
lto = "thin"
codegen-units = 1
debug = false
strip = "debuginfo"
```

### C) Bench commands (copy/paste)

```bash
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par

critcmp auth-plain auth-simd auth-par
```

---

## 9) Target numbers (next instance goals on this laptop)

* **Single small:** < **3.5 µs** (`simd-b64`, native CPU).
* **Many×64 small:** ≤ **220–230 µs** (scalar), **≤ 90–120 µs** (parallel).
* **Single heavy:** ≤ **14–15 µs**.
* **Many×64 heavy:** **≤ 0.35–0.55 ms** (parallel).

---

## 10) Rollback plan

* Feature-gate everything: `parallel`, `simd-b64`. If regressions appear, re-run `auth-plain` vs `auth-simd` vs `auth-par` and keep the best for prod.
* Keep `fast-cbor` disabled until `minicbor` branch is ready.

---

## 11) Next-instance checklist

1. Apply the **`K: Sync`** patch and enable `parallel` feature.
2. Add **bench profile** at workspace root; re-bench with `target-cpu=native`.
3. Tune **`soa_threshold`** (try 12 or 16) and bump scratch to 2048.
4. Record baselines: `auth-plain`, `auth-simd`, `auth-par`, `auth-native`.
5. If time remains: prototype **`minicbor` zero-copy** in a feature branch (borrowed `Capability<'a>`).


### END NOTE - NOVEMBER 8 2025 - 19:48 CST


### BEGIN NOTE - NOVEMBER 9 2025 - 9:53 CST


FURTHER PERFORMAMCE - DO THIS LATER

Here’s a tight, actionable performance TODO list for `ron-auth` you can park and revisit later. It’s grouped by ROI and comes with “done when” gates + copy-paste commands.

# RON-AUTH — Performance TODO (stash for later)

## A) Free/Little Effort (do first)

1. Workspace profiles at root (stability + tiny wins)

* **Why:** make benches/releases consistently optimized.
* **Do:** add to workspace `Cargo.toml`:

  ```toml
  [profile.release]
  opt-level = 3
  lto = "thin"
  codegen-units = 1

  [profile.bench]
  opt-level = 3
  lto = "thin"
  codegen-units = 1
  debug = false
  strip = "debuginfo"
  ```
* **Done when:** profile warnings disappear; numbers steady run-to-run.

2. Lock current tripwires (CI/notes)

* **Do:** record gates on this box:

  * `verify_many_64` ≤ **132 µs**
  * `verify_many_64_heavy` ≤ **370 µs**
  * `verify_single` ≤ **3.7 µs**
  * `verify_single_heavy` ≤ **16.2 µs**
* **Done when:** regressions are obvious via `critcmp`.

3. Heuristic polish (we already applied most)

* **Do later (quick sweeps):**

  * Try `PAR_MIN_BATCH` in `{48, 64, 80}`, keep best.
  * Try `soa_threshold` in `{16, 24, 32}`, keep best.
* **Run:**

  ```
  RAYON_NUM_THREADS=4 RUSTFLAGS="-C target-cpu=native" \
  cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-sweep
  critcmp auth-par-tuned auth-sweep
  ```
* **Done when:** best set chosen and documented in NOTES.MD.

## B) Micro-alloc + cache hygiene (low effort, 2–6%)

4. Thread-local scratch buffers (parallel path)

* **Why:** avoid per-job realloc in Rayon workers.
* **Do:** behind `feature = "tls-scratch"`, make a `thread_local! { static SCRATCH: RefCell<Vec<u8>> }` used in `verify_one_with_buf_thresh`.
* **Done when:** heavy batched allocs drop in flamegraph/pprof; `verify_many_64_heavy` improves by ~2–4%.

5. SmallVec sizing in hot paths

* **Do:** switch `Vec<DenyReason>` → `SmallVec<[DenyReason; 8]>`; sample medians suggest 8 is safe.
* **Done when:** tiny improvement (≤2%); zero extra allocs on common cases.

## C) Decode/MAC polish (1–5%)

6. Base64 SIMD “decode into” / buffer reuse

* **Do:** keep the SIMD-returned `Vec` as persistent scratch per worker; avoid capacity churn.
* **Done when:** small + heavy batches gain ~1–3%.

7. CBOR fragment buffer reuse in MAC

* **Do:** reuse a `Vec<u8>` for CBOR fragments inside MAC chaining (already mostly done; audit).
* **Done when:** minor (<2%) but measurable improvement on heavy.

## D) Medium Effort (5–20% on decode-heavy)

8. **Zero-copy CBOR** feature branch (`minicbor`)

* **Why:** reduce allocations/copies in parse.
* **Do:** add `fast-cbor = ["minicbor"]` (repurpose flag) and implement an internal borrowed decode → convert to owned DTOs at the API boundary.
* **Run:**

  ```
  cargo bench -p ron-auth --features "parallel simd-b64 fast-cbor" --bench verify_bench -- --save-baseline=auth-minicbor
  critcmp auth-par-tuned auth-minicbor
  ```
* **Done when:** heavy/parse-bound benches are **5–15%** faster with the feature on.

## E) Advanced (situational, 3–10%)

9. PGO (Profile-Guided Optimization)

* **Do:** generate train profile from benches, rebuild with PGO for release binaries (docs in NOTES.MD).
* **Done when:** **3–8%** uplift on real binaries (library benches may show smaller gains).

10. Optional: dynamic runtime knobs via env

* **Do:** let `PAR_MIN_BATCH` and `soa_threshold` be overridden via env for ops tuning without rebuilds.
* **Done when:** environment toggles reflected in logs; perf parity with compiled-in defaults.

## F) Validation & Tooling

11. Benchmark macros

* **Do:** add `scripts/bench_auth.sh` that runs the 3 canonical baselines and prints a tiny table with current tripwires.
* **Run:**

  ```
  RUSTFLAGS="-C target-cpu=native" scripts/bench_auth.sh
  ```
* **Done when:** one-command reproducible runs are standard.

12. (Optional later) Flamegraphs/pprof capture

* **Do:** when tooling is available, capture `verify_many_64_heavy` (scalar vs parallel) to verify time splits (decode/MAC/eval).
* **Done when:** no unexpected hotspots; allocs low.

---

### Keep these handy (canonical runs)

```
# Scalar (SIMD only)
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "simd-b64" --bench verify_bench -- --save-baseline=auth-simd

# Parallel + SIMD
RAYON_NUM_THREADS=4 RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par-tuned

# Compare
critcmp auth-simd auth-par-tuned
```

That’s the list. We can mark the current state as **God-tier (v1)** and return to A→F as needed when we want to squeeze the next 5–15%.



### END NOTE - NOVEMBER 9 2025 - 9:53 CST




### BEGIN NOTE - NOVEMBER 9 2025 - 10:00 CST


# CARRY-OVER NOTES — `ron-auth` Beta Push (as of 2025-11-09)

## 0) TL;DR

* **State:** Build + tests green. SIMD Base64 path on; **parallel batch verify** landed; **adaptive crossover** tuned; constant-time MAC compare in place.
* **Perf on reference laptop (2019 MBP i5, `-C target-cpu=native`, Rayon=4):**

  * `verify_single` ~**3.6 µs**
  * `verify_many_64` ~**129 µs**
  * `verify_single_heavy` ~**16.0 µs**
  * `verify_many_64_heavy` ~**360 µs**
* **API:** `verify_token`, `verify_many`, `verify_many_into`; builder + sign helpers for benches/dev.
* **Next:** Ship Beta: docs, tests (unit/prop/fuzz), CI gates, examples, stability pass, and interop notes for svc-passport/ron-kms.

---

## 1) What we shipped (this instance)

### Functional / Pipeline

* Hybrid verification pipeline:

  * **Streaming** path for small caveat sets.
  * **SoA** (columnar) path for large caveat sets.
  * **Auto crossover** via `VerifierConfig.soa_threshold` (default 24, tuned for our fixtures).
* Batch APIs:

  * `verify_many` (returns `Vec<Decision>`)
  * `verify_many_into` (writes into caller-provided `Vec`), **order-preserving**, **parallelized** when `feature=parallel` and `len ≥ 64`.

### Performance

* **SIMD Base64** via `base64-simd` behind `feature=simd-b64`.
* **Parallel batch** via Rayon behind `feature=parallel` (added `Sync` bound to `MacKeyProvider` usage).
* **CT MAC compare**: pure constant-time loop (no early exit, no alloc).
* **Scratch reuse** + modest pre-reserve sizes (1–2 KiB) in hot paths.

### Build & Hygiene

* Features organized and documented:

  * `simd-b64` ✅
  * `parallel` ✅
  * `bench-eval-modes` (exposes streaming/SoA only helpers for benchmarking) ✅
  * `fast-cbor` kept but **disabled by default** (ciborium regressed on this box).
* Bench harness (`criterion`) integrated; `critcmp` flows validated.

---

## 2) Public surface (stable façade)

```rust
// crates/ron-auth/src/lib.rs
pub fn verify_token<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &K,
) -> Result<Decision, AuthError>;

pub fn verify_many<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
) -> Result<Vec<Decision>, AuthError>;

pub fn verify_many_into<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError>;
```

**Types (stable DTOs):** `Capability`, `Scope`, `Caveat`, `Decision`, `VerifierConfig`, `RequestCtx`.
**Errors:** `AuthError` (malformed/bounds/unknown_kid/mac_mismatch/expired/nbf/policy_deny), `DenyReason` (time, shape, tenant, bytes, rate, custom).

**Key lookup:**

```rust
pub trait MacKeyProvider {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey>;
}
pub struct MacKey(pub [u8; 32]); // BLAKE3 keyed MAC
```

---

## 3) Feature flags (how to use)

* `simd-b64`: enable SIMD base64 decoding (recommended).
* `parallel`: enable Rayon parallel path for large batches (order preserved).
* `bench-eval-modes`: exposes streaming-only / soa-only helpers for micro-bench.
* `fast-cbor`: maps to `ciborium` currently; **OFF** for prod (slower on our box). Reserved for future `minicbor` swap.

---

## 4) Pipelines & invariants

* **Decode:** Base64URL (no pad) → CBOR → `Capability`.
* **Bounds:** reject if `len(b64)` exceeds `max_b64url_chars_for(max_token_bytes)`; enforce `cfg.max_token_bytes` and `cfg.max_caveats`.
* **MAC:** compute BLAKE3 keyed over deterministic CBOR fragments (scope+caveats) and **constant-time compare**.
* **Eval:** choose streaming vs SoA; write deny reasons; `Allow{scope}` iff empty reasons.
* **No I/O:** keys provided by `MacKeyProvider`; no network/disk.

---

## 5) How to run (tests, benches, comparisons)

### Tests

```
cargo test -p ron-auth
```

### Benches (canonical baselines)

```
# Scalar (SIMD only)
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "simd-b64" --bench verify_bench -- --save-baseline=auth-simd

# Parallel + SIMD
RAYON_NUM_THREADS=4 RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par

# Tuned (skip sampling for big parallel batches)
RAYON_NUM_THREADS=4 RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par-tuned

# Compare
critcmp auth-simd auth-par auth-par-tuned
```

**Tripwires (reference laptop):**

* `verify_many_64` ≤ **132 µs**
* `verify_many_64_heavy` ≤ **370 µs**
* `verify_single` ≤ **3.7 µs**
* `verify_single_heavy` ≤ **16.2 µs**

> Note: Move `[profile.release]/[profile.bench]` to **workspace root** `Cargo.toml` to silence “profiles ignored” warnings and keep codegen consistent.

---

## 6) Interop (svc-passport / ron-kms)

* **Key source:** In services, implement `MacKeyProvider` via **ron-kms** (read-only accessor returning `[u8;32]` MAC keys by `(tid,kid)`).
* **svc-passport flow:** svc-passport issues tokens (CBOR→b64url) and uses **ron-kms** for signing MACs. ron-auth verifies tokens at ingress (svc-gateway/omnigate) or service boundary, using cached verifying keys from kms (rotate-safe).
* **Rotation:** Keep old verifying keys available (no-break historical validation). `kid` in token maps to the specific key version.

---

## 7) Security / correctness invariants

* Deterministic CBOR encoding (stable MAC domain).
* No ambient authority: caller supplies `RequestCtx` + keys; no globals.
* Constant-time MAC compare; no length leaks via early exit.
* Strict bounds: token bytes, caveat count, path/method/aud filters; deny on malformed.
* **No panics** on user input; errors are typed.

---

## 8) What’s left to ship **Beta** (must-do)

**A) Documentation & examples**

* `README.md`: tighten quickstart (how to verify, example `MacKeyProvider`, feature flags, perf notes).
* Add `examples/verify.rs`:

  * Build `VerifierConfig::with_defaults()`
  * Implement tiny `MacKeyProvider`
  * Verify one token and a batch; print decisions.

**B) Tests**

* **Unit tests** for:

  * Bounds (oversized base64; oversized decoded; too many caveats)
  * Time caveats (exp/nbf with skew)
  * Method/path/aud/tenant
  * Bytes limit (`BytesLe`)
  * MAC mismatch (tamper any field)
* **Property tests** (proptest):

  * Attenuation monotonicity (adding caveats cannot widen privilege)
  * Random caveat permutations preserve deny/allow for commutative subsets (where applicable)
* **Fuzz seeds** (optional for Beta): malformed base64, truncated CBOR, unknown tags.

**C) CI & gates**

* Add GitHub Actions:

  * `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
  * Optional: benchmark smoke (one short run) behind a label
* Document perf tripwires in NOTES.MD (manual check acceptable for Beta)

**D) Stability pass**

* Confirm `pub` API: keep façade minimal (`verify_*`, DTOs, errors, traits). No breaking changes between Beta and 1.0 without semver guidelines.
* Ensure `serde` field names/tags are finalized (no rename churn post-Beta).

**E) Metrics hooks (lightweight)**

* Provide optional counters (feature-gated) to integrate with `ron-metrics` later:

  * `auth_verifies_total{result=allow|deny}`
  * `auth_denies_total{reason}`
  * `auth_verify_latency_seconds` (Histogram)

> For Beta, this can be a no-op trait in `ron-auth` with a default unit impl; real exporter lives in services.

**F) Examples & fixtures**

* Add `benches/fixtures.rs` (already implied by bench) and keep deterministic seeds.
* Include a tiny **golden vector** (token + expected decision) for regression testing.

**G) Crate polish**

* `Cargo.toml`: categories/keywords set, repository/readme present, license files OK.
* Ensure `#![forbid(unsafe_code)]` at crate root (if not already).
* Audit `fast-cbor` feature: mark **experimental** in docs with a note to prefer default.

---

## 9) Nice-to-have (post-Beta / later)

* **Zero-copy CBOR** (`minicbor`) feature branch with borrowed types, converting to owned at API boundary. Target +5–15% on parse heavy.
* **Thread-local scratch** buffers for parallel path (feature-gated).
* **Env-tunable knobs** (`PAR_MIN_BATCH`, `soa_threshold`) for ops.
* **PGO** for release binaries in services.
* **pprof/flamegraph** captures (when CLT available) to validate hotspots.

---

## 10) Known decisions / tradeoffs

* `fast-cbor` (ciborium) showed regressions on this laptop → **disabled** by default; we’ll revisit with `minicbor`.
* Parallel path requires `K: Sync`. Our `MacKeyProvider` contract remains read-only; callers wrap interior mutability if needed.
* We bias to **streaming** for heavy caveat sets; SoA reserved for very large sets.

---

## 11) Commands (copy-paste)

**Build & test**

```
cargo fmt -p ron-auth
cargo clippy -p ron-auth --no-deps -- -D warnings
cargo test  -p ron-auth
```

**Benches (reference)**

```
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "simd-b64" --bench verify_bench -- --save-baseline=auth-simd

RAYON_NUM_THREADS=4 RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par

critcmp auth-simd auth-par
```

---

## 12) Next-instance checklist (to close Beta fast)

1. Add **examples/verify.rs** and refresh `README.md` with end-to-end usage.
2. Land **unit + property tests** for caveats, bounds, and MAC mismatch.
3. Wire a minimal **CI** (fmt/clippy/tests) for the crate.
4. Freeze **public API** docs and derive comments; ensure semver notes in README.
5. Tag **tripwires** in NOTES.MD and record current critcmp snapshot.
6. (Optional same-day) Add no-op **metrics trait** so services can hook counters later.

That’s everything we need to pick up next session and ship **ron-auth Beta** swiftly.



### END NOTE - NOVEMBER 9 2025 - 10:00 CST




### BEGIN NOTE - NOVEMBER 9 2025 - 12:39 CST


# CARRY-OVER NOTES — **ron-auth** Beta (Hybrid Verify, SIMD B64, Optional Parallel)

## 0) TL;DR (Where we are)

* ✅ **Beta is green**: all unit/integration tests pass, including order-preserving batch verify and error-path tests.
* ✅ **Hybrid pipeline**: auto-chooses streaming eval for small caveat sets; SoA (columnar) for larger sets; adaptive threshold sampling per batch.
* ✅ **Performance knobs**: `simd-b64` (wins on decode-heavy tokens), `parallel` (Rayon sharding for large batches) — both compile cleanly together.
* 🔒 **Safety gates**: bounds on token size and caveat count; constant-time MAC compare; deterministic CBOR + Base64URL; pure/sync, no I/O.
* 🧩 **Interop-ready**: simple `MacKeyProvider` (tid,kid→[u8;32]) integrates with `svc-passport` (issuing) and future `ron-kms` (key custody).

---

## 1) What’s shipped (Beta)

### Functional/API

* **Public API**:
  `verify_token`, `verify_many`, `verify_many_into`.
  Feature-gated helpers for A/B benches: `verify_*_streaming_only`, `verify_*_soa_only` (`bench-eval-modes`).
* **Caveats supported**: `Aud`, `Tenant`, `Method`, `PathPrefix`, `IpCidr`, `BytesLe`, `Exp`, `Amnesia`, `Custom{name,ns,cbor}`.
* **Decisions**: `Allow{scope}` / `Deny{reasons}` with stable `DenyReason` set.
* **Errors**: `Malformed(&'static str)`, `Bounds`, `UnknownKid`, `MacMismatch`, `Expired`.

### Pipeline/Perf

* **Hybrid evaluator**: streaming for ≤ `soa_threshold` (cfg), SoA otherwise.
* **Adaptive threshold**: median caveat-count sampling from the first few tokens in a batch; heavy sets bias to streaming (on this laptop).
* **Batch**: `verify_many_into` amortizes scratch buffers; **order preserving**; optional parallel sharding when `parallel` feature enabled and batch ≥64.
* **MAC**: BLAKE3 MAC over canonical caveat order; compare with constant-time equality.
* **Base64URL**: stock or `base64-simd` when `simd-b64` enabled.
* **Bounds**: `max_token_bytes`, `max_caveats`, `clock_skew_secs` enforced before heavy work.

### Metrics hook (scaffold)

* `metrics.rs` provides a tiny **no-op trait** (hook points invoked from the pipeline).
  It’s intentionally inert at Beta; wiring into `ron-metrics`/Prometheus is a post-beta “nice-to-have” (see §5).

---

## 2) Interoperability Map (ron-core crates)

### With `svc-passport` (issuer/verifier service)

* **Key flow**: `svc-passport` holds the **verifying** keys for MAC (32B secret per `(tenant, kid)`) or delegates to `ron-kms` later.

* **Auth path**:

  1. Client sends token (Base64URL CBOR capability) to `svc-passport`.
  2. `svc-passport` implements `MacKeyProvider` → returns `MacKey([u8;32])` for `(tid,kid)`.
  3. `svc-passport` calls `ron-auth::verify_*` with its `VerifierConfig` and a `RequestCtx` derived from the HTTP request.
  4. It maps `Decision::{Allow|Deny}` and `AuthError::*` to HTTP 200/403/4xx.

* **Config to mirror** (recommended service defaults):

  * `max_token_bytes: 4096` (or lower if your tokens are small).
  * `max_caveats: 64` (typical ≤32).
  * `clock_skew_secs: 60`.
  * `soa_threshold: 8..16` (8 is a good Beta default on laptop; re-tune per hardware).

### With `ron-kms` (future custody)

* **Today**: `ron-auth` uses shared MAC keys via `MacKeyProvider`.
* **Later**: `ron-kms` can expose **derive-on-demand** or **fetch** of `[u8;32]` MAC keys keyed by `(tenant,kid)`; rotate by bumping `kid` and keeping old verifying keys around for historical checks (non-breaking rotation).
* **No API change needed** in `ron-auth` to adopt KMS; only `MacKeyProvider` implementation swaps to a KMS client.

### With `svc-gateway` / `micronode`

* `svc-gateway` or `micronode` calls into `svc-passport` (HTTP) or links `ron-auth` directly if embedded.
* `RequestCtx` creation is straightforward: method (uppercased), path, peer IP, tenant, optional audience/policy digest.

---

## 3) Benchmarks (current + projections)

### Hardware & setup

* **Dev laptop**: 2019 **MacBook Pro 13"**, Intel **i5-8257U** (4C/8T, 1.4→3.9 GHz), plugged in, macOS.
* **Build flags**:

  * `RUSTFLAGS="-C target-cpu=native"` for benches.
  * Feature sets below per run (SIMD/Parallel).
* **Criterion** defaults (100 samples) unless noted.

### Latest results (you just ran)

**SIMD only** (`--features "simd-b64"`):

* `verify_single` … **3.60–3.63 µs**
* `verify_batch_64_loop` … **231–232 µs**
* `verify_many_64` … **247–248 µs**
* `verify_single_heavy` … **15.98–16.04 µs**
* `verify_many_64_heavy` … **1.059–1.066 ms** (warning about sample time is fine)

**Parallel+SIMD** (`--features "parallel simd-b64"`, `RAYON_NUM_THREADS=4`):

* `verify_single` … **3.58–3.62 µs** (single doesn’t benefit from parallel)
* `verify_batch_64_loop` … **228–230 µs** (loop, no batch API)
* `verify_many_64` … **~149–157 µs**  **← big win from parallel+hybrid**
* `verify_single_heavy` … **16.8–17.0 µs**
* `verify_many_64_heavy` … **~402–419 µs** **← 2.5–2.7× faster vs SIMD-only**

**critcmp (auth-par vs auth-simd)**:

* `verify_many_64`: **~1.69×** speedup (SIMD-only baseline slower)
* `verify_many_64_heavy`: **~2.65×** speedup
* Singles are ~equal (parallel doesn’t help single-token latency).

### Throughput framing (back-of-envelope)

* `verify_single ≈ 3.6 µs` → **~277k verifications/sec** per core (idealized).
* `verify_many_64 ≈ 150 µs` for 64 → **~426k verifications/sec batch** (amortized) on 4 threads.

### Server projections (educated estimates)

* **Modern desktop (M2 Pro/M3, recent Ryzen)**: expect **1.5–3×** faster single-thread latency; batch ~**2–3×** faster due to higher memory BW and turbo.
* **EPYC 7xx3/9xx4, Xeon Scalable (Ice/Granite)**: with more cores and high L3, expect **2–4×** on latency and **4–8×** on batch throughput at the same batch size; with larger batches (≥256) and `RAYON_NUM_THREADS` tuned to sockets, **10M+ verifications/sec** aggregate is plausible.
* **Arm servers (Graviton3+)**: similar or better batch scaling; SIMD B64 path differs but overall **~2×** vs laptop is reasonable.

> Reality check: true gains depend on **token shape** (caveat count, CIDR/method/path density), cache locality, and key lookup. Our hybrid/SoA keeps evaluator overhead low across shapes.

---

## 4) Recommended operating modes (service)

* **Default features**: `std` only (tiny footprint) — good for base library linking.
* **Service build (recommended)**: enable **`parallel simd-b64`** for `svc-passport`.

  * Tune `RAYON_NUM_THREADS` to physical cores (or leave default = num CPUs).
  * Set `soa_threshold` per host; start at `8` and re-tune with your prod-shaped tokens.
* **Bounds**:

  * If tokens are small/regular, drop `max_token_bytes` (e.g., 2048) and `max_caveats` (e.g., 32) for tighter DoS ceilings.
* **Context fast-path**: ensure HTTP method is uppercased once upstream; pass `peer_ip` only when needed to avoid parse work in unrelated policies.

---

## 5) “Nice-to-have” roadmap (post-Beta)

### A) Performance Enhancements

1. **Zero-copy CBOR (borrowed)** behind `minicbor-zerocopy`

   * Replace owned deserialization on hot fields with borrowed views; **1.2–1.8×** less allocation and copy; expect **10–25%** latency win on single, bigger on batch.
2. **Caveat specialization**

   * **Method**: bit-set or small fixed bitmap for common verbs → **5–10%** on mixed workloads.
   * **PathPrefix**: trie or sorted array + binary search; batch of prefixes folded into a single walk → **10–20%** where path checks dominate.
   * **IpCidr**: preparse to `(addr, mask)` tuples; SIMD-assist mask compare for IPv4 groups → **5–15%** on CIDR-heavy tokens.
3. **More aggressive hybrid tuning**

   * Learn `soa_threshold` online per process (EWMA on caveat counts and outcome cost) → **few %** steady improvement across mixed traffic.
4. **Batch scheduling**

   * Micro-coalescing queue in `svc-passport` to gather 8–64 tokens before verify; preserves order per request stream → **1.2–2×** throughput at tail latencies you can afford.
5. **MAC prehash** (if issuer agrees)

   * Keep canonicalized caveat CBOR bytes and avoid re-encoding; already doing decode only, but pre-arranged framing could trim MAC prep → **5–10%**.
6. **Arena scratch & smallvec tuning**

   * Reuse larger `Vec<u8>` arenas across requests; turn some `Vec`s into `SmallVec` with tuned inline capacities → **5–15%** in allocation-heavy paths.

### B) Observability & Operations

1. **Wire the metrics hook** (`metrics.rs`) to `ron-metrics`/Prometheus:

   * Counters: `auth_allow_total`, `auth_deny_total`, `auth_mac_mismatch_total`, `auth_unknown_kid_total`, `auth_malformed_total`.
   * Histograms: `auth_verify_latency_seconds` (label `{mode="single|batch", eval="stream|soa|auto"}`), `auth_token_bytes`, `auth_caveats`.
   * Gauges: EWMA threshold, parallel batch size.
     *Benefit*: production tuning confidence, SLOs, quick anomaly detection.
2. **Tracing spans** (sampled) around verify paths with outcome tags.
3. **Fuzz/property tests**: expand proptest suites for tricky Unicode paths, path normalization cases, and CIDR edge boundaries.

### C) Product Features (later)

* **Delegation chains / attenuation proof** surfaces (return proof of which caveats matched).
* **Embedded policy digest** cross-check with `ron-policy` (already passing digest today; add integrity rules).
* **Key rotation policy** helpers: kid strategies, grace windows, and stale-kid alerts.

---

## 6) How to run & read benches (quick script)

```bash
# 1) SIMD
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "simd-b64" --bench verify_bench -- --save-baseline=auth-simd

# 2) Parallel + SIMD
RAYON_NUM_THREADS=4 RUSTFLAGS="-C target-cpu=native" \
cargo bench -p ron-auth --features "parallel simd-b64" --bench verify_bench -- --save-baseline=auth-par

# 3) Compare
critcmp auth-simd auth-par
```

Interpretation hints:

* Prefer **`verify_many_*`** for service-like throughput.
* If `verify_many_64_heavy` is >2× faster in `auth-par`, keep `parallel` on.
* If your tokens are tiny (≤6 caveats), tweak `soa_threshold` ↓ and re-bench.

---

## 7) Invariants & guardrails (do not relax)

* **Constant-time MAC compare** must remain.
* **Bounds first, MAC second, caveats last** — preserves DoS posture and avoids attacker-chosen work.
* **Pure/sync**: no I/O during verify; no global state mutation.
* **Determinism**: caveat evaluation order must be domain-fixed; never data-dependent branching that leaks secrets (current eval is fine).

---

## 8) Release notes snippet (Beta)

* **New**: Hybrid verification pipeline with adaptive threshold (streaming vs SoA).
* **New**: Optional parallel batch verify (Rayon), preserves order.
* **New**: SIMD Base64URL decode (`simd-b64`) for decode-heavy tokens.
* **Safety**: strict bounds; constant-time MAC; deterministic CBOR.
* **Interop**: simple `MacKeyProvider` for `(tenant,kid)` → 32B key; drop-in for `svc-passport`; KMS-ready.

---

## 9) Quick acceptance checklist (ship it)

* [x] All tests green (incl. `verify_batch_order`).
* [x] Builds with feature combos: **simd-b64**, **parallel**, **parallel+simd-b64**, **bench-eval-modes**, **fast-cbor**.
* [x] Benches saved & compared (`auth-simd` vs `auth-par`).
* [x] `svc-passport` compiles with current `ron-auth` API (done earlier).
* [ ] (Optional) Pin `soa_threshold=8` in `svc-passport` config; note TODO to re-tune in prod.

---

## 10) What to do next (when we resume)

* Wire **metrics hook** to Prometheus counters/histogram and expose them in `svc-passport`.
* Start **minicbor-zerocopy** branch; port a few hot fields to borrowed decode; re-bench.
* Add **path trie** & **method bit-set**; re-bench “heavy” profile.
* Add **micro-coalescer** in `svc-passport` for batch verify; hold to ≤2ms to keep P99 tight.
* Document stable DTOs and error mapping table (for external adopters).

---

**Bottom line**: ron-auth Beta is **fast**, **safe**, and **service-ready**. Parallel+SIMD on your laptop already shows **~1.7×–2.7×** wins for real workloads; modern servers should scale that substantially. The roadmap items above can conservatively net another **~20–50%** latency win and **2×+** aggregate throughput with batching/coalescing and zero-copy paths — without changing the public API.



### END NOTE - NOVEMBER 9 2025 - 12:39 CST