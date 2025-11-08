//! RO:WHAT  Capability attenuation builder (pure); requires caller-provided signer.
//! RO:WHY   Build stricter children; never broaden scope.
//! RO:INVARIANTS No I/O; callers inject MacKeyProvider to sign.

use crate::types::MacKey;
use crate::{
    cbor::encode_b64url_cbor_capability,
    mac::compute_mac,
    types::{Capability, Caveat, MacKeyProvider, Scope},
};

#[derive(Debug, Clone)]
pub struct CapabilityBuilder {
    tid: String,
    kid: String,
    scope: Scope,
    caveats: Vec<Caveat>,
}

impl CapabilityBuilder {
    pub fn new(scope: Scope, tid: impl Into<String>, kid: impl Into<String>) -> Self {
        Self {
            tid: tid.into(),
            kid: kid.into(),
            scope,
            caveats: Vec::new(),
        }
    }

    pub fn caveat(mut self, c: Caveat) -> Self {
        self.caveats.push(c);
        self
    }

    pub fn build(self) -> Capability {
        // Unsigned until caller signs; mac is zeroed.
        Capability {
            tid: self.tid,
            kid: self.kid,
            scope: self.scope,
            caveats: self.caveats,
            mac: vec![0u8; 32],
        }
    }
}

/// Helper for hosts/tests to sign-and-encode using their key source.
/// Note: this is referenced primarily from integration tests and host crates.
#[allow(dead_code)]
pub fn sign_and_encode_b64url(
    cap: &mut Capability,
    keys: &impl MacKeyProvider,
) -> Result<String, &'static str> {
    let MacKey(k) = keys.key_for(&cap.kid, &cap.tid).ok_or("unknown kid")?;
    let tag = compute_mac(&MacKey(k), cap);
    cap.mac.clear();
    cap.mac.extend_from_slice(&tag);
    Ok(encode_b64url_cbor_capability(cap))
}
