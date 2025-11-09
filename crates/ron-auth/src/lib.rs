//! RO:WHAT  Public crate façade for ron-auth.
//! RO:WHY   Stable, minimal surface; re-exports common types and verify APIs.
//! RO:INTERACTS  Delegates to `verify` module; no I/O or globals.
//! RO:INVARIANTS  No panics; propagate typed errors; keep generics simple and zero-IO.

pub mod cbor;
pub mod errors;
pub mod mac;
pub mod metrics;
pub mod types;
pub mod verify;

pub use errors::{AuthError, DenyReason};
pub use types::{
    Capability, Caveat, Decision, MacKey, MacKeyProvider, RequestCtx, Scope, VerifierConfig,
};

/// Verify a single Base64URL-encoded token.
#[inline]
pub fn verify_token<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &K,
) -> Result<Decision, AuthError> {
    verify::verify_token(cfg, token_b64url, ctx, keys)
}

/// Verify many tokens; amortizes internal buffers and may parallelize
/// when built with `--features parallel`.
#[inline]
pub fn verify_many<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
) -> Result<Vec<Decision>, AuthError> {
    verify::verify_many(cfg, tokens_b64url, ctx, keys)
}

/// Same as `verify_many` but writes into `out` (clears it first).
#[inline]
pub fn verify_many_into<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError> {
    verify::verify_many_into(cfg, tokens_b64url, ctx, keys, out)
}

// ===== Bench/Test-friendly helpers (stable, zero I/O) ========================

/// Builder to assemble a Capability and sign/encode it for benches/tests.
///
/// NOTE: ctor order matches the bench: `new(scope, tid, kid)`.
#[derive(Debug, Clone)]
pub struct CapabilityBuilder {
    tid: String,
    kid: String,
    scope: Scope,
    caveats: Vec<Caveat>,
}

impl CapabilityBuilder {
    /// Create a new builder.
    pub fn new<T: Into<String>, K: Into<String>>(scope: Scope, tid: T, kid: K) -> Self {
        Self {
            tid: tid.into(),
            kid: kid.into(),
            scope,
            caveats: Vec::new(),
        }
    }

    /// Add a caveat (alias for `push` to match bench naming).
    #[inline]
    pub fn caveat(self, c: Caveat) -> Self {
        self.push(c)
    }

    #[inline]
    pub fn push(mut self, c: Caveat) -> Self {
        self.caveats.push(c);
        self
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item = Caveat>>(mut self, it: I) -> Self {
        self.caveats.extend(it);
        self
    }

    /// Finalize into an unsigned Capability (MAC empty).
    pub fn build_unsigned(self) -> Capability {
        Capability {
            tid: self.tid,
            kid: self.kid,
            scope: self.scope,
            caveats: self.caveats,
            mac: Vec::new(),
        }
    }

    /// Alias expected by benches.
    #[inline]
    pub fn build(self) -> Capability {
        self.build_unsigned()
    }

    /// Build, sign with `key`, and return the finalized Capability.
    pub fn build_and_sign(self, key: &MacKey) -> Capability {
        let mut cap = self.build_unsigned();
        let tag = crate::mac::compute_mac(key, &cap);
        cap.mac = tag.to_vec();
        cap
    }

    /// Build, sign, and Base64URL-encode the CBOR token.
    pub fn build_sign_encode(self, key: &MacKey) -> String {
        let cap = self.build_and_sign(key);
        crate::cbor::encode_b64url_cbor_capability(&cap)
    }
}

/// Public so it can be used in a public function bound (fixes `private_bounds` warning).
pub trait KeyLookup {
    fn key_for_cap(&self, kid: &str, tid: &str) -> Option<MacKey>;
}

impl KeyLookup for &MacKey {
    #[inline]
    fn key_for_cap(&self, _kid: &str, _tid: &str) -> Option<MacKey> {
        Some((**self).clone())
    }
}

impl<K: MacKeyProvider> KeyLookup for &K {
    #[inline]
    fn key_for_cap(&self, kid: &str, tid: &str) -> Option<MacKey> {
        MacKeyProvider::key_for(*self, kid, tid)
    }
}

/// Sign the provided `Capability` using either a `&MacKey` or a `&impl MacKeyProvider`,
/// then return the Base64URL-encoded CBOR token.
///
/// - Accepts `&Capability`.
/// - Returns `Result<String, AuthError>` so benches can `.unwrap()`.
#[inline]
pub fn sign_and_encode_b64url<L: KeyLookup>(
    cap: &Capability,
    lookup: L,
) -> Result<String, AuthError> {
    let key = lookup
        .key_for_cap(&cap.kid, &cap.tid)
        .ok_or(AuthError::UnknownKid)?;

    let mut tmp = Capability {
        tid: cap.tid.clone(),
        kid: cap.kid.clone(),
        scope: cap.scope.clone(),
        caveats: cap.caveats.clone(),
        mac: Vec::new(),
    };
    let tag = mac::compute_mac(&key, &tmp);
    tmp.mac = tag.to_vec();
    Ok(cbor::encode_b64url_cbor_capability(&tmp))
}
