# OMNIGATE BUILD PLAN — Bronze/Silver/Gold (GMI-1.6)
**Scope locked:** Omnigate blueprint **unchanged**. This plan sequences delivery, acceptance gates, and engineering tickets to reach a Vest‑ready “very good” standard fast, then iterate — without scope creep.

**Principles:** 
- No app logic in kernel. 
- No QUIC/PQ/ZK/Intents in the pilot (hooks only). 
- Only the rails: OAP/1, Rust SDK, Gateway quotas/readiness, Mailbox (at‑least‑once), Storage GET streaming.
- **Observability mapping:** `ServiceCrashed{service, reason}` MUST populate `rejected_total{reason=...}` and structured logs for postmortems.
- **ZK hooks (scope):** Feature-gated and **service-layer** only (`ledger`/`svc-rewarder`); kernel stays auth/token-agnostic.


### ZK Hooks — Roadmap (M1 → M3)

- **M1 (Enable hooks, no proofs in hot path):**
  - Add feature-gated traits in `ledger` for commitments/proofs (`UsageCommitment`, `ZkProver`, `ZkVerifier`).
  - Extend settlement records with optional fields: `usage_commitment`, `usage_proof` (opaque bytes).
  - Document privacy rationale in `docs/security.md` (commit-only mode); keep default **off**.
  - _Accept:_ Compiles behind feature flag; integration tests pass with commit-only mode.

- **M2 (Commitments + basic proofs):**
  - Implement commitment scheme and proof-of-sum / range checks in `svc-rewarder` (e.g., Bulletproofs or IPA-based).
  - Add verifier path and metrics: `zk_verify_total`, `zk_verify_failures_total`.
  - Bench offline prover cost; ensure proof verification stays off hot data path.
  - _Accept:_ End-to-end demo: tenant usage → commitment → proof → verifier **OK**; failure path logged and metered.

- **M3 (Productionization):**
  - Harden circuits/policies (quota conformance, per-tenant bounds) and rotate parameters if scheme requires it.
  - Add governance note in `docs/GOVERNANCE.md` for ceremony/parameter custody (if applicable).
  - Add TLA+ sketches for rewarder flows (`specs/rewarder.tla`) and mailbox (`specs/mailbox.tla`).
  - _Accept:_ Load test with ZK verification enabled; error budget intact; docs shipped.
- **Performance simulation:** Add `testing/performance` rig for OAP/1 throughput & DHT scalability; include ZK verifier on/off runs.


---

## 0) Milestone Overview (Rings)

- **M1 (Bronze): “Hello Any App”**
  - Deliver: OAP/1, Rust SDK, Gateway quotas + /readyz, Mailbox (at‑least‑once), metrics, fuzz/property tests, registry.
  - DoD: External app can REQ→RESP with caps; echo example OK; overload yields 429/503; no OOM; metrics present.

- **M2 (Silver): “Useful Substrate”**
  - Deliver: Storage/DHT GET/HAS + streaming (tiles), Mailbox polish (ACK levels, visibility timeout, DLQ), SDK DX polish, revocation v1.
  - DoD: Vest demo: tiles + mailbox E2E with backpressure; latency targets met intra‑AZ.

- **M3 (Gold): “Ops‑Ready & Smooth Growth”**
  - Deliver: Parser proptests in CI, privacy/leakage harness, revocation diffs, governance SLA, polyglot planning (no blueprint changes).
  - DoD: Multi‑tenant load test passes; docs allow a new dev to integrate in < 30 minutes.

---

## 1) Workstreams & Ownership
- **Core**: OAP/1 spec, overlay/protocol, Rust SDK, examples.
- **Services**: Mailbox, Storage (read path), Index (optional stub).
- **Ops**: Gateway quotas, readiness gating, metrics/alerts.
- **Security**: Cap tokens (macaroons v1), fuzzing/property tests, red‑team suite.
- **Docs/DX**: specs/oap‑1.md, SDK guides, registry process, examples, `ronctl` UX.

---

## 2) Timeline (suggested, adjust to team size)
Weeks are illustrative; parallelize where safe.

### Weeks 1–2 (M1 kickoff)
- Finalize `specs/oap-1.md` + HELLO probe + reject taxonomy.
- Implement `overlay/protocol.rs`: frame parsing/serialization, bounds checks, flags, codes.
- Start `crates/ron-app-sdk`: connect/retry/deadlines/backpressure/idempotency; echo client.
- Add `/readyz` capacity gating to gateway; token buckets per tenant/app_id.
- Metrics: `requests_total, bytes_{in,out}_total, rejected_total{reason}, latency_seconds, inflight, quota_exhaustions_total, bus_overflow_dropped_total`.
- Fuzz harness skeleton + property tests for frame parser.

