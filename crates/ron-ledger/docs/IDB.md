
---

# 📄 Paste-ready replacement — `crates/ron-ledger/docs/IDB.md`

````markdown
---
title: ron-ledger — Economic Truth (IDB)
version: 1.4.0
status: reviewed
msrv: 1.80.0
last-updated: 2025-10-13
audience: contributors, ops, auditors
pillar: P12 — Economics & Wallets
concerns: [ECON, SEC, GOV]
owners: [Stevan White]
---

# 0) Role & Scope

**What is this doc?**  
**IDB = Invariant Driven Blueprinting.** It’s our blueprint that defines **invariants**, **scope/anti-scope**, **design principles**, and **operational glue** that must hold for `ron-ledger` across implementations and environments. It is enforceable via tests, CI gates, and runbooks.

**Role.** `ron-ledger` is RustyOnions’ **immutable, append-only economic truth**. Transient counters/intents flow **in**; a canonical, auditable state (with deterministic roots) flows **out** to rewarders, wallets, and auditors.

**Scope (what it is):** settlement journal with **total order**, **conservation**, and **deterministic accumulator** roots.

**Non-scope (what it is not):** wallet UX/limits, reward distribution logic, ads economics, OLAP/analytics, or mutable balance tables.

---

# 1) Invariants (MUST)

- **[I-1] Append-only truth.** No in-place mutation/deletion; total, gap-free order.
- **[I-2] Conservation & non-negativity.** Value is conserved; balances cannot go negative except via bounded, explicitly modeled instruments (e.g., holds) enforced by policy.
- **[I-3] Role separation.** Accounting = transient counters; Ledger = canonical truth; Rewarder = distribution; Wallet = spend enforcement.
- **[I-4] Capability-gated writes.** Every mutation requires a verified capability (macaroons or equivalent). No ambient authority.
- **[I-5] Deterministic state hash.** Each commit yields a deterministic accumulator root (for anchoring/audit).
- **[I-6] Idempotent ingestion.** Replays of the same mutation (idempotency tuple) cannot diverge state.
- **[I-7] Conflict ordering.** Conflicts resolve by a single, stable rule: `(seq, tie_break)`.
- **[I-8] Observability.** Export canonical metrics (commit latency, backlog, rejects{reason}, root continuity) and health probes.
- **[I-9] Governance hooks.** Policy/regulatory constraints (limits, residency, reversible bridges) enforce **pre-commit**.
- **[I-10] Amnesia honor (Micronode).** Default in-memory engine; no disk artifacts unless explicitly disabled.
- **[I-11] Key custody & redaction.** Persist/log **KIDs** (Key IDs) only; no private keys/tokens/signatures in logs/traces.
- **[I-12] Canon alignment.** Pillar P12; concerns = ECON/SEC/GOV; never absorb wallet/rewarder/ads semantics.
- **[I-13] Crash-recovery monotonicity.** Recovery resumes from last committed `seq/root`; partial batches replay to the *same* `new_root`.
- **[I-14] Reversible bridges are bounded.** Reversals/chargebacks/holds require GOV-approved capability; must link to original entry; conservation proven.
- **[I-15] Backward-compatible evolution.** Entry formats are versioned; additive growth preferred; unknown kinds hard-reject unless policy toggled.

---

# 2) Design Principles (SHOULD)

- **[P-1] Simple model.** Compact append-log of typed entries + periodic checkpoints (accumulator roots).
- **[P-2] Determinism over raw throughput.** Scale with batching; keep commit critical section minimal.
- **[P-3] Plane separation.** Capability/policy/wallet checks **before** atomic append; commit path is short/deterministic.
- **[P-4] Audit-first.** First-class extraction of ranges, roots, and proofs (CLI + API).
- **[P-5] Interop-neutral.** Ledger records verified movements; business interpretation stays outside.
- **[P-6] Fail closed.** Unknown kinds / invalid capabilities → structured rejects; no partial writes.
- **[P-7] Storage by profile.** Macronode = persistent (WAL/LSM). Micronode = in-mem (Amnesia). Same append/checkpoint shape.
- **[P-8] Explicit extensibility.** New instruments = new kinds behind feature/policy toggles; never mutate legacy kinds.
- **[P-9] Deterministic batching.** Same input batch → same `seq` and `root` across runs/nodes.

