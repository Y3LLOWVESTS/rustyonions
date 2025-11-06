//! RO:WHAT  Internal trait to expose verifying keys for batch verify fast paths.
//! RO:WHY   Our public Verifier trait only exposes boolean checks; batch verify
//!          needs access to the raw verifying keys to use dalek's batch API.

use crate::{error::KmsError, types::KeyId};

/// Internal-only surface to fetch verifying key bytes for a given `KeyId` version.
/// Implemented by in-crate backends (memory, file, pkcs11) as needed.
pub trait PubkeyProvider {
    /// Returns the raw verifying key bytes (Ed25519, 32 bytes) for this `KeyId`/version.
    fn verifying_key_bytes(&self, kid: &KeyId) -> Result<[u8; 32], KmsError>;
}
