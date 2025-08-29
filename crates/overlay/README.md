# overlay

## 1. Overview
- **What it is:** The legacy overlay crate implementing early versions of the RustyOnions overlay protocol (bundle PUT/GET, storage, and protocol handling).  
- **Role in RustyOnions:** This crate predates the service-split architecture. It now serves mainly as a reference and staging ground for code that has been or is being migrated into `svc-overlay` and `svc-storage`.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Defines the original overlay protocol handlers.  
  - [x] Contains logic for PUT/GET of bundles.  
  - [x] Implements sled-backed storage (legacy).  
  - [x] Provides types and error handling used in earlier integration tests.  

- **What this crate does *not* do:**
  - [x] Does not conform to the new microkernel bus-first model (that’s `svc-overlay` + `svc-storage`).  
  - [x] Does not provide long-term storage strategy (sled is being phased out).  
  - [x] Should not be used by new services.  

---

## 3. APIs / Interfaces
- **Rust API (legacy):**
  - Protocol handlers (`put`, `get`) for bundle transfers.  
  - Store module wrapping sled for persistence.  
  - Error types for overlay protocol failures.  

---

## 5. Configuration
- **Environment variables:**  
  - Historically supported config for sled DB paths.  
  - Now superseded by `RON_STORAGE_SOCK` and `RON_INDEX_SOCK` in the service crates.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - Previously used by `gateway` directly.  
  - Still linked in for backwards-compatibility and reference.  

- **Downstream (what it depends on):**  
  - `sled` for local DB.  
  - `tokio` for async handlers.  

- **Flow (legacy):**  
  ```text
  client → gateway → overlay (direct sled + protocol handling)
```