---

# 3) Implementation (HOW)

## 3.1 Entry schema (wire sketch)

```rust
/// KID = Key ID (identifier for the KMS/HSM key used to authorize the capability).
/// We only store/log the KID, never private material or raw signatures/tokens.
struct LedgerEntry {
  id: Uuid,
  ts: UnixMillis,
  seq: u64,                 // assigned by sequencer, strictly increasing
  kind: EntryKind,          // Credit|Debit|Transfer|Mint|Burn|Hold|Reverse|...
  account: AccountId,
  amount: u128,             // fixed scale (integer cents/atoms)
  nonce: [u8; 16],          // caller-provided idempotency component
  kid: Kid,                 // key id reference only
  capability_ref: CapId,    // capability granting this mutation
  prev_root: Hash256,       // accumulator before
  new_root: Hash256,        // accumulator after
  v: u16,                   // entry schema version
}
````

* **At rest:** compact binary (`postcard`/`bincode`) append segments + checkpoints.
* **Edges:** strict JSON with `serde(deny_unknown_fields)`.
* **Idempotency:** tuple `(account, amount, nonce)` (+ optional caller `idem_id`) is a no-op if repeated.

## 3.2 Sequencing & ordering

A single sequencer assigns `seq`; ties resolved deterministically (e.g., `(source_ts, entry.id)`).
Commit path:
`verify(capability) → policy.guard → wallet.double_spend_check → enqueue → atomic append + root → publish`.

## 3.3 Storage engines & WAL

| Engine  | Profile             | Strengths                                | Risks / Notes                                |
| ------- | ------------------- | ---------------------------------------- | -------------------------------------------- |
| In-mem  | Micronode (Amnesia) | Zero disk IO; trivially wiped            | Volatile; snapshot/export optional           |
| RocksDB | Macronode (default) | Mature LSM; fast WAL; compaction control | Needs tuning; compaction stalls if mis-set   |
| Sled*   | Macronode (alt)     | Simple API; embedded                     | Past edge cases; use only if pinned & tested |

* Use a pinned, vetted version; run the destructive test suite before enabling.

**WAL replay (pseudocode):**

```text
open last_checkpoint → (root, height)
for seg in segments_after(last_checkpoint) in order:
  for rec in seg:
    if rec.seq == height+1 && hash(prev_root, rec) == rec.new_root:
      apply(rec); height += 1; prev_root = rec.new_root
    else:
      SAFE_MODE=1; stop writes; expose /recovery; raise alerts; require operator action
