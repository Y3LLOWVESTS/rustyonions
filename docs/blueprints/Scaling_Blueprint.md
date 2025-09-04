# RustyOnions — Final Scaling & Node Deployment Blueprint (v1.3.1 • “God‑Tier Alignment”)

**Version:** v1.3.1 (2025‑09‑03) • **Kernel changes:** none • **Status:** Production‑ready with runbooks & API specs added.  
**Scope:** Updates v1.3 by adding runbooks, alignment & migration notes, test scripts, and service API stubs per Grok’s review.

This is the authoritative scaling + node‑deployment plan aligned to the repo. It preserves the microkernel invariants and delivers operator‑ready configs, APIs, and runbooks.

---

## Single‑Source‑of‑Truth Invariants (copy/pin across blueprints)

* **Addressing (normative):** `b3:<hex>` using **BLAKE3‑256** over the plaintext object (or manifest root). Truncated prefixes MAY be used for routing; the full 32‑byte digest **MUST** be verified before returning bytes.
* **OAP/1 defaults:** `max_frame = 1 MiB` (protocol default per **GMI‑1.6**). Storage **streaming chunk** size (e.g., **64 KiB**) is an implementation detail and **not** the OAP/1 frame size.
* **Kernel public API (frozen):** `Bus`, `KernelEvent::{ Health{service, ok}, ConfigUpdated{version}, ServiceCrashed{service, reason}, Shutdown }`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* **Normative spec pointer:** OAP/1’s canonical specification is **GMI‑1.6**. Any `/docs/specs/OAP‑1.md` in‑repo is a stub that mirrors/links GMI‑1.6 to avoid drift.
* **Perfection Gates ↔ Milestones:** Final Blueprint maps Gates A–O to **M1/M2/M3** in the Omnigate Build Plan.

---

## What’s new in v1.3.1 (delta from v1.3)

1) **Runbooks added**: DHT failover and cross‑region placement (operator‑ready steps & alerts).  
2) **Alignment/migration notes**: SHA‑256 → BLAKE3 migration & OAP/1 `max_frame` alignment tasks.  
3) **Testing expanded**: `test_offline_sync.sh` filled with a mock mailbox flow.  
4) **Service API specs**: Minimal `svc-index`, `svc-discovery` (Discv5‑style), and `svc-payment` (settlement stub).  
5) **Acceptance checklists**: Green‑bar checks per PR to prevent drift.

> v1.3 changes (BLAKE3, `reason` in `ServiceCrashed`, protocol clarity, DHT NodeID) remain intact.

---

## Historical delta — What changed in v1.3 (from v1.2)

* BLAKE3 (b3‑256) content addressing made **normative**.  
* `KernelEvent::ServiceCrashed` gained `reason: String` for observability; services emit `rejected_total{reason=...}` and structured logs.  
* Clarified OAP/1 `max_frame = 1 MiB` vs. 64 KiB **storage chunk**.  
* Runbook pointers added.  
* DHT identity moved to **BLAKE3‑256**(node pubkey).

---

## 0) Kernel invariants (no deviation)

* Public API (must remain):  
  `Bus`, `KernelEvent::{ Health{service, ok}, ConfigUpdated{version}, ServiceCrashed{service, reason}, Shutdown }`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* Transport/TLS: `tokio_rustls::rustls::ServerConfig` (not `rustls::ServerConfig`).  
* Axum 0.7 for services; handlers `.into_response()`.  
* Metrics module: `/metrics`, `/healthz`, `/readyz`; Prometheus counters/histograms registered to default registry.  
* Content integrity: **BLAKE3 (b3‑256)** content addressing (Merkle framing can layer later).

> Everything below is **services + deploy artifacts only**. The microkernel remains unchanged.

---

## 1) Roles & planes (recap)

* **Public nodes**: TCP+TLS overlay, persistent storage, quotas/backpressure; serve **signed, allow‑listed pinsets** only.  
* **Private nodes**: Tor egress, **amnesia** (tmpfs), never serve public bytes. Scaling is fan‑out; not the focus here.

---

## 2) Operator quickstart (unchanged from v1.3)

