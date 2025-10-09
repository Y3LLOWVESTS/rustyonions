---

title: svc-registry — Invariant-Driven Blueprint (IDB)
version: 1.0.0
status: draft
last-updated: 2025-10-08
audience: contributors, ops, auditors, integrators
--------------------------------------------------

# svc-registry — Invariant-Driven Blueprint (IDB)

> **Role (one line)**: The authoritative registry of nodes/services and their **signed descriptors**, with versioned governance (multi-sig), bus announcements, and **zero ambient authority**.

---

## 1) Invariants (MUST)

* **[I-1] Canon alignment.** `svc-registry` is a fixed canon crate; no renames, no scope drift.
* **[I-2] Signed descriptors only.** Every record (node/service/region/placement set) is a **signed artifact**; signature verification **precedes** publication or query.
* **[I-3] Multi-sig governance.** Mutations require configurable **M-of-N** approvals; quorum, signer set, and version are validated atomically; changes are **append-only** with monotonic versioning.
* **[I-4] Capability-gated writes.** No ambient trust: write paths require explicit, attenuable capabilities (e.g., macaroons) issued by the identity/auth stack; short-lived, auditable.
* **[I-5] Audit evidence.** Every accepted or rejected mutation emits a structured **audit event** containing input hashes, signers, decision, and reason.
* **[I-6] Bus announcements.** Commits publish **immutable** update events on the kernel bus; the write path never blocks on subscribers; all drops are counted.
* **[I-7] DTO hygiene.** Public DTOs live in `ron-proto`; all (de)serializers use `#[serde(deny_unknown_fields)]`; private fields never escape.
* **[I-8] Hardening defaults.** Service enforces timeouts, inflight caps, RPS, body size ≤ 1 MiB; UDS mode uses strict FS perms; peer creds checked when applicable.
* **[I-9] Amnesia compliance.** Micronode profile with amnesia=ON persists no local byproducts beyond required state; transient caches are RAM-only.
* **[I-10] Geo/Residency correctness.** Region/placement maps are versioned and signed; registry is source of truth; zero policy-violation commits.
* **[I-11] Tamper-evident audit chain.** All commits (including rejections) link via `prev_hash` (BLAKE3) to form an append-only hash chain; any fork requires an explicit, signed **supersede** record with human-readable reason.
* **[I-12] Signer lifecycle.** Signer sets are registry artifacts; rotation and revocation occur **only** via signed commits. Revoked keys are immediately invalid for approvals; partial quorums that include revoked keys are rejected.
* **[I-13] Availability floor.** **Reads remain available** under degradation; write path may brown-out first. Startup from last valid `HEAD` and deterministic log replay is mandatory.
* **[I-14] Amnesia vs audit.** In amnesia mode, audits are still **emitted** (bus/off-box collector) but not locally persisted. Macronode persists locally. No mode may omit audit emission.
* **[I-15] Back-pressure safety.** Bus/metrics back-pressure can’t block commits. Dropped notifications are **counted** and **exposed**; consumers reconcile via `GET /registry/head`.
* **[I-16] DTO versioning.** All public DTOs carry `schema_version` (semver). Evolution is additive; removal/breaks require a major version and deprecation window.
* **[I-17] Bounded growth (cost/scale).** The append-only log is **retained and compacted** by policy: periodic **checkpoints/snapshots** and **pruned segments** beyond a configurable horizon (by count, age, or size). Compaction is tamper-evident (each checkpoint signs its coverage). Retention violations must alert.

---

## 2) Design Principles (SHOULD)

