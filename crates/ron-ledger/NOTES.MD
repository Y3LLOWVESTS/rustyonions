### RON-LEDGER NOTES - APRIL 26 2026 - 17:20 CST


## ron-ledger — current status

`ron-ledger` is now a **working library-first ledger core**, not just a scaffold. That matters because the TODO and crate layout were intentionally designed as a **library, not a service**: the crate should expose DTOs, types, config, error taxonomy, engine logic, storage seam, replay/checkpoint logic, and observer hooks, while a future service wrapper is responsible for HTTP, metrics endpoints, readiness, and ops surfaces. 

At the architecture level, the crate’s intended role is still exactly what the IDB says: RustyOnions’ **immutable, append-only economic truth**, with total order, conservation, deterministic accumulator roots, idempotent ingestion, crash-recovery monotonicity, amnesia compatibility, and strict role separation from wallet/rewarder/accounting. 

## What is finished now

Based on your latest successful run, the crate currently has a clean basic engineering floor:

* `cargo fmt` passes
* `cargo clippy -p ron-ledger --all-targets -- -D warnings` passes
* `cargo test -p ron-ledger --all-targets` passes
* the Criterion micro bench smoke runs successfully

That means the crate is currently **build-clean, lint-clean, test-clean, and bench-smoke-clean** in your local repo.

Functionally, the following parts are now real enough to count as implemented:

* The **library-first file structure** exists and is coherent: `api.rs`, `types.rs`, `config.rs`, `error.rs`, `engine/{storage,ledger,accumulator,checkpoint,replay,observer}.rs`, tests, bench, fuzz target, example, vectors, and CI script scaffolds. That matches the intended ron-ledger shape from the TODO.
* The crate now enforces the **core invariants we targeted first**: append-only behavior, conservation checks, deterministic roots, idempotent ingestion, stable reject taxonomy, and crash/replay restoration. Those are exactly the invariants the IDB treats as non-negotiable. 
* The crate is aligned with **Pillar 12** in the project map: `ron-ledger` as immutable truth, with wallet/accounting/rewarder kept separate.

## What the passing tests have actually proved

The current passing test set proves more than “it compiles.”

`idempotency_prop.rs` proves that repeated ingestion keyed the same way behaves like a **no-op** instead of mutating ledger state again. That is direct evidence for the IDB’s idempotent ingestion invariant.

`interop_vectors.rs` proves three different things:

* the happy-path ingest shape can parse and commit,
* malformed nested DTO fields are rejected,
* conservation is checked rather than assumed.
  That lines up with the documented goals for interop vectors and DTO strictness.

`reject_taxonomy.rs` proves the internal error mapping is stable and machine-countable for the current buckets: `Invalid`, `TooLarge`, and `Conflict`. That is exactly what the TODO says this suite is supposed to guard.

`replay_recovery.rs` is the biggest milestone. It proves file-backed replay restores both **root** and **balance state**, which is the first real proof that crash-recovery monotonicity and deterministic replay are working in practice rather than only on paper. The TODO explicitly says replay/checkpoint must yield the same `new_root` across runs, and that now appears satisfied by the current implementation.

`public_api_semver.rs` proves there is at least a basic guard on public surface drift, though it is still only a smoke-level protection compared with the full public-api and semver-checks workflow the docs expect.

The `micro.rs` bench smoke proves the ingest path can execute under Criterion without blowing up, but it does **not** yet prove the documented SLOs or regression gates. The docs expect much more than a smoke bench for performance signoff.

## What remains inside the crate

This is the most important section, because the crate is working now, but it is **not done**.

First, the crate still needs **deeper test coverage inside `src/`**. The docs expect unit coverage for accumulator hashing, config validation, serde strictness, and redaction helpers, while the current run still shows `src/lib.rs` itself has zero unit tests. 

Second, the crate still lacks the **broader validation envelope** promised by `TESTS.md`:

* more property tests,
* actual fuzz execution,
* loom concurrency models,
* chaos/smoke scripts,
* coverage thresholds,
* perf regression gating.
  The docs lay out Bronze, Silver, and Gold acceptance gates, and the crate is not yet at Silver or Gold by those standards.

Third, the **observer layer is still just hooks**, which is correct for a library crate, but it means the full observability contract in the docs is not yet realized inside a service wrapper. The TODO is explicit that `observer.rs` should emit typed callbacks only and leave Prometheus wiring to a service.

