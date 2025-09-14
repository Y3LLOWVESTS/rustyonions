#![forbid(unsafe_code)]
//! ron-app-sdk: Minimal OAP/1 client for RustyOnions overlay.
//! Bronze ring scope: framing, HELLO, single-shot request (REQ|START|END).
//! Streaming APIs are stubbed and will land in Silver.

pub mod client;
pub mod constants;
pub mod errors;
pub mod oap;

pub use client::OverlayClient;
pub use constants::{DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION};
pub use errors::Error;
pub use oap::codec::OapCodec;
pub use oap::{Hello, OapFlags, OapFrame};
