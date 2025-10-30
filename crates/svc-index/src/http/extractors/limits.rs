//! RO:WHAT — Request body cap placeholder (Hardening v2.0).

#[derive(Clone, Copy)]
pub struct BodyLimits {
    pub max_bytes: usize,
}
