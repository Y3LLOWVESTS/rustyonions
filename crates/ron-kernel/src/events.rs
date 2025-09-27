//! KernelEvent (shape frozen; fields may not change without a major bump).
#[derive(Debug)]
pub enum KernelEvent {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}
