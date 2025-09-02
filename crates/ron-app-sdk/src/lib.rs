#![forbid(unsafe_code)]
//! ron-app-sdk: Minimal OAP/1 client for RustyOnions overlay.
//! Bronze ring scope: framing, HELLO, single-shot request (REQ|START|END).
//! Streaming APIs are stubbed and will land in Silver.

pub mod errors;
pub mod constants;
pub mod oap;
pub mod client;

pub use client::OverlayClient;
pub use oap::{Hello, OapFlags, OapFrame};
pub use oap::codec::OapCodec;
pub use constants::{OAP_VERSION, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME};
pub use errors::Error;
