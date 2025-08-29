# gateway

## 1. Overview
- **What it is:** The HTTP façade for RustyOnions. It exposes content bundles over standard HTTP.  
- **Role in RustyOnions:** Acts as the public entry point. Instead of reading from the filesystem or database directly, it delegates to `svc-overlay`, which in turn calls `svc-index` and `svc-storage`. This enforces the microkernel design of isolation and fault tolerance.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Provides HTTP routes (`/o/:addr/*tail`) via Axum.  
  - [x] Retrieves bundle files by calling `svc-overlay` through `ron-bus`.  
  - [x] Optionally enforces payment requirements (`402 Payment Required` if `Manifest.toml` specifies).  

- **What this crate does *not* do:**
  - [x] Does not access sled or filesystem directly.  
  - [x] Does not resolve addresses (delegates to `svc-index` through overlay).  
  - [x] Does not read/write bundle bytes (delegates to `svc-storage` through overlay).  

---

## 3. APIs / Interfaces
- **HTTP API:**
  - `GET /o/:addr/*tail`  
    - Returns file bytes from the bundle.  
    - Defaults to `payload.bin` if no tail is specified.  
    - Status codes:  
      - `200 OK` → file returned  
      - `402 Payment Required` → if payments are enforced and required  
      - `404 Not Found` → if address or file missing  
      - `502 Bad Gateway` → if underlying services fail  

- **Rust API (internal only):**  
  - `router(AppState)` — builds the Axum router for embedding in binaries.

---

## 5. Configuration
- **Environment variables:**  
  - `RON_INDEX_SOCK` — Path to `svc-index` UDS (default: `/tmp/ron/svc-index.sock`, used indirectly).  
  - `RON_OVERLAY_SOCK` — Path to `svc-overlay` UDS (default: `/tmp/ron/svc-overlay.sock`).  

- **Command-line flags:**  
  - `--bind <addr:port>` — Bind address for the HTTP server (default: `127.0.0.1:54087`).  
  - `--enforce-payments` — Enforce `[payment].required` manifests by returning `402 Payment Required`.  

---

## 8. Integration
- **Upstream:**  
  - External clients (browsers, curl, apps).  

- **Downstream:**  
  - `svc-overlay` (all fetches delegated here).  
  - `svc-index` (indirectly through overlay for address resolution).  
  - `svc-storage` (indirectly through overlay for file bytes).  

- **Flow:**  
  ```text
  client → gateway (HTTP) → svc-overlay (bus RPC)
                           ↘ svc-index (resolve addr → dir)
                           ↘ svc-storage (read file bytes)
```