* **[P-1] Read-optimized, write-audited.** Queries are fast, side-effect-free; writes are explicit and auditable.
* **[P-2] Small surface, stable semantics.** Keep API minimal: `get/list/stream`, `propose/approve/commit`. Deprecations follow explicit windows.
* **[P-3] Separation of powers.** Registry **publishes** signed state; services **enforce**; policy **lives** in `ron-policy`.
* **[P-4] First-class observability.** `/metrics`, `/healthz`, `/readyz`, `/version`; golden histograms/counters with stable names.
* **[P-5] Interop-first.** Treat descriptors as content-addressed blobs (`b3:<hex>`) with stable DTOs.
* **[P-6] Parametric hardening.** Limits/timeouts are runtime-configurable with reasoned “golden defaults”.
* **[P-7] Trace what can page a human.** Approval, quorum, commit, and bus emit are traced with stable span names and error codes.
* **[P-8] Explicit deprecation windows.** Public DTO/API changes follow a documented window: **min(2 minor releases, 180 days)**. Deprecations are announced via `/version` and `/schema`, plus `Deprecation` and `Sunset` headers.

---

## 3) Implementation (HOW)

> Scope: service (Axum/Tower) + storage abstraction (sled/sqlite/kv) with an **append-only** log of signed descriptor sets; **no policy engine** inside.

* **[C-1] DescriptorSet (sketch).**
  `DescriptorSet { version:u64, items: Vec<Descriptor>, prev_hash: Option<b3>, created_at, expiry? }` — hashed to `b3:<hex>`. Version increases by 1 per commit.
* **[C-2] Signed artifact envelope.**

  ```text
  SignedArtifact {
    schema_version: SemVer,
    payload: DescriptorSet,        // DTO per ron-proto
    payload_hash: b3,
    approvals: Vec<ThresholdSig>,  // {signer_id, algo, sig, signed_at}
    prev_hash: Option<b3>,         // audit chain
    committed_at: Timestamp,
  }
  ```

  Evolution: additive fields only; breaking changes → major bump.
* **[C-3] Write path.**
  `POST /registry/proposals` (cap required) → store pending proposal.
  `POST /registry/approvals/{proposal_id}` → verify signer, append approval.
  `POST /registry/commit/{proposal_id}` → enforce M-of-N → compute `payload_hash` → produce `SignedArtifact` → append commit → advance `HEAD` → emit **audit + bus**.
* **[C-4] Read path.**
  `GET /registry/head` → current version + content hash;
  `GET /registry/{version}` → full set;
  `GET /registry/stream` (SSE/WebSocket) → follow updates. Reads can be public but rate-limited; never expose private fields.
* **[C-5] AuthZ.**
  Capabilities carry scopes: `{registry:propose, registry:approve, registry:commit}`; TTL ≤ 30 days; rotation seamless; delegation/attenuation supported.
* **[C-6] Error taxonomy.**
  Typed errors across all layers: `InvalidSig`, `QuorumFailed`, `RevokedKey`, `VersionSkew`, `ChainMismatch`, `HardLimitExceeded`, `AuthzDenied`. Codes appear in logs, spans, and metric labels.
* **[C-7] Observability & health.**
  Metrics:

  * `registry_updates_total{result}`
  * `registry_pending_proposals`
  * `registry_signers{org}`
  * `request_latency_seconds{route}`
  * `rejected_total{reason}`
  * `bus_overflow_dropped_total`
  * `registry_recoveries_total`
  * `audit_chain_verify_seconds`
    `/readyz` is **write-aware** and flips to unready on storage/bus faults before read unready.
* **[C-8] Hardening as config.**

  ```toml
  [hardening]
  request_timeout_ms   = 5000
  max_inflight         = 512
  max_rps              = 500
  max_body_bytes       = 1048576    # 1 MiB
  decompress_ratio_max = 10
  ```

  CI asserts these defaults; ops may override.
* **[C-9] Availability/Bootstrap.**
  Startup: (1) load `HEAD`; (2) validate `SignedArtifact` chain back to the last checkpoint; (3) **serve reads** after `HEAD` validation; (4) continue background deep-verify with progress metric.
* **[C-10] Amnesia mode.**
  Micronode: RAM caches only; no disk spill; audits emitted to bus/collector; local persistence off. Macronode: persist `HEAD` + log.
