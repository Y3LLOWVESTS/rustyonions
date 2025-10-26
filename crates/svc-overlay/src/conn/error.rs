//! RO:WHAT — Connection-level errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Proto(#[from] crate::protocol::error::ProtoError),
}

pub type ConnResult<T> = Result<T, ConnError>;
