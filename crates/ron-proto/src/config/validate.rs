//! RO:WHAT — Trait-based validation sugar for DTOs.
//! RO:WHY  — Optional ergonomics so hosts can call `x.validate(...)` directly.

use crate::config::{validate_data, validate_hello, validate_start};
use crate::error::ProtoError;

/// Negotiated/host limits needed for validating certain frames.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Max bytes allowed in a single DATA frame (usually negotiated from START).
    pub max_frame_bytes: u32,
}

impl Default for Limits {
    fn default() -> Self {
        // Be conservative by default; many hosts will set this from START.
        Self {
            max_frame_bytes: crate::oap::MAX_FRAME_BYTES as u32,
        }
    }
}

/// DTOs can opt into trait-based validation.
pub trait Validate {
    fn validate(&self, limits: Limits) -> Result<(), ProtoError>;
}

impl Validate for crate::oap::hello::Hello {
    fn validate(&self, _limits: Limits) -> Result<(), ProtoError> {
        validate_hello(self)
    }
}

impl Validate for crate::oap::start::Start {
    fn validate(&self, _limits: Limits) -> Result<(), ProtoError> {
        validate_start(self)
    }
}

impl Validate for crate::oap::data::Data {
    fn validate(&self, limits: Limits) -> Result<(), ProtoError> {
        validate_data(self, limits.max_frame_bytes)
    }
}
