//! RO:WHAT — Protocol version & ABI fingerprint constants.
//! RO:WHY  — Gate breaking changes; SDKs/tests assert these during upgrades.

pub const PROTO_VERSION: u32 = 1;

// NOTE: In CI you can replace this with include_str!("../docs/schema/fingerprint.txt")
// to enforce schema-diff gates. Using a placeholder here to keep the crate self-contained.
pub const PROTO_ABI_FINGERPRINT: &str = "dev-fingerprint";
