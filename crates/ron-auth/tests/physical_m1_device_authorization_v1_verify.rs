//! RO:WHAT — Physical M1 strict-verifier tests for root-signed DeviceAuthorizationV1.
//! RO:WHY — Freeze trusted-root, Passport-ID, Device-ID, context, time, transcript, and strict Ed25519 behavior before the real physical Passport root signs the Mac device.
//! RO:INTERACTS — ron-auth strict verifier/transcript, ron-proto canonical authorization/identity DTOs, and deterministic test-only ed25519-dalek signing.
//! RO:INVARIANTS — trusted root never comes from authorization wire; exact canonical transcript bytes are signed; field/signature mutations reject; BLAKE3-digest, JSON, and legacy pipe signatures reject.
//! RO:METRICS — none.
//! RO:CONFIG — deterministic fake keys and fixed timestamps only.
//! RO:SECURITY — fake test-only private keys; no real Passport root, PIN, recovery material, vault, Keychain, network route, capability, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p ron-auth --test physical_m1_device_authorization_v1_verify.

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_device_authorization_v1_transcript, verify_device_authorization_v1_strict,
    DeviceAuthorizationVerificationContextV1, DeviceAuthorizationVerificationError,
    DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN,
};
use ron_proto::{
    DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
    DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationV1, DeviceClassV1, DeviceIdV1,
    Ed25519PublicKeyHex, Ed25519SignatureV1, NativePassportContextLabelV1, NativePassportScopeV1,
    PassportIdV1, DEVICE_ID_V1_ED25519_B3_PREFIX, DEVICE_ID_V1_HASH_DOMAIN,
    PASSPORT_ID_V1_HASH_DOMAIN, PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
};

const NOW_MS: u64 = 1_720_000_010_000;
const ROOT_EPOCH: u64 = 7;

fn lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        use std::fmt::Write as _;

        write!(&mut output, "{byte:02x}",).unwrap();
    }

    output
}

fn root_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
        0x2e, 0x2f,
    ])
}

fn other_root_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x77; 32])
}

fn device_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x42; 32])
}

fn root_public_key(key: &SigningKey) -> Ed25519PublicKeyHex {
    Ed25519PublicKeyHex::parse(lower_hex(&key.verifying_key().to_bytes())).unwrap()
}

fn passport_id(root_public_key: &Ed25519PublicKeyHex) -> PassportIdV1 {
    let input = format!(
        "{}|main|ed25519|{}",
        PASSPORT_ID_V1_HASH_DOMAIN,
        root_public_key.as_str(),
    );

    let digest = blake3::hash(input.as_bytes()).to_hex().to_string();

    PassportIdV1::parse(format!(
        "{}{}",
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX, digest,
    ))
    .unwrap()
}

fn device_public_key() -> Ed25519PublicKeyHex {
    let key = device_signing_key();

    Ed25519PublicKeyHex::parse(lower_hex(&key.verifying_key().to_bytes())).unwrap()
}

fn device_id(public_key: &Ed25519PublicKeyHex) -> DeviceIdV1 {
    let input = format!(
        "{}|ed25519|{}",
        DEVICE_ID_V1_HASH_DOMAIN,
        public_key.as_str(),
    );

    let digest = blake3::hash(input.as_bytes()).to_hex().to_string();

    DeviceIdV1::parse(format!("{}{}", DEVICE_ID_V1_ED25519_B3_PREFIX, digest,)).unwrap()
}

fn network() -> NativePassportContextLabelV1 {
    NativePassportContextLabelV1::parse("rustyonions-devnet").unwrap()
}

fn environment() -> NativePassportContextLabelV1 {
    NativePassportContextLabelV1::parse("local-dev").unwrap()
}

fn scopes() -> DeviceAuthorizationScopeCeilingV1 {
    let values = [
        "capability.revoke_self",
        "catalog.read",
        "confirmed_roc.read",
        "content.read",
        "entitlement.read",
        "identity.read",
        "receipts.read",
    ];

    DeviceAuthorizationScopeCeilingV1::new(
        values
            .into_iter()
            .map(|value| NativePassportScopeV1::parse(value).unwrap())
            .collect(),
    )
    .unwrap()
}

fn signing_payload() -> DeviceAuthorizationSigningPayloadV1 {
    let root = root_public_key(&root_signing_key());

    let device = device_public_key();

    DeviceAuthorizationSigningPayloadV1 {
        version: 1,
        network_id: network(),
        environment: environment(),
        passport_id: passport_id(&root),
        root_key_epoch: ROOT_EPOCH,
        device_id: device_id(&device),
        device_public_key: device,
        device_class: DeviceClassV1::RootAdminDesktop,
        authorized_scope_ceiling: scopes(),
        authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]),
        issued_at_ms: NOW_MS - 10_000,
        expires_at_ms: Some(NOW_MS + 60_000),
    }
}

