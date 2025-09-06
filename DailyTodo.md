# RustyOnions — Status & Next-Step Plan
_Date: 2025-09-05 · Timezone: America/Chicago_

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

- [ ] Gateway: token buckets + `/readyz` gate; 429/503 + Retry-After present.
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
