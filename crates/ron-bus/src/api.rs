// crates/ron-bus/src/api.rs
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Generic bus envelope exchanged between services.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope {
    pub service: String,  // e.g., "svc.index"
    pub method: String,   // e.g., "v1.resolve"
    pub corr_id: u64,     // correlation id for RPC
    pub token: Vec<u8>,   // capability blob (MsgPack<CapClaims> or empty)
    pub payload: Vec<u8>, // method-specific bytes (MessagePack-encoded)
}

/// Simple status reply, common across services.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub ok: bool,
    pub message: String,
}

/// RPCs for svc-index (requests)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexReq {
    Health,
    Resolve { addr: String },
    PutAddress { addr: String, dir: String },
}

/// RPCs for svc-index (responses)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexResp {
    HealthOk,
    Resolved { dir: String },
    PutOk,
    NotFound,
    Err { err: String },
}

/// RPCs for svc-storage (requests)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageReq {
    Health,
    /// Read a file from a directory (both absolute or canonical within data root).
    ReadFile {
        dir: String,
        rel: String,
    },
    /// Write a file (not used by gateway yet, but handy for tests/tools).
    WriteFile {
        dir: String,
        rel: String,
        bytes: Vec<u8>,
    },
}

/// RPCs for svc-storage (responses)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageResp {
    HealthOk,
    File { bytes: Vec<u8> },
    Written,
    NotFound,
    Err { err: String },
}

/// RPCs for svc-overlay (requests)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OverlayReq {
    Health,
    /// Get the file bytes within a bundle addressed by `addr`.
    /// If `rel` is empty, defaults to "payload.bin".
    Get {
        addr: String,
        rel: String,
    },
}

/// RPCs for svc-overlay (responses)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OverlayResp {
    HealthOk,
    Bytes { data: Vec<u8> },
    NotFound,
    Err { err: String },
}

/// Optional capability claims (future service auth).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CapClaims {
    pub sub: String,      // subject (service, role, or client id)
    pub ops: Vec<String>, // allowed methods
    pub exp: u64,         // expiry (unix seconds)
    pub nonce: u64,       // replay guard
    pub sig: Vec<u8>,     // ed25519 signature (svc-crypto later)
}