fn signed_authorization() -> DeviceAuthorizationV1 {
    let payload = signing_payload();

    let transcript = canonical_device_authorization_v1_transcript(&payload).unwrap();

    let signature = root_signing_key().sign(&transcript).to_bytes();

    DeviceAuthorizationV1::from_signing_payload(payload, Ed25519SignatureV1::from_bytes(signature))
        .unwrap()
}

fn context<'a>(
    trusted_passport_id: &'a PassportIdV1,
    trusted_root_public_key: &'a Ed25519PublicKeyHex,
    expected_network_id: &'a NativePassportContextLabelV1,
    expected_environment: &'a NativePassportContextLabelV1,
) -> DeviceAuthorizationVerificationContextV1<'a> {
    DeviceAuthorizationVerificationContextV1 {
        trusted_passport_id,
        trusted_root_public_key,
        trusted_root_key_epoch: ROOT_EPOCH,
        expected_network_id,
        expected_environment,
        now_ms: NOW_MS,
        max_clock_skew_ms: 5_000,
    }
}

fn assert_rejected(authorization: &DeviceAuthorizationV1) {
    let root = root_public_key(&root_signing_key());

    let passport = passport_id(&root);

    let net = network();
    let env = environment();

    assert!(verify_device_authorization_v1_strict(
        authorization,
        context(&passport, &root, &net, &env,),
    )
    .is_err(),);
}

#[test]
fn physical_m1_valid_authorization_verifies_strictly() {
    let authorization = signed_authorization();

    let root = root_public_key(&root_signing_key());

    let passport = passport_id(&root);

    let net = network();
    let env = environment();

    verify_device_authorization_v1_strict(&authorization, context(&passport, &root, &net, &env))
        .unwrap();
}

#[test]
fn physical_m1_wrong_root_and_passport_binding_fail_closed() {
    let authorization = signed_authorization();

    let correct_root = root_public_key(&root_signing_key());

    let correct_passport = passport_id(&correct_root);

    let wrong_root = root_public_key(&other_root_signing_key());

    let wrong_passport = passport_id(&wrong_root);

    let net = network();
    let env = environment();

    assert_eq!(
        verify_device_authorization_v1_strict(
            &authorization,
            context(&correct_passport, &wrong_root, &net, &env,),
        ),
        Err(DeviceAuthorizationVerificationError::TrustedPassportRootBindingMismatch,),
    );

    assert_eq!(
        verify_device_authorization_v1_strict(
            &authorization,
            context(&wrong_passport, &wrong_root, &net, &env,),
        ),
        Err(DeviceAuthorizationVerificationError::PassportIdMismatch,),
    );
}

