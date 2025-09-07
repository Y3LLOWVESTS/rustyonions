# RustyOnions — Status & Next-Step Plan
_Date: 2025-09-07 · Timezone: America/Chicago_

## HIGH PRIORITY: Encountered a race condition so built a blueprint for Concurrency & Aliasing to prevent any potential future issues(docs/Concurrency_And_Aliasing_Blueprint.md) - Currently reviewing previous code to meet these new requirements.##

- **HIGH PRIOTITY REVIEW AND TESTS: After reviewing the code so it passes all the Concurrency and Aliasing Blueprint tests we will proceed with rest of DailyTodo list.**

## TL;DR
- **Microkernel (ron-kernel): _complete for M0_**, ~95% toward the full blueprint bar (remaining: rigorous validation gates in CI like loom/fuzz/TLA+/SBOM/signing).
- **Project completion (overall): _~50%_** (weighted estimate across subsystems; see breakdown).
- **Highest impact next push:** Finish **M1 Bronze** (Gateway quotas/readyz, OAP stub, golden metrics, Mailbox ops, red-team suite) **and** land **Storage/DHT read-path + SDK DX**. That single sprint moves us to **~68–70%** complete.

---

## 1) Weighted Completion Estimate (project-wide)

| Area | Weight | Status | Completion |
|---|---:|---|---:|
| Microkernel | 10% | API frozen; bus tests (basic/topic/load) green; metrics/health endpoints in place | **95%** |
| Gateway/Omnigate | 15% | gwsmoke E2E success; need quotas/readiness, error taxonomy | **60%** |
| Overlay + Index + Storage (read path) | 20% | Alpha path exercised in gwsmoke; streaming & DHT wiring partial | **45%** |
| Mailbox Service | 10% | SEND/RECV/ACK + idempotency; ops (DELETE/SUBSCRIBE) pending | **55%** |
| SDKs (Rust first) | 10% | Demos; env & retry polish pending | **40%** |
| Security (caps/rotation/amnesia) | 10% | Plan in blueprints; partial stubs | **35%** |
| CI/CD & Validation (fuzz/loom/TLA+/chaos) | 10% | Unit/integration tests in; formal/chaos pending | **30%** |
| Observability & SLOs | 5% | Metrics basic; dashboards/SLOs/runbooks partial | **50%** |
| Docs, Runbooks, Governance | 5% | Blueprints done; OAP stub & runbooks missing | **45%** |
| Scaling & DHT/Erasure/Placement | 5% | Planned | **25%** |

**Point estimate:** **~49.5% ⇒ ~50%**.

### Why the weights?
They mirror the blueprints’ emphasis: data plane services + SDK + security/validation dominate value delivery, with the kernel intentionally tiny (10%).

---

## 2) What moved recently
- **gwsmoke** proved Gateway→Index→Overlay→Storage **HTTP 200** manifest fetch locally.
- **Kernel tests** (`bus_basic`, `bus_topic`, `bus_load`, `event_snapshot`, `http_index_overlay`) are green.
- **TODO.md** updated to reflect gwsmoke success and add `docs/TESTS.md` how-to.

**Known gotchas to keep in mind (carry-over):**
- Fixed script arg parsing: use `readarray -d '' -t` not `read -d '' -a` (the latter dropped args).
- 404 symptom: ensure the same **sled DB path** (`RON_INDEX_DB`) is used by pack/index/services/gateway; otherwise resolves may miss.
- Keep MAX_FRAME semantics straight: **OAP/1 `max_frame = 1 MiB`**; **storage streaming chunk = 64 KiB** (different layers).

---

## 3) Most High-Impact Next Work (one sprint)

### Sprint theme: **“Bronze + Read-Path + DX”**
This closes many TODO checkboxes quickly and unlocks Vest readiness.

#### A. Finish **M1 Bronze**
1) **Gateway quotas & readiness**
   - Per-tenant token buckets; `/readyz` reflects capacity.
   - Return **429/503** with **Retry-After**; label metrics.
   - _DoD_: soak test shows throttling; metrics `quota_exhaustions_total` increments.