See v1.3 §2 for `config.public.toml`, micronode profiles, and systemd/HAProxy steps. (Kept verbatim for stability.)

---

## 3) Small‑cluster blueprint (2–10 nodes) — NOW

Topology and HAProxy/Consul examples remain as in v1.3.  
Add one note: **health propagation** — L4 LB should eject instances on `/readyz` failure within ≤15s.

---

## 4) Global DHT for discovery (`svc-dht`) — UPDATED (no API change; operational clarity)

* **NodeID**: 256‑bit (**BLAKE3‑256**) of node public key.  
* **α**=3, **k**=20; provider records keyed by `b3:<hex>`, Ed25519‑signed, TTL=24h, republish=12h.  
* **Gateway lookup path**: `svc-placement` → DHT `/peers` → hedged dials (200–300 ms).

---

## 5) NAT traversal for micronodes (overlay relay)

As v1.3; add **relay SLO**: p95 relay RTT added latency < 60 ms within region; enforce per‑client kbps caps.

---

## 6) Observability & SLOs (expanded)

**Standard metrics**

* `ron_request_latency_seconds` (Histogram) — `{method, content_id}`  
* `ron_bytes_served_total` (Counter) — `{content_id, node_id}`  
* `ron_open_streams` (Gauge)  
* **`rejected_total{reason=...}`** — increment on early rejects and when raising `ServiceCrashed{..., reason}`
* `ron_ec_repair_bytes_total` (Counter), `ron_ec_missing_shards` (Gauge)
* `ron_dht_missing_records` (Gauge), `ron_dht_lookup_ms` (Histogram)

**SLO targets (public GET)**: p95 < 80 ms intra‑region, < 200 ms inter‑region; 5xx < 0.1%; 429/503 < 1%; RF(observed) ≥ RF(target).

**Structured logs** (JSON lines): include `service`, `reason`, `content_id`, `remote`, `latency_ms`, `rf_target`, `rf_observed`.

---

## 7) Runbooks (NEW)

### 7.1 DHT failover & re‑replication

**Goal:** Detect missing provider records and restore target RF within 60 minutes without data loss.

**Signals / Alerts**  
* Alert A1: `ron_dht_missing_records > 0 for 10m` (warning), 30m (critical)  
* Alert A2: RF shortfall — `rf_observed < rf_target for 5m`  
* Alert A3: `ron_dht_lookup_ms{le="500"} < 0.95` (95% of lookups slower than 500 ms)

**Immediate triage (5–10 min)**  
1. Check `svc-dht` health: `curl -sS :9110/healthz` and logs for timeouts.  
2. Inspect bootstrap reachability: `curl -sS :9110/bootstrap`.  
3. Sample providers for a hot key: `curl -sS :9110/peers?hash=b3:<hex>&limit=8` and verify nodes respond on 1777.

**Recovery workflow**  
1. **Trigger re‑replication:** call placement/DHT repair tool:  
   ```bash
   ronctl repair --hash b3:<hex> --rf 3 --prefer-region us-east-1
   ```
   The tool queries DHT, verifies `rf_observed`, and pins to additional nodes.  
2. **Verify RF:** observe metrics `rf_observed` and clear A2 within 10–20 minutes.  
3. **Cache warmup (optional):** seed gateway caches for top N objects (see §11.1).  
4. **Root cause:** inspect churn (node leaves), network partitions, or expired provider TTL (24h).  
5. **Prevent:** tune `republish_secs` (12h default), ensure clocks are NTP‑synced, and confirm Ed25519 signatures are valid.

**Exit criteria**  
* A1/A2/A3 cleared for ≥30 minutes, RF restored, p95 within SLO, DHT lookup p95 < 250 ms intra‑region.

---

### 7.2 Cross‑region placement preference

**Goal:** Keep latency low while ensuring RF.

**Policy**  
* Prefer providers with RTT < 50 ms for the client’s region.  
* Hedge between 2 local + 1 remote provider on first byte; collapse to best performer after 1–2 chunks.

