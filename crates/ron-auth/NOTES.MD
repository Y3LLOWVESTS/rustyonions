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