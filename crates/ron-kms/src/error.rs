use thiserror::Error;

#[derive(Debug, Error)]
pub enum KmsError {
    #[error("no such key")]
    NoSuchKey,
    #[error("algorithm unavailable in this build")]
    AlgUnavailable,
    #[error("key expired or not valid for use")]
    Expired,
    #[error("entropy/rng error")]
    Entropy,
    #[error("verification failed")]
    VerifyFailed,
    #[error("capability required")]
    CapabilityMissing,
    #[error("rotation in progress; try again")]
    Busy,
    #[error("internal error: {0}")]
    Internal(&'static str),
}

impl KmsError {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            KmsError::NoSuchKey => "NoSuchKey",
            KmsError::AlgUnavailable => "AlgUnavailable",
            KmsError::Expired => "Expired",
            KmsError::Entropy => "Entropy",
            KmsError::VerifyFailed => "VerifyFailed",
            KmsError::CapabilityMissing => "CapabilityMissing",
            KmsError::Busy => "Busy",
            KmsError::Internal(_) => "Internal",
        }
    }
}