### Weeks 3–4
- Mailbox (M1): SEND/RECV/ACK/SUBSCRIBE/DELETE; at‑least‑once; ULID message_id; idempotency key support; best‑effort FIFO per topic.
- SDK streaming API; APP_E2E flag pass‑through (opaque to kernel).
- Compression guard rails: configurable zstd ratio (default 10:1) + hard decompressed byte cap; errors → 413.
- Registry: `REGISTRY.md` + JSON mirror (signed PR process).

### Weeks 5–6
- Red‑team suite: malformed frames, slow‑loris, partial writes, quota storms, compression bombs.
- Soak test (24h) on echo+mailbox: availability ≥ 99.99%, zero FD leaks.
- Docs: Quickstart, SDK guide, example walkthroughs.
- Bronze ring sign‑off; tag `v0.1.0-bronze`.

### Weeks 7–10 (M2)
- Storage/DHT (read‑path first): GET, HAS, **64 KiB streaming chunk** (implementation detail, not OAP/1 max_frame); tileserver example.
- Mailbox polish: ACK levels (auto/explicit), visibility timeout, DLQ.
- SDK DX: env (`RON_NODE_URL`, `RON_CAP`), nicer errors, `corr_id` tracing.
- Capability distribution v1: short‑TTL macaroons + `ronctl cap mint/rotate`; HELLO returns revocation sequence.
- Vest demo E2E; latency tuning; tag `v0.2.0-silver`.

### Weeks 11–14 (M3)
- Parser proptests integrated in CI; fuzz corpus persisted and replayed.
- Leakage harness v1 (timing/size correlation across planes); docs + toggles (padding/jitter).
- Governance SLA + appeal path for registry; privacy dashboards (DP off by default).
- Tag `v0.3.0-gold` and publish docs site snapshot.

---

## 3) Ticket Backlog (paste into tracker)

### M1 — Core
- **M1-C01** — Write `specs/oap-1.md` (normative): header, flags, HELLO, codes, parsing rules, test vectors.  
  _Path:_ `specs/oap-1.md` • _Owner:_ Core • _Est:_ 3d • _Deps:_ —  
  _Accept:_ Spec builds; examples render; vectors load in tests.

- **M1-C02** — Implement frame parser/ser (`overlay/protocol.rs`) with bounds checks & errors.  
  _Path:_ `crates/ron-kernel/src/overlay/protocol.rs` • _Owner:_ Core • _Est:_ 4d • _Deps:_ C01  
  _Accept:_ Unit tests pass; proptests 1k cases green; fuzz 4h no crash.

- **M1-C03** — Add HELLO probe handler (app_proto_id=0).  
  _Path:_ `crates/ron-kernel/src/overlay/service.rs` • _Owner:_ Core • _Est:_ 1d • _Deps:_ C02  
  _Accept:_ Returns `{max_frame,max_inflight,supported_flags,version}`; round‑trip test green.

- **M1-C04** — Rust SDK minimal client: connect/retries/deadlines/backpressure/idempotency.  
  _Path:_ `crates/ron-app-sdk/` • _Owner:_ Core • _Est:_ 5d • _Deps:_ C02  
  _Accept:_ Echo client/server examples pass under TLS and `APP_E2E` opaque path.

### M1 — Services & Ops
- **M1-S01** — Mailbox MVP (at‑least‑once): SEND/RECV/ACK/SUBSCRIBE/DELETE; ULID; idempotency.  
  _Path:_ `crates/mailbox/` • _Owner:_ Services • _Est:_ 6d • _Deps:_ C02  
  _Accept:_ Integration tests pass; replays deduped; best‑effort FIFO documented.

- **M1-O01** — Gateway quotas + capacity `/readyz` (503 gating) + Retry‑After (1–5s).  
  _Path:_ `crates/gateway/` • _Owner:_ Ops • _Est:_ 4d • _Deps:_ C02  
  _Accept:_ Load test shows graceful 429/503; no OOM; metrics increment correctly.

- **M1-O02** — Metrics plumbing (all golden metrics).  
  _Paths:_ gateway, mailbox, overlay • _Owner:_ Ops • _Est:_ 2d • _Deps:_ O1  
  _Accept:_ Metric names/labels present and scrape cleanly.

### M1 — Security/QA
- **M1-Q01** — Compression safety: zstd ratio limit + hard output cap.  
  _Path:_ overlay/gateway • _Owner:_ Security • _Est:_ 2d • _Deps:_ C02  
  _Accept:_ Bomb corpus rejected as 413; observed ratio metrics exported.

