//! RO:WHAT — Physical M1 locked-byte tests for the canonical DeviceAuthorizationV1 root-signing transcript.
//! RO:WHY — Freeze exact cross-SDK bytes before adding Ed25519 verification or asking the real physical Passport root to sign @testmac.
//! RO:INTERACTS — ron-auth canonical transcript builder and ron-proto DeviceAuthorizationSigningPayloadV1.
//! RO:INVARIANTS — binary length-prefixed V1 only; no JSON signing; Phase-0 pipe transcript is not reused; every valid bound-field mutation changes bytes; invalid version fails closed.
//! RO:METRICS — prints deterministic transcript byte count and BLAKE3 audit digest.
//! RO:CONFIG — deterministic fake fixture only.
//! RO:SECURITY — no private key, real signature, physical Passport, vault, Keychain, network route, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — cargo test -p ron-auth --test physical_m1_device_authorization_v1_transcript -- --nocapture.

use ron_auth::native_passport::{
    canonical_device_authorization_v1_transcript, device_authorization_v1_transcript_b3_hex,
    DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING,
    DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE,
    DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN,
};
use ron_proto::{
    DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
    DeviceAuthorizationSigningPayloadV1, DeviceClassV1, DeviceIdV1, Ed25519PublicKeyHex,
    NativePassportContextLabelV1, NativePassportScopeV1, PassportIdV1,
};

const PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";

const DEVICE_ID: &str =
    "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";

const DEVICE_PUBLIC_KEY: &str = "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";

const EXPECTED_TRANSCRIPT_BYTES: usize = 480;

// Compile-time lock: the historical Phase-0 transcript is never signable.
const _: [(); 0] = [(); DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE as usize];

const EXPECTED_TRANSCRIPT_HEX: &str = concat!(
    "003372757374796f6e696f6e732e6e61746976652d70617373706f72742e6465",
    "766963652d617574686f72697a6174696f6e2e76310001001272757374796f6e",
    "696f6e732d6465766e657400096c6f63616c2d646576005c70617373706f7274",
    "3a76313a6d61696e3a656432353531393a62333a616363323736316535383366",
    "6166633933636262383830626566316264373238356634336233626266333236",
    "6239653138356232323663353533336362376466000000000000000000556465",
    "766963653a76313a656432353531393a62333a34633138623935306665623536",
    "6263616432353739383231643839616565373662323539633238663737643435",
    "3964356665633333626564643366343166326400202dfbfd60452275c726f8be",
    "b1a3d6ff9e91abbe670977716225807e4645044b17000c74765f726561645f6f",
    "6e6c79000700166361706162696c6974792e7265766f6b655f73656c66000c63",
    "6174616c6f672e726561640012636f6e6669726d65645f726f632e7265616400",
    "0c636f6e74656e742e726561640010656e7469746c656d656e742e7265616400",
    "0d6964656e746974792e72656164000d72656365697074732e72656164001000",
    "0102030405060708090a0b0c0d0e0f0000019077fd300001000001907d238c00",
);

fn scope(value: &str) -> NativePassportScopeV1 {
    NativePassportScopeV1::parse(value).unwrap()
}

fn ceiling(values: &[&str]) -> DeviceAuthorizationScopeCeilingV1 {
    DeviceAuthorizationScopeCeilingV1::new(values.iter().map(|value| scope(value)).collect())
        .unwrap()
}

fn payload() -> DeviceAuthorizationSigningPayloadV1 {
    DeviceAuthorizationSigningPayloadV1 {
        version: 1,
        network_id: NativePassportContextLabelV1::parse("rustyonions-devnet").unwrap(),
        environment: NativePassportContextLabelV1::parse("local-dev").unwrap(),
        passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
        root_key_epoch: 0,
        device_id: DeviceIdV1::parse(DEVICE_ID).unwrap(),
        device_public_key: Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
        device_class: DeviceClassV1::TvReadOnly,
        authorized_scope_ceiling: ceiling(&[
            "capability.revoke_self",
            "catalog.read",
            "confirmed_roc.read",
            "content.read",
            "entitlement.read",
            "identity.read",
            "receipts.read",
        ]),
        authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]),
        issued_at_ms: 1_720_000_000_000,
        expires_at_ms: Some(1_720_086_400_000),
    }
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        use std::fmt::Write as _;

        write!(&mut output, "{byte:02x}").unwrap();
    }

    output
}

#[test]
fn physical_m1_binary_transcript_matches_locked_cross_sdk_bytes() {
    assert_eq!(
        DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN,
        "rustyonions.native-passport.device-authorization.v1",
    );

    assert_eq!(
        DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING,
        "length-prefixed-binary-v1",
    );

    let transcript = canonical_device_authorization_v1_transcript(&payload()).unwrap();

    assert_eq!(transcript.len(), EXPECTED_TRANSCRIPT_BYTES,);

    assert_eq!(lower_hex(&transcript), EXPECTED_TRANSCRIPT_HEX,);

    let digest = device_authorization_v1_transcript_b3_hex(&payload()).unwrap();

    println!("PHYSICAL_M1_BINARY_TRANSCRIPT_BYTES={}", transcript.len(),);

    println!("PHYSICAL_M1_BINARY_TRANSCRIPT_B3={digest}");
}

#[test]
fn physical_m1_every_valid_bound_field_mutation_changes_transcript() {
    let baseline = payload();

    let baseline_bytes = canonical_device_authorization_v1_transcript(&baseline).unwrap();

    let mut variants = Vec::new();

    let mut changed = baseline.clone();
    changed.network_id = NativePassportContextLabelV1::parse("rustyonions-testnet").unwrap();
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.environment = NativePassportContextLabelV1::parse("private-beta").unwrap();
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.passport_id =
        PassportIdV1::parse(
            "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.root_key_epoch = 1;
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.device_id = DeviceIdV1::parse(
        "device:v1:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.device_public_key = Ed25519PublicKeyHex::parse(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.device_class = DeviceClassV1::TestHarness;
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.authorized_scope_ceiling = ceiling(&[
        "capability.revoke_self",
        "catalog.read",
        "confirmed_roc.read",
        "content.read",
        "entitlement.read",
        "identity.read",
        "receipts.read",
        "zz.test",
    ]);
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.authorization_nonce = DeviceAuthorizationNonceV1::from_bytes([0xff; 16]);
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.issued_at_ms += 1;
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.expires_at_ms = None;
    variants.push(changed);

    for variant in variants {
        let bytes = canonical_device_authorization_v1_transcript(&variant).unwrap();

        assert_ne!(bytes, baseline_bytes,);
    }
}

#[test]
fn physical_m1_invalid_version_never_produces_signable_bytes() {
    let mut invalid = payload();
    invalid.version = 2;

    assert!(canonical_device_authorization_v1_transcript(&invalid,).is_err(),);
}