2) **Error taxonomy + JSON envelope**
   - Canonical `{code,message,retryable,corr_id}`; map 400/404/413/429/503.
   - _DoD_: golden tests; SDK surfaces friendly errors.

3) **Spec stub: `/docs/specs/OAP-1.md`**
   - Mirror GMI-1.6; include two hex vectors; call out `max_frame = 1 MiB` explicitly.
   - _DoD_: CI links this doc; greps enforce no drift.

4) **Mailbox ops**
   - Implement `DELETE` and `SUBSCRIBE`; tune visibility.
   - _DoD_: demo shows end-to-end behavior with retries and visibility timeout.

5) **Golden metrics & Red-team suite**
   - Required counters/histograms (requests, bytes in/out, rejected{reason}, latency, inflight, bus overflow).
   - Red-team: malformed frames, slow-loris, partial writes, compression bombs.
   - _DoD_: red-team passes; metrics present and graphed.

#### B. Land **Storage/DHT read-path** and **SDK DX**
1) **Storage/DHT read path**
   - Implement `GET`, `HAS`, streaming **64 KiB**; verify BLAKE3 on read.
   - Minimal **tileserver** example for clarity.
   - _DoD_: gateway demos stream tiles; read latency histograms present.

2) **SDK polish**
   - `RON_NODE_URL`, `RON_CAP`; respect **Retry-After**; bounded jitter backoff; propagate `corr_id`.
   - _DoD_: quota demo + transient failure tests green.

**Impact math:** The above raises Gateway, Mailbox, Docs, Observability, CI a bit—and especially **Overlay/Index/Storage** and **SDK**. New overall estimate afterward: **~68–70%**.

---

## 4) Concrete Acceptance Checklist

- [x] Gateway: token buckets + `/readyz` gate; 429/503 + Retry-After present.
- [ ] Error envelope + taxonomy; SDK parses & retries appropriately.
- [ ] OAP/1 spec stub exists; CI greps pass (no 64 KiB-as-max-frame).
- [ ] Mailbox: DELETE + SUBSCRIBE; visibility timeout; idempotency preserved.
- [ ] Golden metrics: requests/bytes/latency/inflight/rejected/overflow/quota.
- [ ] Red-team suite passes.
- [ ] Storage/DHT: GET/HAS; 64 KiB streaming; BLAKE3 verified on read.
- [ ] Tileserver example streams through Gateway; histograms visible.
- [ ] SDK env + jitter backoff; propagates corr_id; retries respect Retry-After.

---

## 5) Sanity Greps (keep these in CI)
```bash
# Hashing & addressing
rg -n "sha-?256|sha256:" README.md docs/ *.md crates/ -S
rg -n "b3:" -S

# Protocol vs storage chunk confusion
rg -n "max_frame\s*=\s*1\s*Mi?B" -S

# Kernel API freeze (presence of ServiceCrashed{reason} etc.)
rg -n "ServiceCrashed\s*\{\s*service\s*:\s*.*reason\s*:" -S

# Terminology
rg -n "Overlay Plane|Private Message Plane" README.md docs/ *.md -S

# README structure
rg -n "Project Structure" README.md -n
rg -n "ron-kernel" README.md -n
rg -n "svc-omnigate|svc-overlay|svc-index|svc-mailbox" README.md -n
```

---

## 6) Short README line you can paste
**Status:** Microkernel is complete (API frozen; bus tests green; metrics/health endpoints live). Gateway+Overlay path verified by gwsmoke. Focus now: Bronze ring (quotas/readyz, OAP stub, golden metrics, mailbox ops, red-team) and Storage/DHT read-path + SDK DX to unlock Vest readiness.

---

## 7) Owner Map (suggested for this sprint)
- **Core:** OAP stub + error taxonomy (1 person)
- **Gateway:** quotas/readyz + golden metrics + red-team (1–2)
- **Mailbox:** DELETE/SUBSCRIBE + visibility (1)
- **Storage/Index:** GET/HAS streaming + tileserver (1–2)
- **SDK:** env + backoff + corr_id (1)
- **CI/Docs:** greps, dashboards, `docs/TESTS.md` (1)









