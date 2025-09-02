#![forbid(unsafe_code)]

/// Protocol version (OAP/1).
pub const OAP_VERSION: u8 = 1;

/// Default maximum encoded frame size accepted/produced by the SDK.
pub const DEFAULT_MAX_FRAME: usize = 1 * 1024 * 1024; // 1 MiB

/// Default maximum decompressed payload (server MAY tighten via config).
pub const DEFAULT_MAX_DECOMPRESSED: usize = 8 * 1024 * 1024; // 8 MiB
