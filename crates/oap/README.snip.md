# OAP (Overlay Access Protocol)

This crate provides the OAP/1 envelope types and a safe Tokio codec:

- **1 MiB max frame** (protocol invariant)
- Optional **zstd** bounded inflate (≤ **8×** frame cap)
- Strict `START` capability handling
- Minimal **HELLO** helpers (app_proto_id=0)

Networking and policy live above (gateway/SDK). This crate is pure framing.
