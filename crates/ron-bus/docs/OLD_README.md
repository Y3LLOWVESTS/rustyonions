# ron-bus

## 1. Overview
- **What it is:** The inter-process communication (IPC) layer for RustyOnions, built on Unix Domain Sockets with MessagePack serialization.  
- **Role in RustyOnions:** Provides the common protocol (`Envelope`) that all services (`svc-index`, `svc-overlay`, `svc-storage`, etc.) use to communicate. This enforces isolation and decoupling in the microkernel design.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Defines the `Envelope` struct for all bus messages.  
  - [x] Provides common RPC request/response enums (`IndexReq`, `IndexResp`, `OverlayReq`, etc.).  
  - [x] Handles socket framing (length-prefix + MessagePack).  
  - [x] Provides helpers (`listen`, `send`, `recv`) for services to use.  

- **What this crate does *not* do:**
  - [x] Does not implement service logic (e.g., resolve, storage, overlay).  
  - [x] Does not manage processes (that’s `ron-kernel`).  
  - [x] Does not provide HTTP (that’s `gateway`).  

---

## 3. APIs / Interfaces
- **Rust API:**
  - `Envelope { service, method, corr_id, token, payload }`  
  - Request/Response enums:  
    - `IndexReq` / `IndexResp`  
    - `StorageReq` / `StorageResp`  
    - `OverlayReq` / `OverlayResp`  
  - `CapClaims` struct for capability tokens (future use).  

- **Bus framing (over UDS):**
  - Frame = 4-byte big-endian length + MessagePack-encoded `Envelope`.

- **Helper functions (`uds.rs`):**
  - `listen(path: &str)` — Bind a new Unix socket.  
  - `recv(stream: &mut UnixStream)` — Receive one `Envelope`.  
  - `send(stream: &mut UnixStream, env: &Envelope)` — Send one `Envelope`.  

---

## 5. Configuration
- **Environment variables:** None directly.  
- Socket paths are chosen per-service (`RON_INDEX_SOCK`, `RON_OVERLAY_SOCK`, etc.), not by `ron-bus` itself.  

---

## 8. Integration
- **Upstream (who uses it):**  
  - All RustyOnions services: `svc-index`, `svc-overlay`, `svc-storage`, `ron-kernel`, `gateway`, and tools like `ronctl`.  

- **Downstream (what it depends on):**  
  - OS-provided Unix Domain Sockets.  
  - `rmp-serde` for MessagePack encoding/decoding.  

- **Flow:**  
  ```text
  service A → ron-bus (uds.rs framing) → Unix socket → ron-bus (decode) → service B
```