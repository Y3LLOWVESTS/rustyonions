

---

````markdown
---
title: svc-rewarder — Invariant-Driven Blueprint (IDB)
version: 1.1.0
status: reviewed
last-updated: 2025-10-13
audience: contributors, ops, auditors
---

# svc-rewarder — IDB

## 1. Invariants (MUST)
- [I-1] **Canon & Pillar fit:** `svc-rewarder` is one of the fixed 33 crates under Pillar 12 (Economics & Wallets). No renames, no new crates.
- [I-2] **Strict boundaries:** Rewards math is pure & deterministic. Final settlement lives in `ron-ledger`. Transient volume/rollups originate in `ron-accounting`. Rewarder never becomes a ledger, wallet, ads engine, or policy editor.
- [I-3] **Conservation:** `Σ allocations ≤ pool_total` for the epoch. On violation: fail-closed, quarantine the run, emit audit event, do **not** emit intents.
- [I-4] **Idempotency:** Same `(epoch, policy_id, inputs_hash)` ⇒ byte-identical manifest. Downstream intents carry this key to prevent duplicate settlement.
- [I-5] **Immutable inputs:** Epochs read **sealed** snapshots (content-addressed `b3:<hex>`) for accounting/ledger/policy. Inputs are recorded in the run manifest.
- [I-6] **Capability security:** All ingress/egress are capability-bound (macaroons/caps). No ambient trust or pass-through bearer tokens.
- [I-7] **Observability:** `/metrics`, `/healthz`, `/readyz`, `/version` are required. Export golden metrics and conservation deltas.
- [I-8] **Protocol limits & backpressure:** Honor framing/size limits; apply bounded queues with explicit shed/degrade paths. Never block the runtime with unbounded CPU work.
- [I-9] **Amnesia mode:** With amnesia=ON (Micronode profile), no persistent artifacts remain after run completion; secrets are zeroized.
- [I-10] **Governance:** Reward policy and actor registry are versioned, signed, and referenced by ID; policy changes are audit-visible in manifests.
- [I-11] **Money safety:** No IEEE-754 floats in any monetary path. Use integer minor units or fixed-point decimal with explicit scale; conversions are lossless and tested.
- [I-12] **Proof evolution:** Each run emits a deterministic **commitment** (BLAKE3 over canonicalized inputs+outputs). Optional zk proof artifacts MAY be attached now or later without changing allocations; the transcript domain-separates the epoch.

## 2. Design Principles (SHOULD)
- [P-1] **Math/IO split:** Keep reward calculus in a pure crate/module; all external calls live in thin adapters (ledger/accounting/policy/wallet).
- [P-2] **Epoch windows & watermarks:** Use fixed windows (hour/day) with watermarking to bound recomputation; allow safe replay.
- [P-3] **Policy as data:** Treat weights/tariffs as signed docs resolved via registry/policy services; no hard-coded rates.
- [P-4] **Proof-first outputs:** Always write `run.json` + `commitment.txt` and, if enabled, `proof.*` (zk) with clear provenance and CIDs.
- [P-5] **Failure-aware readiness:** `/readyz` flips to NOT READY under queue/CPU pressure or dependency outage; `/healthz` remains conservative.
- [P-6] **DTO hygiene:** `#[serde(deny_unknown_fields)]`, semantic versioned schemas, forward/backward compat within major.
- [P-7] **Parallelism & sharding:** Partition work by stable keys (e.g., creator_id, content_id) to maximize cache-locality; use bounded task sets (Tokio) or rayon for CPU-only phases.
- [P-8] **Error taxonomy:** Classify errors as `Retryable | Fatal | Quarantine`. Only Retryable may auto-retry; Quarantine seals artifacts and pages ops.
- [P-9] **Profile symmetry:** Same API/behaviors across Micronode/Macronode; only storage knobs differ.

## 3. Implementation (HOW)
- [C-1] **Module layout**
  - `core/` — `compute(epoch, snapshot, policy) -> AllocationSet` (pure; no IO, no time).
  - `inputs/` — readers for `accounting_snapshot`, `ledger_snapshot`, `policy_doc` (content-addressed, verified).
  - `outputs/` — `intent_writer` (to ledger/wallet), `artifacts_writer` (manifest, commitment, optional zk).
  - `schema/` — DTOs referencing `ron-proto` versions; `serde(deny_unknown_fields)`.
  - `ops/` — error taxonomy, backoff, quarantine, readiness gates.