- **M1-Q02** — Fuzz/property tests for parser; red‑team harness.  
  _Path:_ `testing/fuzz/`, `crates/ron-kernel/tests/` • _Owner:_ Security • _Est:_ 5d • _Deps:_ C02  
  _Accept:_ 8h fuzz no crash; proptests green; red‑team suite all correct `4xx/5xx`.

- **M1-G01** — Registry process + JSON mirror; signed PR checks.  
  _Path:_ `REGISTRY.md`, `registry/app_proto_ids.json` • _Owner:_ Docs • _Est:_ 1d  
  _Accept:_ CI validates schema; sample entry for Vest merged.

### M2 — Silver
- **M2-S01** — Storage read‑path: GET, HAS, 64 KiB streaming; tileserver example.  
  _Path:_ `crates/storage/`, `examples/storage_tileserver.rs` • _Owner:_ Services • _Est:_ 8d • _Deps:_ M1  
  _Accept:_ Vest demo pulls tiles over OAP; throughput & latency logged.

- **M2-S02** — Mailbox polish: ACK levels, visibility timeout, DLQ.  
  _Path:_ `crates/mailbox/` • _Owner:_ Services • _Est:_ 5d • _Deps:_ M1  
  _Accept:_ Reliability tests pass; DLQ shows expected messages.

- **M2-C05** — SDK DX polish: env keys, `corr_id` tracing, friendly errors.  
  _Path:_ `crates/ron-app-sdk/` • _Owner:_ Core • _Est:_ 3d • _Deps:_ M1  
  _Accept:_ Docs updated; examples show traces; errors mapped to OAP codes.

- **M2-Q03** — Capability rotation v1 + `ronctl cap mint/rotate`; HELLO revocation sequence.  
  _Path:_ `tools/ronctl/` • _Owner:_ Security • _Est:_ 4d • _Deps:_ M1  
  _Accept:_ Tokens rotate; clients refresh; audit logs show rotation events.

### M3 — Gold
- **M3-Q04** — Parser proptests in CI; fuzz corpus persistence.  
  _Path:_ `.github/workflows/ci.yml`, testing • _Owner:_ Security • _Est:_ 2d • _Deps:_ M1  
  _Accept:_ CI fails fast on regressions; corpus reused.

- **M3-P01** — Leakage harness v1 + docs/toggles (padding/jitter).  
  _Path:_ `testing/leakage/` • _Owner:_ Privacy • _Est:_ 5d • _Deps:_ M1  
  _Accept:_ Report shows measured leakage; AUC target documented; toggles wired.

- **M3-G02** — Registry SLA & appeal path; governance doc.  
  _Path:_ `docs/GOVERNANCE.md` • _Owner:_ Docs • _Est:_ 1d • _Deps:_ M1  
  _Accept:_ Process published; dry‑run with 10+ apps.

---

## 4) Acceptance Gates (mapped)

- **Bronze (M1)** → PERFECTION_GATE items A1–A3, B4–B9, C10–C12, D13–D14 **green**.  
- **Silver (M2)** → strengthens B4/B7, adds storage throughput targets, mailbox reliability.  
- **Gold (M3)** → adds CI proptests/fuzz reuse, leakage harness, governance SLA.

---

## 5) “Definition of Ready/Done” for Vest Pilot

**Ready**: M1 complete; echo + mailbox soak tests pass; quotas & readiness verified; docs live.  
**Done**: Vest pulls tiles via OAP; runs E2E mailbox chat; under overload sees 429/503 (not hangs); no plaintext leaks with `APP_E2E`.

---

## 6) Scope Guardrails (Kill Switches)
- Any change that alters OAP/1 header/fields, adds QUIC/PQ/ZK/Intents, or slips app logic into kernel/services → **Defer** to OAP/2/R&D.  
- Any request to “just add Vest behavior” → **Implement via app_proto_id + SDK**, not kernel.

---

## 7) Repo Touchpoints (to wire tickets)
```
/specs/oap-1.md
/REGISTRY.md
/registry/app_proto_ids.json
/crates/ron-app-sdk/...
/crates/ron-kernel/src/overlay/{protocol.rs,service.rs}
/crates/gateway/...
/crates/mailbox/...
/crates/storage/...
/examples/{oap_echo_server.rs,oap_echo_client.rs,storage_tileserver.rs}
/testing/{fuzz,leakage,load}
/docs/{Quickstart.md,SDK_Guide.md,GOVERNANCE.md}
/tools/ronctl/...
```
 Ensure `ServiceCrashed{service, reason}` increments `rejected_total{reason=...}` and emits structured logs.