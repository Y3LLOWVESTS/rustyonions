

# 30-day plan to ship the 23 CORE crates (Micronode + Macronode profiles)

## Ground rules (to keep the month timeline real)

* **Scope lock:** CORE only (no Web3/value-plane crates). Keep ryker optional behind a feature flag.
* **One API spine:** `oap` + `ron-proto` DTOs are the single source of truth that all services import.
* **Uniform HTTP stack:** axum 0.7, reqwest 0.12 (rustls).
* **Observability contract everywhere:** `/metrics`, `/healthz`, `/readyz` + Prometheus counters.
* **Bus-first orchestration:** `ron-bus` for fanout; service internals use `tokio::mpsc` mailboxes (ryker later).
* **Bronze DoD now, Silver later:** First month ships Bronze (correct + observable + reproducible + benchable). Perf “diamond” passes can continue in parallel but don’t block Bronze.

## Definition of Done (Bronze) — every crate must check all

* **Compiles** under workspace pins; **unit tests pass**.
* **Endpoints** (if service): `/metrics`, `/healthz`, `/readyz` present and green locally.
* **Config**: loads from env + file; hot-reload emits `KernelEvent::ConfigUpdated`.
* **Logs/Spans**: structured tracing on by default, request IDs propagated.
* **Benches/Smoke**: at least one Criterion bench or example `*_demo` that proves the happy path.
* **Docs**: README with run command + curl smoke steps; crate-level doc comments; TODO tracked.

---

## Week-by-week schedule (single dev; add parallelism if you have help)

### Week 1 — Kernel plane + Protocol/SDK (crates 1–6)

**Goal:** rock-solid core and a typed API surface, so all later crates stitch in cleanly.

1. **ron-kernel**: finalize bus events, health state, config watcher stub.
2. **ron-bus**: bounded broadcast MVP + tests + publish/subscribe examples.
3. **ron-metrics**: exporter with Prometheus registry, counters/histograms helpers.
4. **ron-proto**: DTOs for node, overlay, storage, index; versioned modules.
5. **oap**: request/response envelopes, limits, error codes.
6. **ron-app-sdk**: retry policy, idempotency keys, simple client for gateway.

**Deliverable:** `kernel_demo` + `metrics_demo` run concurrently; SDK calls a fake echo route in gateway (stub).

### Week 2 — IO/Overlay/Discovery (crates 7–9) and Content/Naming (10–12)

**Goal:** secure IO + session layer + minimal discovery, then CAS + name→providers path.

7. **ron-transport**: TCP/TLS server client with timeouts, max conns, idle eviction.
8. **svc-overlay**: sessions/gossip skeleton; node hello, heartbeat, peer table.
9. **svc-dht**: K-bucket table in-memory; GET/PUT provider records (RAM).
10. **svc-storage**: CAS with BLAKE3; PUT/GET blobs; in-mem or file-backed (toggle).
11. **svc-index**: name → content/provider mapping; basic query API.
12. **ron-naming**: typed name IDs, validation, helper codecs.

**Deliverable:** `node_overlay` and `node_index` examples: two processes exchange a small blob via name→index→provider lookup.

### Week 3 — Ingress/Edge (13–14) and Identity chain (15–17)

**Goal:** front door + caching; keys→auth→passport flow working end-to-end.

13. **svc-gateway**: axum HTTP ingress; routes: `/put`, `/get`, `/name/resolve`.
14. **svc-edge**: read-through cache + range requests; local LRU.
15. **ron-kms**: key generation/rotation (in-mem + file keystore options).
16. **ron-auth**: token mint/verify (HMAC/Ed25519), scopes/claims, middleware.
17. **svc-passport**: user/service passport issuance/refresh; minimal profile.

**Deliverable:** `gateway_smoke` script proves: SDK → gateway → index/storage round-trip with auth on.

### Week 4 — Macronode extras (18–21) and profiles (22–23)

**Goal:** policy/audit/registry, then wrap it as single-binary (Micronode) and multi-service (Macronode).

18. **ron-policy**: load signed policy docs; expose `/policy/hash`; enforce basic quotas.
19. **ron-audit**: append-only audit log (file or RAM), export endpoint.
20. **svc-registry**: registry of nodes/services/topology; signed snapshots.
21. **omnigate**: front of services; DRR/quotas; routes to gateway/edge; 429/503 on readiness.
22. **micronode**: single binary wiring everything with amnesia defaults; one `config.toml`.
23. **macronode**: multi-service launcher + compose file(s); omnigate in front; readiness gates.

**Deliverable:**

* `micronode` binary: starts, serves, resolves, stores, retrieves with auth, metrics green.
* `macronode` compose: same flow via omnigate; policy/audit endpoints return sane data.

---

## Daily cadence (to keep velocity)

* **AM (90m):** implement + unit tests
* **Midday (60m):** example + smoke scripts
* **PM (60m):** benches (if applicable) + README pass
* **End (30m):** update NOTES.md (carryover), open next TODOs

---

## Critical path & risks (and how we blunt them)

* **Protocol drift:** lock `ron-proto` early; gate any changes behind `vX_next` modules.
* **TLS gotchas:** use `tokio_rustls::rustls::ServerConfig` everywhere; add a compile-time check macro.
* **Cross-crate version churn:** workspace pins only; forbid wildcard features; run `cargo deny` daily.
* **Observability missing:** template `/metrics|/healthz|/readyz` handlers into each service from Week 1.
* **Perf cliffs:** perf isn’t the blocker for Bronze—capture benches, file issues, move on.

---

## Quick-start build & run helpers

**Build in recommended order**

```
cargo build -p ron-kernel -p ron-bus -p ron-metrics -p ron-proto -p oap -p ron-app-sdk -p ron-transport -p svc-overlay -p svc-dht -p svc-storage -p svc-index -p ron-naming -p svc-gateway -p svc-edge -p ron-kms -p ron-auth -p svc-passport -p ron-policy -p ron-audit -p svc-registry -p omnigate -p micronode -p macronode
```

**Local smoke (Micronode)**

```
cargo run -p micronode
curl -s localhost:9600/readyz
curl -s localhost:9600/metrics | head -n 20
```

**Local smoke (Macronode via omnigate)**

```
cargo run -p macronode
curl -s localhost:8080/readyz
```

---

## What the 2,000 pages of IDB give us (why 1 month is plausible)

* **APIs & invariants are pre-decided** → minimal time lost on design churn.
* **Cross-cutting concerns are standardized** → copy templates for metrics, config, health.
* **Bench & test scaffolds** are patterned → paste + adjust DTOs, not rethink from scratch.