fsync accumulator + checkpoint(height, prev_root)
```

**Disk-full / corruption handling:**

* Pre-allocate segments; detect `ENOSPC` **before** append; fail-closed; set `disk_pressure=1`.
* On checksum mismatch: increment `ledger_seq_gap_detected_total`, set `ledger_safe_mode=1`; block writes until resolved.

## 3.4 Accumulator abstraction

```rust
trait Accumulator {
  fn update(prev_root: Hash256, entry: &LedgerEntry) -> Hash256;
  fn checkpoint(height: u64, root: Hash256);
  fn verify_range(start_seq: u64, end_seq: u64) -> RangeProof;
}
```

Default = **Merkle** (SHA-256/BLAKE3). Verkle/EdAcc live behind a feature flag with **dual-write migration** (§8).

## 3.5 Tracing & correlation

* **OpenTelemetry** tracing (OTLP exporter).
* Propagate `corr_id`/`span_id` end-to-end; record `seq`, `entry.id`, `kid`, `capability_ref`, `batch_size`.
* Sampling: 1% baseline; **100%** during `ledger_safe_mode=1`.

## 3.6 Redaction discipline

Logs/traces store IDs only (entry id, KID, capability id). **Never** log signatures, secrets, or plaintext tokens.

---

# 4) Governance & Economic Integrity

**System-level invariants:** no double issuance/spend; entries balance; emissions follow schedule; governance actions auditable; bounded authority (no “god mode”).

**Roles/Boundaries:**

* *Policy* proposes limits/bridges; *Ledger* enforces; *Rewarder* cannot mint; *Wallet* enforces spend caps.
* All authorities act via **revocable capabilities** (macaroons) with `KID`-anchored proofs.

**Quorum & timelines:**

* **Multi-sig quorum:** default **3/5** for emergency freeze; **2/3** for parameter changes (configurable; auditable).
* **Dispute SLA:** tag `disputed=true` within **15 minutes**; adjudication within **48 hours**; outcomes are **linked entries**.
* **Emission deviations:** alert at **>1%** instantaneous variance; governance-freeze evaluation at **>5%** sustained **10 minutes**.

**Economic SLOs:**

* 99.9% settlements < **5 s** end-to-end (ingress → committed root published).
* Root publication lag < **1 s** after commit.

---

# 5) Interop Pointers

* **Ingress:** `POST /ingest` (batch), `GET /roots?since=seq`, `GET /healthz`, `GET /readyz`, `GET /metrics`, `GET /version`.
* **DTOs:** `IngestRequest`, `IngestResponse`, `RejectReason`, `ReadyzReport` are `#[non_exhaustive]`.
* **Wire guarantees:** additive JSON only; deny unknown fields; breaking HTTP = new major + migration note.

---

# 6) Anti-Scope (Forbidden)

* ❌ Wallet UX/limits; ❌ reward algorithms; ❌ ads economics.
* ❌ Materialized, mutable balance tables as a source of truth.
* ❌ Writes without capability + policy checks.
* ❌ Logging secrets/signatures/tokens (store **KID** only).
* ❌ Ignoring Amnesia mode in Micronode.
* ❌ OLAP/secondary indexes beyond audit/range proofs.
* ❌ Non-deterministic commit ordering.

---

# 7) Observability, Alerts & Dashboards

**Prometheus metrics (minimum set):**

* `ledger_commits_total`
* `ledger_commit_latency_seconds` (histogram)
* `ledger_backlog_gauge`
* `ledger_rejects_total{reason}`
* `ledger_seq_gap_detected_total`
* `ledger_safe_mode` (0/1)
* `ledger_roots_published_total`

**Alert thresholds (initial):**

* p95 `ledger_commit_latency_seconds` > **80ms** for 5 min → **WARN**
* `ledger_backlog_gauge` > **10×** steady-state for 2 min → **PAGE**
* `ledger_seq_gap_detected_total` delta > 0 → **PAGE**
* `ledger_safe_mode` == 1 → **PAGE**
* `ledger_rejects_total{reason="unknown_kind"}` rate > 0 for 5 min → **WARN**

**Dashboards (Grafana):**

* Latency heatmap by batch size; backlog vs ingest RPS; rejects by reason; safe-mode timeline with deploy annotations.

**Tracing:**

* OTEL service graph: `ingress → policy → wallet → ledger.commit`.
* Span attrs: `seq`, `entry_id`, `kid`, `cap_ref`, `batch_size`.

---

# 8) Migrations & Cutovers

**Accumulator change (Merkle → Verkle):**

* Phase A (dual-write): compute both roots; publish **Merkle** as canonical.
* Phase B (shadow verify): compare roots/range proofs; **alert** on mismatch.
* Phase C (cutover): flip canonical root; keep dual-write **24h**.
* **Rollback:** if alerts fire, revert canonical to Merkle; retain Verkle artifacts for forensics.

**Storage engine change (e.g., RocksDB bump):**

* Online canary (1% traffic) with mirror append; verify replay/roots; promote if clean.
* **Rollback:** pointer flip back to prior engine; keep mirror segments.

