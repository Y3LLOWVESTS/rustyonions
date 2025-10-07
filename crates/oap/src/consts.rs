// Protocol constants and invariants (no implementation).
/// OAP protocol version (placeholder)
pub const OAP_VERSION: u32 = 1;

/// Hard cap per-frame payload (1 MiB). Changing this is a MAJOR SemVer event.
pub const MAX_FRAME_BYTES: usize = 1_048_576;
