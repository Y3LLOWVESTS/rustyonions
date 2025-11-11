//! Adapter: content-addressed storage (read-only) — stub.

/// Digest bytes (opaque placeholder).
pub type Digest = [u8; 32];

/// CAS interface (read-only, stubbed).
pub trait CasStore: Send + Sync {
    /// Fetch a blob by digest. Returns `None` if missing.
    fn get(&self, _digest: &Digest) -> Option<Vec<u8>>;
}

/// No-op CAS for early wiring.
#[derive(Debug, Clone, Default)]
pub struct NullCas;

impl CasStore for NullCas {
    fn get(&self, _digest: &Digest) -> Option<Vec<u8>> {
        None
    }
}
