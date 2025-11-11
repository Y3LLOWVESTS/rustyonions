//! Local Result alias.
pub type Result<T, E = crate::error::Error> = core::result::Result<T, E>;
