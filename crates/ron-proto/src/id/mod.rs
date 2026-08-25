//! RO:WHAT — Newtypes and helpers for canonical IDs (content-addresses, names).
//! RO:WHY  — Strong typing for interop; prevent stringly-typed bugs.
//! RO:INTERACTS — Used across OAP envelopes, manifests, mailbox, governance.
//! RO:INVARIANTS — ContentId must be "b3:<64 lowercase hex>"; no hashing performed here.
//! RO:TEST — Round-trip serde tests and parser property tests live in this module.

mod content_id;
mod parse;
mod passport;

pub use content_id::{ContentId, CONTENT_ID_HEX_LEN, CONTENT_ID_PREFIX};
pub use parse::{is_lower_hex64, validate_b3_str, ParseContentIdError};
pub use passport::{
    is_capability_id_v1_b3, is_challenge_id_v1_b3, is_device_id_v1_ed25519_b3,
    is_passport_id_v1_main_ed25519_b3, B3DigestHex, CapabilityIdV1, ChallengeIdV1, DeviceClassV1,
    DeviceIdV1, Ed25519PublicKeyHex, LegacyPassportSubject, NativePassportDigestAlgorithm,
    NativePassportIdAlgorithm, NativePassportIdKind, NativePassportIdParseError,
    NativePassportIdVersion, PassportIdV1, B3_DIGEST_HEX_LEN, CAPABILITY_ID_V1_B3_PREFIX,
    CHALLENGE_ID_V1_B3_PREFIX, DEVICE_ID_V1_ED25519_B3_PREFIX, DEVICE_ID_V1_HASH_DOMAIN,
    ED25519_PUBLIC_KEY_HEX_LEN, PASSPORT_ID_V1_HASH_DOMAIN, PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
};
