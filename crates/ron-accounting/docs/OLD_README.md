# accounting

## 1. Overview
- **What it is:** A lightweight metrics crate for RustyOnions, focused on counting bytes, requests, and rolling snapshots of activity.  
- **Role in RustyOnions:** Provides traffic/accounting data that other services can use for monitoring, logging, or enforcing quotas.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Maintains thread-safe counters (atomic or lock-free).  
  - [x] Provides rolling/ring-buffer style snapshots of activity.  
  - [x] Exposes APIs to increment/decrement usage stats.  
  - [x] Supplies utilities for services to gather metrics periodically.  

- **What this crate does *not* do:**
  - [x] Does not enforce limits (that logic belongs in higher-level services).  
  - [x] Does not export metrics directly over the network (services decide how to expose).  
  - [x] Does not handle persistent storage of metrics.  

---

## 3. APIs / Interfaces
- **Rust API:**
  - `Counter` type with `inc()`, `dec()`, `get()` methods.  
  - `Snapshot` type for ring-buffer summaries.  
  - Utility functions for aggregating stats across tasks.  

---

## 5. Configuration
- **Environment variables:** None.  
- Services may decide to set their own intervals or limits when using `accounting`.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `svc-index`, `svc-overlay`, `svc-storage`, `gateway` (to track requests/bytes).  
  - `ron-kernel` (potential future: aggregate health + load stats).  

- **Downstream (what it depends on):**  
  - Standard library concurrency primitives.  

- **Flow:**  
  ```text
  service (e.g., gateway) → accounting::Counter → accounting::Snapshot → logs/metrics
```