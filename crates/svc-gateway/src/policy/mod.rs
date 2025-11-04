//! Residency/abuse policy stubs; attach via Extension<> AFTER inner layers.
//! Carry-over: Extension order discipline. :contentReference[oaicite:13]{index=13}
pub mod abuse;
pub mod residency;

#[derive(Clone, Default)]
pub struct PolicyBundle;
