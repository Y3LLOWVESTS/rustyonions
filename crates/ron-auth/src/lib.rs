#![forbid(unsafe_code)]
//! RO:WHAT   ron-auth core — capability verification & attenuation (pure library).
//! RO:WHY    Deterministic, offline decisions; callers inject keys/clock/context.
//! RO:INTERACTS  KMS via MacKeyProvider; service crates call verify_token()/verify_many().
//! RO:INVARIANTS No network/disk I/O; BLAKE3 keyed MAC; CBOR canonical; URL-safe Base64 (no pad).

pub mod prelude;

mod bounds;
mod builder;
mod cbor;
mod errors;
mod mac;
mod types;
mod verify;

pub use builder::{sign_and_encode_b64url, CapabilityBuilder};
pub use errors::{AuthError, DenyReason};
pub use types::{
    Capability, Caveat, Decision, MacKey, MacKeyProvider, RequestCtx, Scope, VerifierConfig,
};

// Public entrypoints (forward to verify module).
pub fn verify_token(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Decision, AuthError> {
    crate::verify::verify_token(cfg, token_b64url, ctx, keys)
}

pub fn verify_many(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Vec<Decision>, AuthError> {
    crate::verify::verify_many(cfg, tokens_b64url, ctx, keys)
}

/// Amortized batch into caller-provided buffer (no per-call Vec alloc).
pub fn verify_many_into(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError> {
    crate::verify::verify_many_into(cfg, tokens_b64url, ctx, keys, out)
}