Fourth, the crate is still missing proof around **single-writer and bounded-queue concurrency semantics** at the test level. The docs expect loom/backpressure/shutdown validation, and that has not been demonstrated by the current passing suite. 

Fifth, the crate has not yet demonstrated stronger **storage corruption / disk pressure / safe-mode** behavior. The docs and runbook expect failure handling around WAL/fsync issues, seq gaps, and safe-mode behavior, but the current test set does not appear to exercise those cases yet.

Sixth, the **fuzz target exists by scaffold**, but existence is not the same thing as being run and enforced. The TODO and tests doc both expect replay/parse fuzzing to be part of the quality contract.

## What remains outside the crate

This is the key distinction:

The crate is now a decent **library core**, but the docs also describe a much larger **service-class surface** for `ron-ledger`, including:

* HTTP/UDS endpoints,
* `/healthz`, `/readyz`, `/metrics`, `/version`,
* OTEL + exemplars,
* readiness degradation rules,
* Grafana dashboards,
* Prometheus alerts,
* operator runbooks,
* TLS/mTLS posture.

That service-class surface is **not** what the TODO said to build inside this crate. The TODO is explicit that the crate itself should stay library-first and that service wrappers consume it. So those docs are still important, but they should be read as **future wrapper obligations**, not missing work inside `ron-ledger` proper. 

In other words:

* **inside the crate**: build and harden deterministic ledger logic
* **outside the crate**: later expose service HTTP/readiness/metrics/ops on top of it

That tension is normal, and the crate is currently on the right side of it.

## Honest completion estimate

I would record the current status like this:

**ron-ledger core library milestone:** about **85% complete**
Because:

* core structure exists,
* core invariants are implemented,
* replay and file-backed recovery now work,
* taxonomy and DTO strictness work,
* local fmt/clippy/tests are green.

**ron-ledger docs-level / production-grade completion:** about **60–65% complete**
Because the docs still expect:

* richer unit/property/fuzz/loom coverage,
* stronger failure-mode testing,
* perf baselines and regression gates,
* semver/public-api workflows enforced in CI,
* service-wrapper observability/readiness/ops surfaces.

So the right mental model is:

**The deterministic ledger core is mostly real now.**
**The full documented operational envelope is not finished yet.**

## Recommended “done enough to move on” line

I would not block `svc-wallet` on making `ron-ledger` perfect. I would block it only on a few remaining high-value items:

* add a few direct unit tests in `src/` for config validation, accumulator determinism, and serialization strictness
* run at least a smoke fuzz pass for `replay_parse`
* add one or two stronger replay/corruption edge tests
* optionally wire a real `cargo public-api` / semver-checks CI gate
* freeze the current carry-over notes with the current local green run

Once that is done, the crate is solid enough to stop being the bottleneck and let `svc-wallet` become the next high-impact step.

## Carry-over note you can save

`ron-ledger` is now a working, library-first deterministic ledger core aligned to the ron-ledger TODO shape. It compiles cleanly, passes clippy under `-D warnings`, passes the current integration/property-style tests, and successfully restores file-backed state through replay. The implemented core now demonstrates append-only truth, conservation checks, idempotent ingestion, strict DTO rejection, stable reject taxonomy, and deterministic replay/root restoration. Remaining work is mostly hardening and breadth: more unit coverage inside `src/`, actual fuzz/loom/chaos/perf gate execution, stronger corruption/disk-pressure scenarios, and CI enforcement for semver/public-api/perf. Treat the crate as roughly 85% complete as a ledger core and about 60–65% complete against the full docs-level production envelope. The next crate can be `svc-wallet`, but ron-ledger should still get a final small hardening pass before we stop touching it.


### END NOTE - APRIL 26 2026 - 17:20 CST

### BEGIN NOTE - APRIL 27 2026 - 19:55 CST

# 1. NOTE — `ron-ledger`

## Current Status

`ron-ledger` remains the durable economic truth substrate for the ROC value plane. We did not heavily rewrite `ron-ledger` in this final `svc-storage` session, but every higher-level WEB3 crate now depends on the correctness of the ledger model through `svc-wallet`.

The important architectural decision remains:

```text
ron-ledger is the source of committed economic truth.
svc-wallet is the mutation front-door.
svc-rewarder plans payouts but does not mutate the ledger directly.
svc-storage charges through wallet escrow rather than writing ledger entries itself.
```

This separation is critical. It prevents the storage layer, rewarder layer, or accounting layer from accidentally becoming alternate sources of balance truth.

## What `ron-ledger` Provides to the Current WEB3 Stack

