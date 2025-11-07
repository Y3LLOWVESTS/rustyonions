//! RO:WHAT — Keyslot types (versioned KID newtype).
#[derive(Debug, Clone)]
pub struct KeyId(pub String);
impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