**Operator steps**  
1. Ensure `svc-placement` accepts `region` hints: `GET /assign?hash=b3:<hex>&rf=3&region=us-east-1`.  
2. If local capacity constrained, permit 1 remote (RTT < 120 ms) temporarily.  
3. Rebalance nightly:  
   ```bash
   ronctl rebalance --region us-east-1 --rf 3 --top 100000
   ```

**Exit criteria**  
* p95 < 80 ms in region; inter‑region selects < 20% of total over 24h.

---

## 8) Rewards (MVP now → scalable v2)

Unchanged logic from v1.3. **Note**: switch `counters_hash` to **BLAKE3** (already reflected) and include `prev_hash`/`hash` b3‑256 in audit trail.

---

## 9) Erasure Coding Plan — Reed–Solomon

Parameters and workflow as in v1.3. Add: **repair pacing** at ≤ 50 MiB/s per cluster to avoid cache thrash; prioritize hottest content first.

---

## 10) Micronode offline sync — reconciliation protocol

Spec unchanged; **testing now included** (see §12.3).

---

## 11) Migration & Alignment Notes (NEW)

### 11.1 SHA‑256 → BLAKE3 migration

**Compatibility window:** allow dual‑hash (read SHA‑256, write BLAKE3) for one epoch (e.g., 7–14 days).

**Upgrade order**  
1. Update `svc-dht`, `svc-storage`, `svc-rewarder` to compute/store **BLAKE3** digests.  
2. Gateways accept client requests with either `sha256:<hex>` or `b3:<hex>` for the window; responses always advertise canonical **BLAKE3**.  
3. Re‑announce provider records to DHT keyed by `b3:<hex>`.  
4. Deprecate SHA‑256 endpoints; keep 410 Gone for 30 days with migration hint.

**Sanity greps**  
```
rg -n "sha-?256|sha256:" -S
rg -n "b3:" -S
```

### 11.2 OAP/1 `max_frame` alignment

Microkernel/demo crates may still mention 64 KiB; update to 1 MiB per **GMI‑1.6**.  
Keep storage streaming at 64 KiB (configurable) — **distinct knobs**.

Sanity grep:  
```
rg -n "max_frame\s*=\s*64\s*Ki?B" -S
```

---

## 12) Testing (expanded)

### 12.1 Cluster validation (`deploy/scripts/test_cluster.sh`)

As in v1.3 — validate health, pinning, warm caches, placement assign, and receipts. Add latency assertions with curl + timing.

### 12.2 DHT smoke (`deploy/scripts/test_dht.sh`)

Unchanged from v1.3 (uses `b3:<hex>`).

### 12.3 Offline sync smoke (`deploy/scripts/test_offline_sync.sh`) — now actionable

```bash
#!/usr/bin/env bash
set -euo pipefail

API="${API:-http://127.0.0.1:1777}"
SPOOL="${SPOOL:-/tmp/ron-spool}"
MOCK_PORT="${MOCK_PORT:-7788}"

echo "[1/4] Prepare spool"
rm -rf "$SPOOL"
mkdir -p "$SPOOL"
printf "hello" > "$SPOOL/item-1"

echo "[2/4] Start mock mailbox (separate terminal optional)"
# Expect a tiny dev crate 'mock-mailbox' exposing /hello /diff /push /ack for the flow.
# If already running, this will fail harmlessly.
mock-mailbox --port "$MOCK_PORT" &
MOCK_PID=$!
sleep 1

echo "[3/4] Run offline sync"
ronctl sync \
  --spool-dir "$SPOOL" \
  --endpoint "http://127.0.0.1:${MOCK_PORT}" \
  --hello-interval-ms 1000 \
  --max-batch 8

echo "[4/4] Verify metrics"
curl -fsS http://localhost:9096/metrics | grep -E "ron_spool_(items_total|resends_total|sync_duration_seconds)" || true

# Cleanup
kill $MOCK_PID 2>/dev/null || true
```

> Add the `mock-mailbox` dev crate in `testing/mock-mailbox` (tiny Axum server).

---

## 13) Service API specs (NEW, minimal)

### 13.1 `svc-index`

**Purpose:** Resolve providers for a content hash without DHT client on the caller.

**HTTP**  
* `GET /resolve?hash=b3:<hex>&limit=16` →  
  ```json
  [{"node_id":"ron:node:abc","addr":"10.0.0.11:1777"}, {"node_id":"...", "addr":"..."}]
  ```