- [C-2] **Run manifest (canonical fields)**
  ```json
  {
    "epoch": "2025-10-13T00:00Z",
    "policy_id": "policy:v3:sha256-...",
    "inputs": {
      "accounting_cid": "b3:...",
      "ledger_cid": "b3:...",
      "policy_cid": "b3:..."
    },
    "pool_total_minor": "123456789",
    "allocations": [{"actor":"did:ron:...","amount_minor":"...","explain":"weight:views*rate"}],
    "payout_total_minor": "123450000",
    "delta_minor": "6789",
    "idempotency_key": "b3:<canonical-hash>",
    "commitment": "b3:<inputs+outputs canonical hash>",
    "proof": {"type":"optional","cid":"b3:...","scheme":"groth16|plonk|none"},
    "version": "1.1.0"
  }
````

*Canon note:* `idempotency_key` and `commitment` are computed over **sorted JSON** (UTF-8, no insignificant whitespace).

* [C-3] **Conservation (pseudo)**

  ```rust
  let payout = allocations.iter().map(|a| a.amount_minor).sum::<i128>();
  assert!(payout <= pool_total_minor, "conservation violated: payout>{}", pool_total_minor);
  ```
* [C-4] **Idempotency**

  ```
  run_key = blake3("svc-rewarder|epoch|policy_id|" + inputs_cids_sorted_json);
  ```

  Every downstream intent carries `run_key` to dedupe at the ledger/wallet boundary.
* [C-5] **Metrics (golden)**

  * `reward_runs_total{status}`  // success|quarantine|fail
  * `reward_compute_latency_seconds` (histogram)
  * `reward_pool_total_minor` (gauge)
  * `reward_payout_total_minor` (gauge)
  * `reward_conservation_delta_minor` (gauge, can be zero or positive only)
  * `rejected_total{reason}`  // policy_invalid|input_stale|cap_denied|schema_mismatch
  * `readyz_degraded{cause}`  // backpressure|dep_outage|quota
* [C-6] **Backpressure & CPU**

  * Per-epoch bounded work queues; CPU-heavy phases offloaded to bounded worker pool.
  * Use `bytes::Bytes` on hot paths; avoid locks across `.await`.
* [C-7] **Interfaces (HTTP)**

  * `GET /rewarder/epochs/{id}` → 200 manifest (cacheable)
  * `POST /rewarder/epochs/{id}/compute` → 202 (started) | 200 (exists) | 409 (in-flight/duplicate); capability required
  * `GET /rewarder/policy/{id}` → signed policy doc
* [C-8] **Quarantine flow**

  * On conservation or schema breach: write manifest with `status=quarantine`, no intents, emit `audit.event` and page ops (GOV label).
* [C-9] **Amnesia wiring**

  * Artifact path → tmpfs when amnesia=ON; cleanup on success/failure; secrets zeroized on drop.

## 4. Acceptance Gates (PROOF)

* [G-1] **Unit/property tests**

  * Conservation property: `payout_total ≤ pool_total` across randomized vectors.
  * Idempotency: identical inputs ⇒ byte-identical manifest and `commitment`.
  * Money safety: round-trip minor↔decimal conversions are lossless.
* [G-2] **Golden vectors**

  * Freeze canonical epochs with fixed inputs; CI compares manifests bit-for-bit.
* [G-3] **Fuzz**

  * DTO decode, policy parsing, and ledger/accounting snapshot shims (structure-aware).
* [G-4] **Perf gates**

  * `reward_compute_latency_seconds` p95 budget per profile (Micronode/Macronode) enforced; store flamegraph on regression.
* [G-5] **Observability gates**

  * `/metrics`, `/healthz`, `/readyz` respond and flip correctly under induced pressure.
* [G-6] **Amnesia matrix**

  * CI runs with amnesia ON/OFF; verify zero persistent artifacts when ON.
* [G-7] **Governance checks**

  * Verify policy signatures/versions; manifest references match registry.
* [G-8] **End-to-end sandbox**

  * Spin minimal `ron-ledger` + `ron-accounting` fixtures; run compute → emit intents → ledger dedupe proves idempotency.
* [G-9] **Chaos & degradation**

  * Inject queue pressure and dependency delays; `/readyz` must degrade; retries respect taxonomy.
* [G-10] **CI teeth**

  * `xtask idb-validate` enforces: no floats in money paths, no new crates, forbidden modules, presence of required metrics & endpoints.

## 5. Anti-Scope (Forbidden)

* Rewarder MUST NOT:

  * mutate ledger balances or hold custody; only emit settlement intents.
  * accept unsandboxed external tokens without capability translation.
  * exceed protocol/size limits or run unbounded parallel work.
  * persist artifacts in amnesia mode.
  * use floating-point types for any monetary arithmetic.
  * introduce new crates or leak wallet/ledger responsibilities.

## 6. References

* Canon docs: Complete Crate List, 12 Pillars, Full Project Blueprint, Scaling Blueprint, Hardening Blueprint, Six Concerns.
* Cross-crate: `ron-ledger` (settlement), `ron-accounting` (rollups), `ron-proto` (schemas), `ron-policy`/registry (policy).
* Ops: RUNBOOK.md (quarantine, conservation violation triage), TESTS.md (golden/fuzz/perf harness).

```
---

