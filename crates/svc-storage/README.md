# svc-storage

## 1. Overview
- **What it is:** A microservice responsible for reading and writing the actual bytes of bundle files.  
- **Role in RustyOnions:** This is the only crate that touches the filesystem for bundle data. Other services (overlay, gateway) never read files directly—they go through `svc-storage`.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Reads files from a given bundle directory.  
  - [x] Writes files into a bundle directory (used by tools/tests).  
  - [x] Responds to RPCs over the `ron-bus` Unix socket.  
  - [x] Provides health checks for supervision by `ron-kernel`.  

- **What this crate does *not* do:**
  - [x] Does not resolve addresses (delegated to `svc-index`).  
  - [x] Does not serve HTTP (that’s `gateway`).  
  - [x] Does not enforce payments or interpret manifests.  

---

## 3. APIs / Interfaces
- **Bus RPC methods:**
  - `v1.health` → `StorageResp::HealthOk`  
  - `v1.read_file { dir, rel }` → `StorageResp::File { bytes } | NotFound | Err { err }`  
  - `v1.write_file { dir, rel, bytes }` → `StorageResp::Written | Err { err }`  

---

## 5. Configuration
- **Environment variables:**  
  - `RON_STORAGE_SOCK` — Path to the service’s Unix Domain Socket (default: `/tmp/ron/svc-storage.sock`).  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `svc-overlay` (for reading bundle files).  
  - Developer tools or tests (may use write RPCs).  

- **Downstream (what it depends on):**  
  - Local filesystem (trusted path input from `svc-index`).  

- **Flow:**  
  ```text
  client → gateway → svc-overlay → svc-index (resolve addr → dir)
                                  ↘ svc-storage (read/write file bytes)
```