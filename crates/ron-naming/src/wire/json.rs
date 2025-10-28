//! RO:WHAT — JSON encode/decode helpers for Address and NameRecord.
//! RO:WHY  — Test vectors & SDK interop in a single place.
//! RO:INVARIANTS — Pure serde; deny unknown fields via parent DTOs.

use crate::{types::NameRecord, Address};
use serde::{de::DeserializeOwned, Serialize};

/// Encode any serializable DTO to JSON bytes.
pub fn to_json_bytes<T: Serialize>(v: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(v)
}

/// Decode any DTO from JSON bytes.
pub fn from_json_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// Convenience: round-trip an Address to JSON and back.
pub fn roundtrip_address_json(a: &Address) -> Result<Address, serde_json::Error> {
    let bytes = to_json_bytes(a)?;
    from_json_bytes::<Address>(&bytes)
}

/// Convenience: round-trip a NameRecord to JSON and back.
pub fn roundtrip_record_json(r: &NameRecord) -> Result<NameRecord, serde_json::Error> {
    let bytes = to_json_bytes(r)?;
    from_json_bytes::<NameRecord>(&bytes)
}