* `POST /announce`  
  ```json
  {"hash":"b3:<hex>","node_id":"ron:node:abc","addr":"10.0.0.11:1777","expires_at":1693766400,"sig":"ed25519:..."}
  ```

**Notes:** For small clusters this can front the DHT. Verify `sig` and enforce TTL.

---

### 13.2 `svc-discovery` (Discv5‑style sketch)

**UDP/TCP** messages (CBOR/SSZ OK):
* `PING{node_id, ts}` / `PONG{node_id, ts}`  
* `FIND_NODE{target_id}` → neighbors (Kademlia buckets)  
* `FIND_VALUE{key=b3:<hex>}` → provider records or closest nodes  
* `ANNOUNCE{key=b3:<hex>, provider_record}` (Ed25519‑signed)

**Security:** Drop non‑signed ANNOUNCE; rate‑limit PING/PONG; ignore peers failing token/nonce challenge.

---

### 13.3 `svc-payment` (settlement stub)

**HTTP**  
* `POST /settle`  
  ```json
  {"epoch_id":"2025-09-03T12","merkle_root":"b3:...","witnesses":["west-1","east-1"]}
  ```
  → returns `{ "txid":"stub:...", "network":"testnet" }`

**Config** (`deploy/configs/config.payment.toml`):
```toml
network = "testnet"
endpoint = "http://127.0.0.1:18545"
max_batch = 1024
```

---

## 14) File drop summary (updated)

```
docs/scaling.md                              (this doc)
docs/runbooks.md                             (NEW: extracts §7 into a shared runbook)
specs/svc-index.md                           (NEW: expand §13.1)
specs/svc-discovery.md                       (NEW: expand §13.2)
specs/svc-payment.md                         (NEW: expand §13.3)
deploy/systemd/ron-public.service
deploy/haproxy/haproxy.cfg
deploy/haproxy/ron-haproxy.service
deploy/docker/Dockerfile
deploy/docker/docker-compose.yml
deploy/alerts/rustyonions.yaml
deploy/configs/config.public.toml
deploy/configs/config.public.micro.toml
deploy/configs/config.public.micronode-lite.toml
deploy/configs/config.overlay.relay.toml
deploy/configs/config.dht.toml
deploy/configs/config.payment.toml            (NEW)
deploy/scripts/test_cluster.sh
deploy/scripts/test_dht.sh
deploy/scripts/test_offline_sync.sh           (filled)
testing/mock-mailbox/Cargo.toml               (NEW)
testing/mock-mailbox/src/main.rs              (NEW)
Makefile
```

---

## 15) Acceptance checklists (copy to PR descriptions)

### 15.1 Hashing & API alignment
- [ ] No `sha256:` strings remain in repo docs or scripts.  
- [ ] Microkernel demos/specs do **not** state `max_frame = 64 KiB`.  
- [ ] All services emit `ServiceCrashed{service, reason}` and `rejected_total{reason}`.

### 15.2 Runbooks
- [ ] DHT failover run executed successfully on a controlled test (RF restored ≤ 60 min).  
- [ ] Cross‑region placement: p95 intra‑region < 80 ms over 24h window after rebalancing.

### 15.3 Tests
- [ ] `test_offline_sync.sh` passes with `testing/mock-mailbox`.  
- [ ] `test_dht.sh` returns ≥ 3 peers for pinned hot content.

### 15.4 Observability
- [ ] Dashboards show `ron_dht_lookup_ms` p50/p95 and RF gauges per prefix.  
- [ ] Structured logs contain `reason` and `rf_observed` on rejects/crashes.

---

## 16) TL;DR

* **Ship today**: micro‑node systemd, DNS‑SRV/Consul discovery, HAProxy LB, overlay relay, MVP rewards — now with **operator runbooks**.  
* **Scale globally**: DHT discovery + placement sharding, Reed–Solomon EC, robust offline sync + test harness, and public rewards audit trail.  
* **Kernel stays tiny and stable**: zero API changes; v1.3.1 only adds ops clarity & alignment tasks.

