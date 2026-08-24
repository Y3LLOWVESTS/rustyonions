//! RO:WHAT — Locks the Physical M1 transition from the historical unsigned Phase-0 authorization digest to the real ron-auth canonical signing transcript.
//! RO:WHY — A real Passport-root signature must never accidentally bless the earlier pipe-delimited fixture merely because its BLAKE3 digest was previously frozen.
//! RO:INTERACTS — ron-auth Native Passport transcript constants and the preserved svc-passport Phase-0 authorization vector.
//! RO:INVARIANTS — Phase-0 fixture remains unchanged and pending signature; its pipe encoding is explicitly non-signable; canonical V1 is deterministic length-prefixed binary.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no private key, signing operation, vault access, route, capability, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p ron-auth --test physical_m1_device_authorization_transcript_posture.

use ron_auth::native_passport::{
    DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING,
    DEVICE_AUTHORIZATION_V1_JSON_TRANSCRIPT_SIGNABLE,
    DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING,
    DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE,
    DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN,
};

const LEGACY_PHASE0_FIXTURE: &str =
    include_str!("../../svc-passport/tests/vectors/native_passport_device_authorization_v1.json");

// Compile-time false locks keep forbidden signing postures impossible.
const _: [(); 0] = [(); DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE as usize];

const _: [(); 0] = [(); DEVICE_AUTHORIZATION_V1_JSON_TRANSCRIPT_SIGNABLE as usize];

#[test]
fn physical_m1_phase0_pipe_transcript_is_preserved_but_not_signable() {
    assert_eq!(
        DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN,
        "rustyonions.native-passport.device-authorization.v1",
    );

    assert_eq!(
        DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING,
        "pipe-delimited-canonical-v1",
    );

    assert!(LEGACY_PHASE0_FIXTURE
        .contains("\"authorization_transcript_encoding\": \"pipe-delimited-canonical-v1\"",),);

    assert!(LEGACY_PHASE0_FIXTURE
        .contains("\"authorization_transcript_status\": \"b3_locked_phase0f_no_signature\"",),);

    assert!(
        LEGACY_PHASE0_FIXTURE.contains(
            "\"root_signature_status\": \"pending_phase1_or_later_ed25519_signature_lock\"",
        ),
    );

    assert!(LEGACY_PHASE0_FIXTURE
        .contains("\"root_signature_hex\": \"PHASE1_PENDING_ED25519_ROOT_SIGNATURE\"",),);
}

#[test]
fn physical_m1_real_signing_transcript_posture_is_binary_and_distinct() {
    assert_eq!(
        DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING,
        "length-prefixed-binary-v1",
    );

    assert_ne!(
        DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING,
        DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING,
    );

    assert!(!DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING.contains("json"),);

    assert!(!DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING.contains("pipe"),);
}
