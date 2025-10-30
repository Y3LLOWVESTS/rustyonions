//! RO:WHAT — Placeholder for capability checks (macaroon etc).

pub mod caps {
    pub fn check_read() -> bool {
        true
    }
    pub fn check_admin() -> bool {
        false
    } // TODO
}
pub mod uds_allow {} // placeholder