* **[C-11] Bus integration.**
  After commit emit: `KernelEvent::ConfigUpdated{version}` and `RegistryUpdated{version,b3}`; use bounded channels; **one receiver per task**.
* **[C-12] Tracing.**
  OpenTelemetry spans: `registry.propose`, `registry.approve`, `registry.commit`, `registry.emit_bus`. Attributes: `proposal_id`, `target_version`, `quorum_m`, `quorum_n`, `result`, `err.code`, `payload_b3`. Link to incoming HTTP span.
* **[C-13] Storage abstraction.**
  Trait `RegistryStore` with ops: `put_proposal`, `append_approval`, `commit_signed_artifact`, `get_head`, `get_version`, `stream_from(v)`. Implementations must be crash-safe and append-only.
* **[C-14] Supersede mechanics (precise).**
  Supersede is a **signed administrative commit** with:

  * `supersede_of: b3` (must equal current HEAD at commit time; CAS on HEAD),
  * `supersede_reason: enum { KeyCompromise, DataError, GovernanceAction, Other(String) }`,
  * inclusion of a **Checkpoint** if it alters retention.
    Readers treat supersede as a new HEAD with an **unbroken** hash chain.
* **[C-15] Retention/compaction.**
  Background task generates `Checkpoint { coverage: [from,to], merkle_root, signers, created_at }`.
  After a durable checkpoint, segments strictly before `coverage.from` are prunable per policy.
  Metrics: `registry_storage_bytes`, `registry_log_segments`, `registry_checkpoint_age_seconds`.
* **[C-16] Deprecation signaling.**
  `/version` and `/schema` include:

  * `deprecations: [{surface, since, sunset, notes}]`
    HTTP adds:
  * `Deprecation: <version>` and `Sunset: <RFC3339>` on deprecated surfaces.
    Traces include `deprecation.active=true` when a deprecated surface is invoked.

---

## 4) Acceptance Gates (PROOF)

* **[G-1] Multi-sig enforcement.** Property tests: **no commit** without quorum across randomized M-of-N sets; negative tests for replay and duplicate approvals.
* **[G-2] Signature verification.** Unit/integration tests: happy path, wrong key, expired key, revoked signer; deterministic error codes.
* **[G-3] Append-only & monotonic.** Proptest: version increments by 1; `HEAD` hash matches committed set; rollback only via explicit, signed **supersede**.
* **[G-4] Observability surfaces.** `/metrics`, `/healthz`, `/readyz`, `/version` exist; golden metrics and labels exported; `/readyz` degrades **writes first** under stress.
* **[G-5] Limits enforcement.** Self-test container: (a) 2 MiB body → 413; (b) decompression bomb → rejected; (c) scrape `request_latency_seconds` and assert budget.
* **[G-6] Amnesia matrix.** Micronode(amnesia=ON) leaves **no** residual files beyond strict runtime scratch; Macronode persists only expected artifacts. CI toggles both.
* **[G-7] Six-Concerns gate.** Changes touching registry must pass labeled checks: **GOV** (runbooks/alerts), **SEC** (caps only), **RES** (bounded queues), **PERF** (latency histograms present).
* **[G-8] Geo/Residency proofs.** Staging rollouts show **0** policy violations; signed region maps validate end-to-end.
* **[G-9] Audit chain integrity.** Property + differential tests: any tampering of `prev_hash` chain is detected; two independent verifiers agree on `HEAD`.
* **[G-10] Signer lifecycle.** Rotation/revocation scenarios, including mid-proposal rotation; pre-rotation approvals valid, post-revocation approvals rejected; include clock-skew cases.
* **[G-11] Fuzzing.** AFL++/libFuzzer corpus for DTOs (CBOR/JSON): unknown fields, truncations, duplicate approvals, mismatched `payload_hash` → safely rejected.
* **[G-12] Chaos & back-pressure.** Inject faults: stalled bus subscriber and slow storage. Commits still succeed; `bus_overflow_dropped_total` increments; `/readyz` flips **write-unready** before **read-unready**.
* **[G-13] Bootstrap resilience.** Kill/restart during commit window: recovers to last committed `HEAD` with no partial state visible to readers; `registry_recoveries_total` increments.
* **[G-14] Trace coverage.** CI validates presence and naming of spans `{propose, approve, commit, emit_bus}` with consistent `proposal_id`/`target_version`. Fail build if spans missing/mis-named.
* **[G-15] Supersede integrity.** Proptests ensure: (a) supersede can only target the **current** HEAD (CAS succeeds/fails correctly); (b) chain remains single-headed (no forks); (c) human-readable `supersede_reason` present; (d) replays rejected.
* **[G-16] Retention & checkpoints.** Chaos tests verify: with retention enabled, checkpoints are created, old segments prune, and recovery from **checkpoint + tail** yields the exact HEAD. Alert fires if `registry_checkpoint_age_seconds` or `registry_storage_bytes` exceed thresholds.
* **[G-17] Deprecation windows.** CI enforces that any PR marking a surface deprecated includes sunset ≥ 180 days or spans ≥ 2 minor releases, and that `/version`/headers reflect it.

