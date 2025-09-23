# svc-index

## 1. Overview
- **What it is:** A microservice that maintains the mapping of RustyOnions content addresses to local bundle directories.  
- **Role in RustyOnions:** Provides the directory resolution service for the rest of the system. Both `svc-overlay` and developer tooling (`ronctl`) rely on `svc-index` for consistent address→directory lookups.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Stores address → directory mappings.
  - [x] Responds to RPC requests over the `ron-bus` Unix socket.
  - [x] Provides health checks for kernel supervision.
- **What this crate does *not* do:**
  - [x] Does not serve HTTP (that’s the gateway).  
  - [x] Does not read/write file bytes (delegates to `svc-storage`).  
  - [x] Does not implement overlay protocol (delegates to `svc-overlay`).  

---

## 3. APIs / Interfaces
- **Bus RPC methods:**
  - `v1.health` → `IndexResp::HealthOk`
  - `v1.resolve { addr }` → `IndexResp::Resolved { dir } | NotFound | Err { err }`
  - `v1.put_address { addr, dir }` → `IndexResp::PutOk | Err { err }`

---

## 5. Configuration
- **Environment variables:**  
  - `RON_INDEX_SOCK` — UDS path to bind for the service (default: `/tmp/ron/svc-index.sock`).

---

## 8. Integration
- **Upstream crates/services that call this:**  
  - `svc-overlay` (to resolve bundle directory for an address).  
  - `ronctl` (CLI to test/insert mappings).  
- **Downstream crates/services this depends on:**  
  - None directly — it is the source of truth for mappings.  
- **Typical flow:**  
  ```text
  client → gateway → svc-overlay → svc-index (resolve)
                             ↘ svc-storage (read bytes)
```

