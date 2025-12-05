// crates/svc-admin/src/nodes/mod.rs

//! Node registry and HTTP client for talking to RON-CORE nodes.
//!
//! This module is the “admin plane view” of nodes: it knows how to
//! list nodes (from config) and query their basic status over HTTP.

pub mod client;
pub mod registry;
pub mod status;

pub use client::NodeClient;
pub use registry::NodeRegistry;