---

## 5) Anti-Scope (Forbidden)

* ❌ No policy evaluation logic (that lives in `ron-policy`).
* ❌ No DHT/provider discovery (that’s discovery/indexing crates).
* ❌ No ledger/economic balances (econ crates handle that).
* ❌ No identity issuance (identity/auth crates handle that).
* ❌ No emergency “maintenance mode” that bypasses quorum. Emergencies use signer rotation and **supersede** commits, never hidden switches.
* ❌ No hidden persistence in amnesia mode. Any local artifact beyond strict runtime scratch (tmpfs) is a violation.
* ❌ No ambient authority or backdoors; all mutations require capabilities + signatures.
* ❌ **No pruning that breaks verifiability.** Compaction must only remove segments **fully covered** by a signed checkpoint.

---

## 6) References

* Canon crate atlas; Twelve Pillars (Governance/Policy, Observability/Hardening).
* Hardening blueprint (service limits, UDS perms, self-test gates).
* Scaling notes (geo/residency, placement maps).
* Six Concerns guide (SEC, GOV, RES, PERF, DX/Interop, COST).
* `ron-proto` (DTOs, schema versions), `ron-auth`/passport (caps), kernel bus events.

---

## 7) Invariants → Concerns → Proofs (cheat table)

| Invariant             | Six Concerns    | Pillar       | Proof Gate(s) |
| --------------------- | --------------- | ------------ | ------------- |
| I-3 Multi-sig         | GOV, SEC        | Governance   | G-1, G-2      |
| I-7 DTO hygiene       | SEC, DX/Interop | Protocols    | G-11          |
| I-8 Hardening         | SEC, RES        | Hardening    | G-5           |
| I-11 Audit chain      | SEC, GOV        | Governance   | G-9           |
| I-12 Signer lifecycle | SEC, GOV        | Governance   | G-10          |
| I-13 Availability     | RES, PERF       | Kernel/Orch. | G-12, G-13    |
| I-14 Amnesia/audit    | SEC, GOV, RES   | Profiles     | G-6, G-12     |
| I-15 Back-pressure    | RES, OBS        | Kernel/Orch. | G-12          |
| I-16 DTO versioning   | DX/Interop      | Protocols    | G-11          |
| I-17 Bounded growth   | COST, RES       | Scaling/Ops  | G-16          |

---

**Definition of Done (for this IDB)**

* Encodes signed-descriptor + multi-sig invariants, signer lifecycle, and tamper-evident chain.
* Maps invariants to Six-Concerns gates; includes amnesia/profile semantics.
* Parametric hardening with CI-verified defaults.
* Fuzz/chaos/trace coverage, retention/compaction, supersede proofs, and bootstrap recovery are explicitly tested.
