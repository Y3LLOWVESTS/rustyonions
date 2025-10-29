//! Error taxonomy for svc-storage.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("object not found")]
    NotFound,

    #[error("bad address format")]
    BadAddress,

    #[error("range not satisfiable")]
    RangeNotSatisfiable,

    #[error("request body too large")]
    CapacityExceeded,

    #[error("integrity check failed")]
    IntegrityFailed,
}
