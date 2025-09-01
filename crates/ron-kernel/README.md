# ron-kernel
**The Rusty Onions Network Microkernel**

## 1. Overview
- **What it is:** The supervisor (microkernel) process for RustyOnions. It starts and monitors the core microservices (`svc-index`, `svc-overlay`, `svc-storage`) and ensures they stay healthy.  
- **Role in RustyOnions:** Provides fault tolerance and orchestration. Without `ron-kernel`, each service would have to be managed manually. With it, the system runs as a cohesive unit with automatic restarts and health checks.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Launches core services as child processes.  
  - [x] Monitors their health by calling `v1.health` RPCs.  
  - [x] Restarts services automatically if they crash or become unhealthy.  
  - [x] Logs the status of each service at regular intervals.  

- **What this crate does *not* do:**
  - [x] Does not serve requests directly (all client traffic flows through gateway + services).  
  - [x] Does not enforce business logic like payments or manifests.  
  - [x] Does not replace system init/service managers (it runs inside them).  

---

## 3. APIs / Interfaces
- **No public API.**  
- Provides logging output to stdout/stderr for supervision.  
- Invokes service binaries using `tokio::process::Command`.  

---

## 5. Configuration
- **Environment variables (paths to binaries):**  
  - `RON_SVC_INDEX_BIN` — Path to `svc-index` binary (default: `svc-index`).  
  - `RON_SVC_OVERLAY_BIN` — Path to `svc-overlay` binary (default: `svc-overlay`).  
  - `RON_SVC_STORAGE_BIN` — Path to `svc-storage` binary (default: `svc-storage`).  

- **Environment variables (UDS paths):**  
  - `RON_INDEX_SOCK` — Path to index socket (default: `/tmp/ron/svc-index.sock`).  
  - `RON_OVERLAY_SOCK` — Path to overlay socket (default: `/tmp/ron/svc-overlay.sock`).  
  - `RON_STORAGE_SOCK` — Path to storage socket (default: `/tmp/ron/svc-storage.sock`).  

---

## 8. Integration
- **Upstream (who calls this):**  
  - Ops/developers launching RustyOnions.  

- **Downstream (what it supervises):**  
  - `svc-index`  
  - `svc-overlay`  
  - `svc-storage`  

- **Flow:**  
  ```text
  ron-kernel supervises →
      svc-index (resolve addr → dir)
      svc-overlay (bundle RPCs)
      svc-storage (read/write bytes)
```