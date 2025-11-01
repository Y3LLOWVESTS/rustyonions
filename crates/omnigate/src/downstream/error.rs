//! RO:WHAT   Egress error taxonomy.
//! RO:WHY    Normalize reqwest errors + HTTP status into a small set.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DsError {
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network: {0}")]
    Net(#[from] reqwest::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

impl DsError {
    pub fn is_retryable(&self) -> bool {
        match self {
            DsError::Http { status, .. } => (500..600).contains(status),
            DsError::Net(e) => e.is_connect() || e.is_timeout() || e.is_request(),
            DsError::Serde(_) => false,
        }
    }
}