`ron-ledger` is currently used as the lower-level accounting/commit engine behind `svc-wallet`. Through `svc-wallet`, it now supports the core ROC operations needed by the internal value plane:

```text
issue      explicit supply increase
transfer   balanced debit/credit movement
burn       explicit supply decrease
hold       payer → escrow reserve movement
capture    escrow → service/provider settlement movement
release    escrow → payer refund movement
balance    ledger-backed balance reads
```

The most important new dependency on `ron-ledger` is the escrow lifecycle. `svc-wallet` now wraps ledger operations into:

```text
hold    = reserve funds from user into escrow
capture = pay the service/provider from escrow
release = return unused escrow to the user
```

This gives paid storage/pinning a safe value flow without introducing direct ledger writes inside `svc-storage`.

## What Was Proven Indirectly Through Higher-Level Tests

Although this session focused more on `svc-wallet` and `svc-storage`, the ledger path was exercised through wallet tests and integration loops. The full sweep shows `svc-wallet` passing its ledger, escrow, non-negativity, conservation, double-spend, nonce, idempotency, and paid-storage smoke tests. 

The following ledger-backed invariants are now being exercised through `svc-wallet`:

```text
1. Issue creates balance for the recipient.
2. Transfer conserves total value between participants.
3. Burn explicitly reduces supply.
4. Failed transfer/burn/capture does not create partial credits.
5. Nonce replay is rejected.
6. Idempotent replay returns the same receipt instead of duplicating state.
7. Hold/capture/release preserves total value across payer, escrow, and service accounts.
8. Capture above held amount rejects without crediting the payee.
```

## Why This Matters for RON Beta

RON’s internal ROC economy now has a ledger foundation strong enough to support:

```text
provider rewards
user balances
paid storage
paid pinning
escrow settlement
future app/service billing
dashboard accounting
```

The ledger is not just a theoretical crate anymore. It is being exercised by the live wallet mutation boundary and by the paid-storage flow.

## What Remains for `ron-ledger`

### 1. Durability / Persistence Hardening

The current tests heavily exercise the in-memory/dev path through wallet. Before beta production, we need to decide exactly how ledger persistence is configured:

```text
amnesia/dev mode       in-memory, rebuildable, test/demo safe
durable beta mode      persistent storage, crash replay, audit roots
production mode        backup/restore, corruption checks, root anchoring
```

Remaining work:

```text
- confirm durable storage backend expectations
- add crash/restart replay tests if not already complete
- document ledger persistence mode for Micronode vs Macronode
- define whether beta ledger is local-only or cluster-replicated
```

### 2. Formal Receipt/Audit Surface

`svc-wallet` currently emits receipts derived from committed ledger operations. `ron-ledger` should eventually expose or support a stronger audit model:

```text
ledger sequence ranges
ledger root after commit
receipt hash binding
append-only audit trail
optional export/snapshot format
```

Remaining work:

```text
- ensure receipts bind to ledger root/sequence in a stable way
- document receipt verification rules
- define how external auditors or dashboard panels verify ledger continuity
```

### 3. Wider Integration Tests

Current ledger proof is mostly indirect through wallet. Eventually add direct multi-crate tests that verify:

```text
rewarder issue batch → wallet → ledger
paid storage hold/capture/release → wallet → ledger
ledger balances match dashboard/API views
ledger replay preserves balances and receipt history
```

### 4. Supply Policy Documentation

The ledger supports explicit supply exceptions like issue and burn, but the system needs governance documentation for:

```text
who can issue ROC
when reward issuance is allowed
how burns are authorized
how supply is reported
how accidental issuance is prevented
```

### 5. Future ROX Boundary

ROX/Solana remains deferred. `ron-ledger` should continue to treat ROC as internal truth. Future bridge/export work should not pollute the current ledger model with external chain assumptions.

## Recommended Next Steps for `ron-ledger`

```text
1. Keep ron-ledger stable while hardening wallet/storage/accounting integration.
2. Add/confirm durable replay tests.
3. Add a ledger receipt verification note.
4. Add a beta-mode persistence decision.
5. Avoid adding ROX/Solana logic until ROC beta is proven internally.
```

## Completion Estimate

```text
ron-ledger for internal ROC beta: ~85–90%
ron-ledger for production-grade audited economy: ~70–78%
```

The crate appears functionally strong for the current internal economy, but production durability, audit export, and governance documentation remain the main unfinished pieces.

---

### END NOTE - APRIL 27 2026 - 19:55 CST