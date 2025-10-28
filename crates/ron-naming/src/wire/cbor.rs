//! RO:WHAT — CBOR encode/decode helpers for Address and NameRecord.
//! RO:WHY  — Compact vectors for interop; mirrors JSON helpers.
//! RO:INVARIANTS — Pure serde; canonical map ordering is a caller concern.

use crate::{types::NameRecord, Address};
use serde::{de::DeserializeOwned, Serialize};

/// Encode any serializable DTO to CBOR bytes.
pub fn to_cbor_bytes<T: Serialize>(v: &T) -> Result<Vec<u8>, serde_cbor::Error> {
    serde_cbor::to_vec(v)
}

/// Decode any DTO from CBOR bytes.
pub fn from_cbor_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_cbor::Error> {
    serde_cbor::from_slice(bytes)
}

/// Round-trip an [`Address`] through CBOR (encode then decode).
pub fn roundtrip_address_cbor(a: &Address) -> Result<Address, serde_cbor::Error> {
    let bytes = to_cbor_bytes(a)?;
    from_cbor_bytes::<Address>(&bytes)
}

/// Round-trip a [`NameRecord`] through CBOR (encode then decode).
pub fn roundtrip_record_cbor(r: &NameRecord) -> Result<NameRecord, serde_cbor::Error> {
    let bytes = to_cbor_bytes(r)?;
    from_cbor_bytes::<NameRecord>(&bytes)
}