---

# 9) Backup, Snapshots & DR

**RPO/RTO (Macronode):** **RPO ≤ 1 min**, **RTO ≤ 15 min**.

**Snapshot policy:**

* Checkpoint + segment snapshot every **5 min**; retain **24 h** locally, **7 d** offsite (WORM bucket).
* Snapshots are **signed**; include `height`, `root`, `seq_range`.

**Restore procedure:**

1. Provision node; verify KMS & capabilities; set `safe_mode=1`.
2. Restore latest signed checkpoint + segments; verify chain-of-trust and roots.
3. WAL replay to head; compare computed root to snapshot; exit safe-mode.

**Micronode (Amnesia):**

* No persistent backups by default; optional export endpoints for user-initiated snapshots.

---

# 10) Tests & CI Gates (Definition of Done)

**Golden tests:**

* **Replay/idempotency:** same batch ⇒ identical `new_root`.
* **Recovery:** kill -9 during commits → final `root` equals expected; `seq` contiguous.
* **Disk-pressure:** simulate `ENOSPC` pre-append → fail-closed; no partial append; alert raised.
* **Corruption:** inject checksum error → **safe-mode**; writes blocked until operator clears.
* **Amnesia matrix:** CI runs with amnesia ON/OFF; ON leaves *no* disk artifacts.
* **Governance proofs:** capability/policy violations → correct 4xx taxonomy; reversible actions link originals; conservation nets to zero.
* **Evolution compatibility:** N-1/N-2 readers ignore new kinds safely.

**Performance SLOs (with load profiles):**

* p95 ≤ **80 ms**, p99 ≤ **150 ms** at:

  * **Micronode A:** 300 RPS ingest, median batch=8, p90 batch=32.
  * **Macronode B:** 1,200 RPS ingest, median batch=16, p90 batch=64.
* **Burst:** 10× surge drains ≤ **5 s** without loss/duplication.

**Fuzz/soak targets:**

* Fuzz entry decoding, WAL segment boundaries, checkpoint headers, reject taxonomy.
* 24h soak with randomized conflicts & reversals; assert I-1..I-15 and root continuity.

**Wire/API snapshots:**

* Rust: `cargo public-api` snapshot at `docs/api-history/ron-ledger/<version>.txt` (CI denies unexpected).
* HTTP: OpenAPI diff must be **non-breaking** unless major bump; CI bot posts diff + checklist.

---

# 11) Operational Notes

* Configs in version control + secret store; mount read-only; TLS material `0600`.
* `/metrics` scrapers gated by network policy; `/healthz` shallow; `/readyz` reflects backpressure.
* External sequencer (if any): mTLS, capability-gated, liveness probed.
* Post-incident: export offending range; attach range proofs; store with incident ticket.

---

# 12) References & Terms

* **KID** — *Key ID* (identifier in KMS/HSM). We never persist/log private key material.
* Pillar P12 crate set: `ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`, `svc-ads`, `svc-interop`.
* Concern mapping: ECON, SEC, GOV.
* See `API.md` (DTOs/endpoints), `INTEROP.md` (wire guarantees), `SECURITY.md` (supply chain, redaction), `OBSERVABILITY.md` (dashboards).

```
```

---

## What changed vs the last draft

* **IDB meaning is explicit** (“Invariant Driven Blueprinting”) in §0.
* **Storage/WAL depth**: engine comparison table, replay pseudocode, `ENOSPC`/corruption handling, **SAFE_MODE** semantics.
* **Tracing**: OpenTelemetry, correlation fields, and sampling rules.
* **Governance specifics**: quorum thresholds, dispute SLA, emission deviation triggers.
* **Alerting**: concrete Prometheus thresholds and pages; dashboard callouts.
* **Backup/DR**: explicit **RPO/RTO**, signed snapshots, restore flow.
* **Performance**: concrete load profiles for Micronode/Macronode.


