//! RO:WHAT — ID helpers (nonce/jti).
pub fn rand_nonce() -> String {
    uuid::Uuid::new_v4().to_string()
}