## CARRY OVER NOTES TO NEXT INSTANCE 13:07 - 9-6-2025 --->

Here’s a tight “carry-over” packet you can drop into a fresh machine (or hand to a new teammate). I split it into: quick-start, what’s working now, gotchas/diagnostics, what’s next (highest impact first), and a project completion estimate.

# Quick-start (bootstrap in a new instance)

## Build & run the local stack

```
# from repo root
cargo build

# keep the stack running on 127.0.0.1:9080
HOLD=1 RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 testing/run_stack.sh
```

## Pack a sample object and fetch it

```
printf 'hello rusty onions\n' > /tmp/payload.bin
RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions \
  target/debug/tldctl pack --tld text --input /tmp/payload.bin \
  --index-db /tmp/ron.index --store-root .onions
# prints: b3:<hex>.text  ← copy it

ADDR=b3:<hex>.text
URL="http://127.0.0.1:9080/o/${ADDR#b3:}/payload.bin"
curl -sS "$URL"
```

## One-shot smoke (HEAD / 304 / precompressed / ranges)

```
RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 testing/http_cache_smoke.sh
```

Expected highlights:

* `etag: "b3:<hex>"` (exactly one pair of quotes)
* `cache-control: public, max-age=31536000, immutable` for payloads
* `vary: Accept-Encoding`
* `content-encoding: br` and `content-encoding: zstd` when requested
* `HTTP/1.1 206 Partial Content` with correct `Content-Range`

# What’s working now (MVP scope)

## Gateway

* ✅ Read path for content-addressed objects: `GET /o/<hex>.tld/<rel>` (also accepts `b3:<hex>.tld`).
* ✅ ETags: canonical `"b3:<hex>"` (fixed double-quote bug).
* ✅ Caching: proper `Cache-Control` (payloads: long/immutable; manifests: short), `Vary: Accept-Encoding`.
* ✅ Precompressed selection: `.br` and `.zst` chosen via `Accept-Encoding`.
* ✅ Range requests: single range `bytes=start-end` → `206` with correct `Content-Range`; unsatisfiable → `416`.
* ✅ HEAD mirrors GET headers with `Content-Length` and no body.
* ✅ Quotas: per-tenant token bucket (tenant derived from `X-RON-CAP` or `X-API-Key` or `X-Tenant`; otherwise `public`), returns `429` with `Retry-After` when exhausted.
* ✅ Health endpoints:

  * `/healthz` → `200 ok`
  * `/readyz` → checks configured Unix sockets (overlay required; index/storage optional) and returns JSON report.
* ✅ Code refactor: `crates/gateway/src/routes/` split into 5 modules:

  * `mod.rs` (wiring), `object.rs` (objects route), `readyz.rs` (health), `errors.rs` (JSON envelopes), `http_util.rs` (ETag/CT/range helpers).

## Services plane (local)

* ✅ `svc-index`, `svc-storage`, `svc-overlay` — local single-node flow exercised by smoke scripts.
* ✅ `tldctl pack` creates bundles with `Manifest.toml`, `payload.bin`, plus `.br` and `.zst` encodings.

## Protocol & addressing

* ✅ OAP/1 crate exists with `max_frame = 1 MiB` (spec alignment) and DATA helpers.
* ✅ Repository is BLAKE3-only (`b3:<hex>`). Guards/tests exist to avoid SHA-256 regressions.

## Red-team checks we ran

* ✅ OAP server (on earlier run @ :9444) dropped invalid frame and slow-loris without panicking.

# Gotchas & diagnostics (things that bit us before)

* **Sled DB lock (index):** if `svc-index` is running, `tldctl pack` trying to open the DB directly will fail with a lock:
  *“could not acquire lock on /tmp/ron.index/db (WouldBlock)”*.
  ✅ Our scripts **pack first, then start services** to avoid this.
  Manual check:

  ```
  lsof +D /tmp/ron.index
  pgrep -fl svc-index
  ```
