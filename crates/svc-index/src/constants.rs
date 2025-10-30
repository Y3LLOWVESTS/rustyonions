//! RO:WHAT — Service-wide constants (OAP/HTTP bounds, header keys).

pub const OAP_MAX_FRAME_BYTES: usize = 1024 * 1024; // 1 MiB
pub const STORAGE_STREAM_CHUNK_HINT: usize = 64 * 1024;

pub const HDR_CORR_ID: &str = "x-corr-id";
pub const HDR_IDEMPOTENCY_KEY: &str = "idempotency-key";

/// Default max accepted body size for inbound HTTP requests (bytes).
/// Set to 1 MiB. Keep aligned with CONFIG.md defaults and body_limits middleware.
pub const MAX_BODY_BYTES: usize = 1_048_576; // 1 MiB
