# svc-overlay

## 1. Overview
- **What it is:** A microservice that provides the overlay layer for RustyOnions. It resolves bundle addresses through `svc-index` and then fetches the corresponding files from `svc-storage`.  
- **Role in RustyOnions:** Acts as the intermediary between the HTTP-facing `gateway` and the lower-level services (`svc-index` + `svc-storage`). It enforces the microkernel boundary by ensuring that `gateway` never talks to storage or index directly.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Accepts RPC requests for bundle files.  
  - [x] Uses `svc-index` to resolve address → directory.  
  - [x] Uses `svc-storage` to fetch file bytes.  
  - [x] Provides health checks for supervision by `ron-kernel`.  

- **What this crate does *not* do:**
  - [x] Does not serve HTTP (that’s `gateway`).  
  - [x] Does not access the filesystem directly (delegates to `svc-storage`).  
  - [x] Does not enforce payments or read manifests.  

---

## 3. APIs / Interfaces
- **Bus RPC methods:**
  - `v1.health` → `OverlayResp::HealthOk`  
  - `v1.get { addr, rel }` → `OverlayResp::Bytes { data } | NotFound | Err { err }`  

---

## 5. Configuration
- **Environment variables:**  
  - `RON_OVERLAY_SOCK` — Path to the service’s UDS (default: `/tmp/ron/svc-overlay.sock`).  
  - `RON_INDEX_SOCK` — Path to `svc-index` UDS (default: `/tmp/ron/svc-index.sock`).  
  - `RON_STORAGE_SOCK` — Path to `svc-storage` UDS (default: `/tmp/ron/svc-storage.sock`).  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `gateway` (all HTTP requests to `/o/:addr/*tail` are delegated here).  

- **Downstream (what it depends on):**  
  - `svc-index` (for resolving the address → directory).  
  - `svc-storage` (for fetching file bytes).  

- **Flow:**  
  ```text
  client → gateway (HTTP) → svc-overlay (bus RPC)
                           ↘ svc-index (resolve addr → dir)
                           ↘ svc-storage (read file bytes)
```