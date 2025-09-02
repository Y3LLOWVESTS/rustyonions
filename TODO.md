# RustyOnions — Roadmap & TODO
_Last updated: 2025-09-01_

RustyOnions is a general-purpose Web3 backbone. Vest is our first production user to prove the rails. This roadmap tracks both lanes:

- **Lane A (Pilot / Vest)** — finish the data plane and SDK to production credibility.
- **Lane B (Backbone)** — overlay, index, privacy, accounting, and interop.

---

## Progress Snapshot

- ✅ **Omnigate node (svc-omnigate)** online: TLS listener, OAP/1, HELLO, Storage GET (streaming), Mailbox MVP (SEND/RECV/ACK, idempotency).
- ✅ **Admin HTTP**: `/healthz`, `/readyz`, `/metrics` (Hyper 1.x).
- ✅ **TLS CryptoProvider**: explicit aws-lc-rs install at runtime (panic class removed).
- ✅ **Demos**: `scripts/run_tile_demo.sh`, `scripts/run_mailbox_demo.sh` — both green on a fresh machine.
- ✅ **Backpressure & quotas**: global inflight gate + per-tenant token buckets; 429 with retry hint, 503 on overload; `/readyz` reflects load.
- 🧪 **Quota demo**: `scripts/run_quota_demo.sh` shows controlled 429 bursts and stable `/metrics`.
- 🧩 **SDK (ron-app-sdk)**: echo, tiles, mailbox examples over TLS; native certs on client side.

**Vest readiness estimate:** ~78–80% now (after quotas/backpressure).

---

## High-Impact Next Steps (Lane A — Vest Pilot)

> These three deliver the biggest jump to “pilot-ready” and lock the contract.

### A1. Compression guard rails (safety → 413)
- Enforce max decompressed bytes (independent of `max_frame`) and a ratio limit (e.g., 10:1).
- Return `413` with a machine-readable error; add `reject_oversize_total` metric.
- **DoD:** unit tests with synthetic compressed inputs that trigger both caps; metrics show rejections.

### A2. Error taxonomy + JSON envelope (+ `corr_id`)
- Standardize mappings: `400` bad request, `404` not found, `413` too large, `429` over quota, `503` overload.
- Error body: `{ "code": "OVER_QUOTA" | "TOO_LARGE" | "OVERLOAD" | ... , "message": "...", "retryable": true|false, "corr_id": "..." }`.
- **DoD:** golden tests asserting status + body per failure path; `/metrics` counters match.

### A3. SDK retries + env polish
- Respect `Retry-After` on `429/503` with bounded, jittered retries.
- Env defaults: `RON_NODE_URL`, `RON_CAP` (or per-op token); propagate/echo `corr_id`.
- **DoD:** examples succeed through forced 429s; clearer typed errors bubble to callers.

**Impact to Vest readiness after A1–A3:** ~90–95%.

---

## Platform Steps (Lane B — Web3 Backbone)

> Start after A-series or in parallel iff bandwidth allows.

### B1. `ron-proto` (tiny shared crate)
- Single source of truth for OAP/1 constants, status codes, headers, error schema, and test vectors.
- **DoD:** SDK + Omnigate import from here; two hex vectors included and round-trip tested.

### B2. Overlay Alpha (`overlay` + `svc-overlay`)
- Content-addressed GET/PUT by hash; minimal replication; integrity check on read.
- Apply same quotas/backpressure + metrics; add request histograms.
- **DoD:** `scripts/soak_tiles.sh` demonstrates stable latency under light contention.

### B3. Index Alpha (`index` + `svc-index`)
- Namespace/addr mapping API; static federation list for multiple indices.
- **DoD:** CLI lookup path; integration test covers basic publish/resolve.

### B4. Privacy option (`arti_transport`)
- Optional Tor/Arti transport gate; smoke test onion listener.
- **DoD:** compile-time feature; one PUT/GET round-trip over onion.

### B5. Accounting v1 (`accounting`)
- Persistent usage counters; policy-driven quotas; `/readyz` reflects policy.
- **DoD:** per-tenant gauges, policy reload without restart.

