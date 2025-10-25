//! RO:WHAT — Error taxonomy for OAP codec and helpers.
//! RO:WHY — Stable, typed errors that map to protocol status codes where meaningful.
//! RO:INTERACTS — Used by codec/frame/hello; callers can translate to HTTP or OAP rejects.
//! RO:INVARIANTS — 413 on size violations; 400 family for client misuse; 5xx for internal.

use thiserror::Error;

/// Minimal status code set suitable for mapping OAP outcomes (also maps to HTTP when proxied).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u16)]
pub enum StatusCode {
    Ok = 200,
    Partial = 206,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    PayloadTooLarge = 413,
    TooManyRequests = 429,
    Internal = 500,
    Unavailable = 503,
}

#[derive(Debug, Error)]
pub enum OapDecodeError {
    #[error("truncated header")]
    TruncatedHeader,
    #[error("bad version {0}")]
    BadVersion(u16),
    #[error("bad flags bits {0:#06x}")]
    BadFlags(u16),
    #[error("cap section present but START flag not set")]
    CapOnNonStart,
    #[error("frame too large: {len} > {max}")]
    FrameTooLarge { len: u32, max: u32 },
    #[error("cap length exceeds frame")]
    CapOutOfBounds,
    #[error("payload length exceeds frame")]
    PayloadOutOfBounds,
    #[error("decompression exceeded bound")]
    DecompressBoundExceeded,
    #[error("zstd not enabled")]
    ZstdFeatureNotEnabled,
    #[error("zstd decode error: {0}")]
    Zstd(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum OapEncodeError {
    #[error("frame too large: {len} > {max}")]
    FrameTooLarge { len: u32, max: u32 },
    #[error("cap length exceeds frame")]
    CapOutOfBounds,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum OapError {
    #[error(transparent)]
    Decode(#[from] OapDecodeError),
    #[error(transparent)]
    Encode(#[from] OapEncodeError),
}

// Allow `?` on std::io::Error to bubble into OapError (encode path preferred).
impl From<std::io::Error> for OapError {
    fn from(err: std::io::Error) -> Self {
        OapError::Encode(OapEncodeError::Io(err))
    }
}
