//! RO:WHAT — Protocol-level errors (OAP/1 framing & handshake).
//! RO:INVARIANTS — No allocation bombs; errors are structured and non-panicking.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("frame too large: {got} > {max} bytes")]
    FrameTooLarge { got: usize, max: usize },

    #[error("incomplete frame")]
    Incomplete,

    #[error("bad magic/version: got {got:?}")]
    BadPreamble { got: [u8; 5] },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("handshake timeout")]
    HandshakeTimeout,

    #[error("capability mismatch")]
    CapabilityMismatch,
}

pub type ProtoResult<T> = Result<T, ProtoError>;
