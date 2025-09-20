#![forbid(unsafe_code)]

use thiserror::Error;

/// Overlay-internal error type. Library callers still use `anyhow::Result`,
/// but internally we keep a precise, typed surface.
#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("early EOF")]
    EarlyEof,

    #[error("unknown opcode: 0x{0:02x}")]
    UnknownOpcode(u8),

    #[error("string too long ({0} bytes)")]
    StringTooLong(usize),

    #[error("invalid chunk_size (must be 0 or >= 4096)")]
    InvalidChunkSize,
}

/// Convenience alias for overlay-internal results.
pub type OResult<T> = std::result::Result<T, OverlayError>;