---

## Milestones & Acceptance Gates

### M0 — Bootstrap & Demos ✅
- [x] TLS provider explicit; admin HTTP; OAP/1 HELLO.
- [x] Storage GET streaming (64 KiB chunks) with bytes counters.
- [x] Mailbox MVP (SEND/RECV/ACK, idempotency; visibility timeout).
- [x] Demos: tiles + mailbox green; metrics visible.

### M1 — Backpressure & Readiness ✅
- [x] Global inflight gate with 503 and `Retry-After`.
- [x] Per-tenant quotas (tiles/mailbox) with 429 and retry hint.
- [x] `/readyz` overload threshold env-driven; metrics for rejections.

### M2 — Guard Rails & Contract (Pilot-ready core)
- [ ] Compression guard rails (cap + ratio) → 413.
- [ ] Error taxonomy + JSON envelope + `corr_id`.
- [ ] SDK retries honoring `Retry-After`; env polish.
- [ ] Spec file (`specs/oap-1.md`) and two hex vectors.
- [ ] Soak scripts: mailbox and tiles with stable latency, predictable 429s.

### M3 — Backbone Alpha
- [ ] `ron-proto` crate wired; SDK + services depend on it.
- [ ] Overlay Alpha live with integrity checks; request histograms.
- [ ] Index Alpha with basic federation.

### M4 — Privacy & Accounting
- [ ] Arti/Tor transport option behind feature flag; onion smoke test.
- [ ] Accounting v1: persistent metering + policy.

### M5 — Beta & Interop
- [ ] Erasure coding for large assets; partial repair.
- [ ] Interop adapters (IPFS/libp2p bridge, S3-style GET).
- [ ] Amnesia mode; per-node legal banner.

---

## Scripts — Tasks & Enhancements

### `scripts/run_tile_demo.sh`
- [ ] Emit JSON summary of result (CI-friendly).
- [ ] Option: save metrics snapshot after run.

### `scripts/run_mailbox_demo.sh`
- [ ] Emit JSON summary (counts, msg_id, acked=true/false).
- [ ] Optional: inject a retry scenario to demonstrate idempotency.

### `scripts/run_quota_demo.sh`
- [x] Burst harness exercising 429s; `/metrics` snapshot.
- [ ] Parameterize `MAX_INFLIGHT`, `QUOTA_*`, and runtime to allow longer soaks.
- [ ] Add pass/fail thresholds (e.g., 429 rate within expected band).

---

## Metrics & SLOs

- [x] Counters: `requests_total`, `rejected_overload_total`, byte counters (storage GET), `inflight_current`.
- [ ] Histograms: request latency per app_proto (p50/p95/p99).
- [ ] Quota gauges: tokens available per tenant/op.
- [ ] Error-code counters split by `code` label (400/404/413/429/503).
- [ ] Target (localhost dev): p50 < 10ms, p99 < 150ms for small GET/MAILBOX at steady load.

---

## Repo Hygiene

- [x] Moved root scratch/docs into `docs/` and `specs/`; strengthened `.gitignore`.
- [ ] Add `CRATE_INDEX.md` (roles: service/library/tool/experimental with 1–2-line summaries).
- [ ] Per-crate READMEs from template (`docs/templates/Crate_Readme_Template.md`).
- [ ] Optional: `xtask/` to replace heavier shell scripts over time.
- [ ] `[workspace] default-members` → `svc-omnigate`, `ron-app-sdk` for faster default builds.

---

## Developer Notes

- Prefer **script-driven** validation while guard rails and taxonomy land.
- Keep scripts **idempotent** (safe to rerun), return meaningful exit codes, and print short JSON summaries for CI.
- Libraries use **`thiserror`**; binaries use **`anyhow`** at the boundary.

---

## Credits

Acknowledgements to Stevan White, OpenAI’s ChatGPT, and xAI’s Grok for reviews, scaffolds, and testing flows that shaped the current bring-up and demo scripts.