* **Precompressed headers looked “missing”:** header names are lowercase (hyper). Use case-insensitive grep: `rg -i` or `grep -Ei`.
* **ETag formatting:** now stable as `"b3:<hex>"` (the earlier doubled quoting is fixed).
* **Unix sockets reachability:** `/readyz` surfaces socket paths and booleans; you can also probe manually with `lsof -U | rg svc-(index|overlay|storage)\.sock`.

# What’s next (highest impact first)

1. ## Integration tests in Rust for the gateway read path (lock in behavior)

   **Why high impact:** turns today’s manual/shell checks into deterministic CI gates; protects ETag, cache, range, and precompressed logic from regressions as features evolve.
   **What:** Add a test suite that spins the local services (or a thin in-process stub for overlay), then asserts:

   * HEAD shows stable validators and lengths
   * 304 on `If-None-Match`
   * `.br` / `.zst` negotiated correctly
   * Ranges: satisfiable `206`, unsatisfiable `416`
   * JSON error envelope shape for 404/429/503
     **Tip:** Reuse the packing routine from `tldctl` or pre-bake fixtures in a tmp dir. Mark these as `#[tokio::test]` and isolate state with unique temp dirs.

2. ## Gateway `/metrics` (Prometheus) + minimal SLO counters

   **Why:** immediate observability; enables rate/latency/error tracking and makes `/readyz` more actionable.
   **What counters:** `requests_total{code}`, `bytes_out_total`, `cache_hits_total` (If-None-Match short-circuit), `range_requests_total`, `precompressed_served_total{encoding}`, `quota_rejections_total`.
   **Where:** small module under `gateway/src/metrics.rs`; export at `/metrics`.

3. ## Quota config & per-tenant policy source

   **Why:** make rate limits non-hardcoded and production-real.
   **What:** allow reading a small TOML/JSON (`RON_QUOTA_PATH`) defining `{ tenant -> rps, burst }`; default fallback to `public`. Optionally add IP-fallback (peer addr) when no tenant hint.

4. ## Harden index access pattern for pack workflows

   **Why:** avoids sled lock surprises as teammates adopt the tool.
   **What:** either (a) change `tldctl` to talk to `svc-index` over UDS instead of opening the DB, or (b) add a `--use-daemon` flag that does so when the daemon is detected.

5. ## CI “invariants” (we skipped earlier)

   **Why:** turn manual repo sweeps into automated gates.
   **What:** add `testing/ci_invariants.sh` (grep fail on `sha-?256`, ensure `max_frame = 1 MiB` present in specs, etc.), and hook into CI. We already have the `no_sha256_guard.rs` test; this just makes it comprehensive.

6. ## Docs touch-ups (fast win)

   **Why:** very visible progress bar for stakeholders; low effort.
   **What:** confirm all diagrams/text explicitly say **BLAKE3 (`b3:<hex>`)** and OAP/1 `max_frame = 1 MiB`; add a short “Quick Start” using our two scripts.

7. ## Nice-to-have but not blocking MVP

   * Basic auth/token enforcement path (tie `X-API-Key` to a tenant registry).
   * Add gzip (`gzip`) if needed for legacy clients (we already cover `br` and `zstd`).
   * Service packaging (systemd/Docker) and a single `ron-dev up` wrapper.
   * Extend red-team scripts to the HTTP side (slow-loris on gateway).

# Estimated completion (whole project)

* **Gateway (read path)**: \~85% (MVP complete; metrics/config/tests pending)
* **Local services (index/storage/overlay)**: \~70% (functional single-node; daemonized pack path + tests pending)
* **OAP/1 protocol & SDK**: \~60% (codec/tests exist; broader SDK coverage and end-to-end demos pending)
* **Scaling plane (DHT/provider records, placement, replication)**: \~15% (design present; implementation not started)
* **Payments & policy**: \~25% (manifest guard exists; real pay plumbing TBD)
* **CI + docs + packaging**: \~40% (guards/tests partially there; CI invariants & packaging missing)