#[test]
fn physical_m1_device_id_public_key_binding_is_required() {
    let mut authorization = signed_authorization();

    authorization.device_id = DeviceIdV1::parse(
        "device:v1:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();

    let root = root_public_key(&root_signing_key());

    let passport = passport_id(&root);

    let net = network();
    let env = environment();

    assert_eq!(
        verify_device_authorization_v1_strict(
            &authorization,
            context(&passport, &root, &net, &env,),
        ),
        Err(DeviceAuthorizationVerificationError::DeviceIdBindingMismatch,),
    );
}

#[test]
fn physical_m1_context_epoch_and_time_mismatches_fail_closed() {
    let authorization = signed_authorization();

    let root = root_public_key(&root_signing_key());

    let passport = passport_id(&root);

    let net = network();
    let env = environment();

    let wrong_net = NativePassportContextLabelV1::parse("rustyonions-testnet").unwrap();

    let wrong_env = NativePassportContextLabelV1::parse("private-beta").unwrap();

    assert_eq!(
        verify_device_authorization_v1_strict(
            &authorization,
            context(&passport, &root, &wrong_net, &env,),
        ),
        Err(DeviceAuthorizationVerificationError::NetworkMismatch,),
    );

    assert_eq!(
        verify_device_authorization_v1_strict(
            &authorization,
            context(&passport, &root, &net, &wrong_env,),
        ),
        Err(DeviceAuthorizationVerificationError::EnvironmentMismatch,),
    );

    let mut wrong_epoch = context(&passport, &root, &net, &env);

    wrong_epoch.trusted_root_key_epoch = ROOT_EPOCH + 1;

    assert_eq!(
        verify_device_authorization_v1_strict(&authorization, wrong_epoch,),
        Err(DeviceAuthorizationVerificationError::RootKeyEpochMismatch,),
    );

    let mut future = signed_authorization();

    future.issued_at_ms = NOW_MS + 5_001;

    assert_eq!(
        verify_device_authorization_v1_strict(&future, context(&passport, &root, &net, &env,),),
        Err(DeviceAuthorizationVerificationError::NotYetValid,),
    );

    let mut expired = signed_authorization();

    expired.expires_at_ms = Some(NOW_MS - 5_001);

    assert_eq!(
        verify_device_authorization_v1_strict(&expired, context(&passport, &root, &net, &env,),),
        Err(DeviceAuthorizationVerificationError::Expired,),
    );
}

#[test]
fn physical_m1_every_bound_field_mutation_is_rejected() {
    let baseline = signed_authorization();

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
    changed.root_key_epoch += 1;
    variants.push(changed);

    let mut changed = baseline.clone();
    changed.device_id = DeviceIdV1::parse(
        "device:v1:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();
    variants.push(changed);

    let alternate_device = SigningKey::from_bytes(&[0x55; 32]);

    let alternate_device_public =
        Ed25519PublicKeyHex::parse(lower_hex(&alternate_device.verifying_key().to_bytes()))
            .unwrap();

    let mut changed = baseline.clone();

    changed.device_public_key = alternate_device_public.clone();

    changed.device_id = device_id(&alternate_device_public);

    variants.push(changed);

    let mut changed = baseline.clone();

    changed.device_class = DeviceClassV1::PersonalDesktop;

    variants.push(changed);

    let mut changed = baseline.clone();

    changed.authorized_scope_ceiling = DeviceAuthorizationScopeCeilingV1::new(vec![
        NativePassportScopeV1::parse("identity.read").unwrap(),
    ])
    .unwrap();

    variants.push(changed);

    let mut changed = baseline.clone();

    changed.authorization_nonce = DeviceAuthorizationNonceV1::from_bytes([0xff; 16]);

    variants.push(changed);

    let mut changed = baseline.clone();

    changed.issued_at_ms -= 1;
    variants.push(changed);

    let mut changed = baseline.clone();

    changed.expires_at_ms = Some(NOW_MS + 120_000);

    variants.push(changed);

    for variant in variants {
        assert_rejected(&variant);
    }
}

#[test]
fn physical_m1_signature_mutation_and_wrong_signer_are_rejected() {
    let mut mutated = signed_authorization();

    let mut signature_bytes = *mutated.root_signature.as_bytes();

    signature_bytes[0] ^= 0x01;

    mutated.root_signature = Ed25519SignatureV1::from_bytes(signature_bytes);

    assert_rejected(&mutated);

    let payload = signing_payload();

    let transcript = canonical_device_authorization_v1_transcript(&payload).unwrap();

    let wrong_signature = other_root_signing_key().sign(&transcript).to_bytes();

    let wrong_signed = DeviceAuthorizationV1::from_signing_payload(
        payload,
        Ed25519SignatureV1::from_bytes(wrong_signature),
    )
    .unwrap();

    assert_rejected(&wrong_signed);
}

#[test]
fn physical_m1_only_exact_canonical_bytes_are_signable() {
    let payload = signing_payload();

    let canonical = canonical_device_authorization_v1_transcript(&payload).unwrap();

    let digest = blake3::hash(&canonical);

    let digest_signature = root_signing_key().sign(digest.as_bytes()).to_bytes();

    let digest_signed = DeviceAuthorizationV1::from_signing_payload(
        payload.clone(),
        Ed25519SignatureV1::from_bytes(digest_signature),
    )
    .unwrap();

    assert_rejected(&digest_signed);

    let json = serde_json::to_vec(&payload).unwrap();

    let json_signature = root_signing_key().sign(&json).to_bytes();

    let json_signed = DeviceAuthorizationV1::from_signing_payload(
        payload.clone(),
        Ed25519SignatureV1::from_bytes(json_signature),
    )
    .unwrap();

    assert_rejected(&json_signed);

    let root = root_public_key(&root_signing_key());

    let nonce_hex = lower_hex(payload.authorization_nonce.as_bytes());

    let scopes_csv = payload
        .authorized_scope_ceiling
        .as_slice()
        .iter()
        .map(|scope| scope.as_str())
        .collect::<Vec<_>>()
        .join(",");

    let legacy_pipe = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN,
        payload.version,
        payload.network_id.as_str(),
        payload.environment.as_str(),
        payload.passport_id.as_str(),
        payload.root_key_epoch,
        root.as_str(),
        payload.device_id.as_str(),
        payload.device_public_key.as_str(),
        payload.device_class.as_str(),
        scopes_csv,
        nonce_hex,
        payload.issued_at_ms,
        payload.expires_at_ms.unwrap(),
    );

    let pipe_signature = root_signing_key().sign(legacy_pipe.as_bytes()).to_bytes();

    let pipe_signed = DeviceAuthorizationV1::from_signing_payload(
        payload,
        Ed25519SignatureV1::from_bytes(pipe_signature),
    )
    .unwrap();

    assert_rejected(&pipe_signed);
}
