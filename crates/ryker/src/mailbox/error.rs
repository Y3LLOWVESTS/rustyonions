//! RO:WHAT — Mailbox-local error facade and Result alias.
//! RO:WHY  — Keep mailbox concerns self-contained while mapping 1:1 to crate errors.
//! RO:INTERACTS — Used by `queue.rs` and `builder.rs`; re-exports for callers via `mailbox::`.
//! RO:INVARIANTS — Mirrors `crate::errors::Error`; stable across minor versions.

#![forbid(unsafe_code)]

pub type MailboxResult<T, E = MailboxError> = std::result::Result<T, E>;

/// Thin alias to the crate-wide error so users can import `mailbox::MailboxError`
/// without reaching into `crate::errors`.
#[derive(Debug, thiserror::Error)]
pub enum MailboxError {
    #[error("mailbox at capacity (Busy)")]
    Busy,
    #[error("message too large (max {max} bytes)")]
    TooLarge { max: usize },
    #[error("mailbox closed")]
    Closed,
    #[error("deadline exceeded")]
    Timeout,
}

impl From<crate::errors::Error> for MailboxError {
    fn from(e: crate::errors::Error) -> Self {
        match e {
            crate::errors::Error::Busy => MailboxError::Busy,
            crate::errors::Error::TooLarge { max } => MailboxError::TooLarge { max },
            crate::errors::Error::Closed => MailboxError::Closed,
            crate::errors::Error::Timeout => MailboxError::Timeout,
            crate::errors::Error::Config(_) => {
                // Mailbox never bubbles config errors; map conservatively.
                MailboxError::Closed
            }
        }
    }
}

impl From<MailboxError> for crate::errors::Error {
    fn from(e: MailboxError) -> Self {
        match e {
            MailboxError::Busy => crate::errors::Error::Busy,
            MailboxError::TooLarge { max } => crate::errors::Error::TooLarge { max },
            MailboxError::Closed => crate::errors::Error::Closed,
            MailboxError::Timeout => crate::errors::Error::Timeout,
        }
    }
}