**Overall weighted estimate:** **\~45% complete** (±5%).
If we land items 1–5 above, we’d push into the **55–60%** range quickly, with a much sturdier core.

---

If you want, I can turn item (1) into a ready-to-run `cargo test` suite that stands up a temp stack per test and asserts the exact headers/ETags/range semantics we validated by shell.

>18:12 update: 1. ## ✅ Integration tests in Rust for the gateway read path (lock in behavior) — DONE

## CARRY OVER NOTES TO NEXT INSTANCE 18:12 - 9-6-2025 --->

awesome—here’s a tight, “drop-in” packet you can paste into a fresh instance (or hand to a new teammate). it captures what’s proven to work, the gotchas we hit, where the bar sits today, and exactly which next steps move the completion meter the most.

# carry-over notes (for the next instance)

## quick start (local bootstrap)

* build the workspace:

  ```
  cargo build
  ```

* keep the stack running on localhost with the helper script:

  ```
  HOLD=1 RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 testing/run_stack.sh
  ```

  (this matches the “Quick-start” block in the docs).&#x20;

* pack an object and fetch it through the Gateway:

  ```
  printf 'hello rusty onions\n' > /tmp/payload.bin
  RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions \
    target/debug/tldctl pack --tld text --input /tmp/payload.bin \
    --index-db /tmp/ron.index --store-root .onions

  # copy the printed address: b3:<hex>.text
  ADDR=b3:<hex>.text
  URL="http://127.0.0.1:9080/o/${ADDR#b3:}/payload.bin"
  curl -sS "$URL"
  ```



* one-shot cache/range smoke:

  ```
  RON_INDEX_DB=/tmp/ron.index OUT_DIR=.onions BIND=127.0.0.1:9080 testing/http_cache_smoke.sh
  ```

  you should see: quoted ETag for `b3:<hex>`, long-lived cache headers, `vary: Accept-Encoding`, `content-encoding: br|zstd` when requested, and correct `206`/`Content-Range`.&#x20;

## what’s working now (MVP scope, proven end-to-end)

* **Gateway ↔ Index ↔ Overlay ↔ Storage read path works locally** (manifest GET returns 200 via `gwsmoke`).&#x20;
* **Gateway read-path tests are green** (HEAD/ETag/304, ranges 206/416, precompressed selection, JSON 404 envelope); those were the exact acceptance points we targeted.&#x20;
* **Health/ready/metrics endpoints exist** across services (foundation for SLOs), and overload paths return 429/503.&#x20;

## known gotchas / diagnostics we already learned

* **Use the same sled DB path** everywhere (`RON_INDEX_DB`) for pack, index, overlay, and gateway; mismatched paths look like phantom 404s.&#x20;
* **Shell array parsing:** when reading NUL-separated args in bash, prefer `readarray -d '' -t …`; `read -d '' -a …` silently dropped args for us.&#x20;
* **Protocol vs. storage chunking:** OAP/1 `max_frame = 1 MiB` vs. storage streaming chunk `64 KiB`—they’re different layers. Keep the wording crisp in code/docs/tests.&#x20;

## where we stand (today’s completion)

The latest internal roll-up pegs the project around **\~50% complete overall** (weighted across subsystems), with Microkernel essentially at M0 and Gateway/Overlay path proven by `gwsmoke`. &#x20;

Breakdown snapshot (weights + completion by area) is captured here for reference: Microkernel 95%, Gateway/Omnigate 60%, Overlay+Index+Storage 45%, Mailbox 55%, SDK 40%, Security 35%, CI/Validation 30%, Observability 50%, Docs 45%, Scaling 25%.&#x20;

## highest-impact next steps (one sprint that moves the bar most)

Shortlist pulled from the daily plan + Omnigate blueprint, optimized for “% complete” lift:

1. **Golden metrics + `/metrics` everywhere**
   Ship the canonical counters: `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `cache_hits_total`, `range_requests_total`, `precompressed_served_total{encoding}`, `quota_rejections_total`—already specified for Gateway and intended for other services.&#x20;
   *Why it matters:* unlocks SLOs, makes readiness actionable.&#x20;

2. **Quota config + per-tenant policy source**
   Move hardcoded limits to a small TOML/JSON (`RON_QUOTA_PATH`) with `{tenant -> rps, burst}` and sensible fallback; add IP-fallback when unauthenticated.&#x20;
   *Blueprint tie-in:* M1-O01 quotas + capacity `/readyz` gate + `Retry-After`.&#x20;

3. **Harden pack/index access**
   Avoid sled lock surprises: have `tldctl` talk to `svc-index` via UDS or a `--use-daemon` mode when available.&#x20;

4. **CI invariants**
   Add the grep gates (hashing terminology, `max_frame = 1 MiB`, README structure) and wire them into CI so drift gets caught automatically.&#x20;

5. **Docs touch-ups** (fast, visible win)
   Ensure all diagrams/text use **BLAKE3 `b3:<hex>`** and call out OAP/1 `max_frame = 1 MiB`; add a concise Quick-Start using our two scripts.&#x20;

6. **M2 forward leaners that jump the meter**

   * **Storage/DHT read-path (GET, HAS, 64 KiB streaming) + tileserver example**.&#x20;
   * **SDK DX polish** (env keys, `corr_id` tracing, friendly errors).&#x20;

> impact math: knocking out the “Bronze” line (quotas/readyz, golden metrics, OAP stub, error taxonomy, mailbox ops, red-team) **and** landing Storage read-path + SDK polish moves the program from \~50% to **\~68–70%**.&#x20;

## concrete acceptance checklist (to track the sprint)

Use this as the Done-Definition grid while you land the items above:

* [x] Gateway: token buckets + `/readyz` gate; 429/503 + Retry-After present.&#x20;
* [ ] Error envelope + taxonomy; SDK parses & retries appropriately.&#x20;
* [ ] OAP/1 spec stub exists; CI greps pass.&#x20;
* [ ] Mailbox: DELETE + SUBSCRIBE; visibility timeout; idempotency preserved.&#x20;
* [ ] Golden metrics (see list in #1).&#x20;
* [ ] Red-team suite passes.&#x20;
* [ ] Storage/DHT: GET/HAS; 64 KiB streaming; BLAKE3 verified on read.&#x20;
* [ ] Tileserver example streams through Gateway; histograms visible.&#x20;
* [ ] SDK env + jittered backoff; propagates `corr_id`; respects `Retry-After`.&#x20;

---

# completion rate (now) and after next steps

* **Current project completion:** **\~50%** (weighted). This is the program-wide estimate in the daily plan, after proving the local read-path and kernel test suites.&#x20;

* **If we implement the next-step set above:** jump to **\~68–70%**. This bound explicitly appears in the plan’s “Impact math” once the Bronze ring + Storage read-path + SDK DX are in.&#x20;

---

# suggested owner map (so it can be parallelized)

* **Core:** OAP stub + error taxonomy (1 person)&#x20;
* **Gateway:** quotas/readyz + golden metrics + red-team (1–2)&#x20;
* **Mailbox:** DELETE/SUBSCRIBE + visibility (1)&#x20;
* **Storage/Index:** GET/HAS streaming + tileserver (1–2)&#x20;
* **SDK:** env + backoff + `corr_id` (1)&#x20;
* **CI/Docs:** greps, dashboards, `docs/TESTS.md` (1)&#x20;

---

## tl;dr

* You’re **at \~50%** with a clean, working read-path and kernel foundation.&#x20;
* The **fastest path to \~70%** is: golden metrics + quotas config + CI invariants + doc touch-ups, then push Storage read-path + SDK DX (with error taxonomy).  &#x20;

If you want, I can also roll these into a `docs/CARRY_OVER.md` in the repo so each new machine (or teammate) gets the exact same bootstrapping experience next time.
