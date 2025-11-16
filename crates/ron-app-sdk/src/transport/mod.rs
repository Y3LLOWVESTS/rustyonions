//! RO:WHAT — Transport module root for ron-app-sdk.
//! RO:WHY  — Split the transport adapter into smaller focused files
//!           (handle + HTTP/error mapping) while keeping a stable API.
//! RO:INTERACTS — crate::config, crate::errors, crate::retry; used by
//!                plane modules (storage, edge, mailbox, index).
//! RO:INVARIANTS — client-only; no server/listener code; all outbound
//!                 calls go through `TransportHandle`; OAP frame cap
//!                 re-exported as `OAP_MAX_FRAME_BYTES`.
//! RO:METRICS — metrics are recorded in planes; this module stays
//!              metric-agnostic.
//! RO:CONFIG — reads SdkConfig fields (transport, gateway_addr,
//!             overall_timeout, timeouts, retry) via `TransportHandle`.
//! RO:SECURITY — TLS/Tor configuration only; macaroon capabilities are
//!               handled at higher layers (planes).
//! RO:TEST — unit tests live in submodules; integration tests under
//!           `tests/i_*` exercise invariants end-to-end.

mod handle;
mod mapping;

pub use handle::{TransportHandle, OAP_MAX_FRAME_BYTES};
