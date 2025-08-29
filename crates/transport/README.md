# transport

## 1. Overview
- **What it is:** A crate providing network transport abstractions (TCP, Tor, etc.) for RustyOnions services.  
- **Role in RustyOnions:** Supplies pluggable, async-ready transports so that higher-level services (overlay, gateway, etc.) can communicate without binding to a single network backend.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Defines async `Transport` traits for dialing and listening.  
  - [x] Provides TCP-based implementations.  
  - [x] Optionally integrates with Tor (via `arti` or similar).  
  - [x] Exposes retry/backoff utilities for reliable connections.  

- **What this crate does *not* do:**
  - [x] Does not implement service-specific RPC (that’s `ron-bus`).  
  - [x] Does not handle content resolution (that’s `svc-index`).  
  - [x] Does not serve HTTP directly (that’s `gateway`).  

---

## 3. APIs / Interfaces
- **Rust API:**
  - `trait Transport { async fn dial(...); async fn listen(...); }`  
  - `TcpTransport` — TCP implementation of the trait.  
  - Retry helpers (`with_backoff`, etc.).  

---

## 5. Configuration
- **Environment variables:**  
  - `RON_TRANSPORT_BACKOFF` (optional) — Default retry/backoff policy.  
- **Command-line flags:** None directly (but may be exposed through services that embed this crate).  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `overlay` (legacy)  
  - Future: services that need P2P or remote communication.  

- **Downstream (what it depends on):**  
  - `tokio` for async runtime.  
  - `arti` (if Tor support enabled).  

- **Flow:**  
  ```text
  service → transport::Transport (dial/listen) → TCP/Tor/other backend
```