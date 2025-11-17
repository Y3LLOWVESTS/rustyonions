//! RO:WHAT — Handle for the storage service (svc-storage) as seen
//!           from Micronode.
//!
//! RO:WHY  — Even though Micronode already has an in-process KV
//!           engine, some deployments will prefer to delegate
//!           large-object or CAS responsibilities to svc-storage.
//!           This client provides the configuration hook.
//!
//! RO:INTERACTS — In the future this will likely send OAP/1 requests
//!                over ron-transport rather than raw HTTP. For now it
//!                remains a pure data type.
//!
//! RO:INVARIANTS —
//!   * `base_url` typically points at the svc-storage ingress surface.
//!   * No implicit retries or backoff here; callers should own policy.

#[derive(Clone, Debug)]
pub struct StorageClient {
    base_url: String,
}

impl StorageClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Short, stable tag suitable for metrics or logging labels.
    pub fn tag(&self) -> &'static str {
        "svc-storage"
    }
}
