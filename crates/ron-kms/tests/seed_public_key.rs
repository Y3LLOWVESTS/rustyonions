//! RO:WHAT — Focused seed-to-public-key contract for ron-kms Ed25519 adapters.
//! RO:WHY — FINAL_BETA Phase 19 live Service Nodes need deterministic public identity
//!          for an explicitly supplied private-beta seed.
//! RO:INVARIANTS — derivation is deterministic; generated public key matches RFC 8032;
//!                 signatures made with the same seed verify under the derived key.
//! RO:SECURITY — no persistence, no seed logging, no wallet/ledger/finality authority.

use ron_kms::backends::ed25519;

fn decode_hex_32(input: &str) -> [u8; 32] {
    assert_eq!(input.len(), 64);

    let mut output = [0_u8; 32];

    for (index, pair) in input.as_bytes().chunks_exact(2).enumerate() {
        output[index] =
            (hex_nibble(pair[0]) << 4)
                | hex_nibble(pair[1]);
    }

    output
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("fixture contains non-lowercase-hex byte"),
    }
}

#[test]
fn seed_public_key_matches_rfc8032_vector_one() {
    let seed = decode_hex_32(
        "9d61b19deffd5a60ba844af492ec2cc4\
         4449c5697b326919703bac031cae7f60",
    );

    let expected_public_key = decode_hex_32(
        "d75a980182b10ab7d54bfed3c964073a\
         0ee172f3daa62325af021a68f707511a",
    );

    assert_eq!(
        ed25519::public_key(&seed),
        expected_public_key
    );
}

#[test]
fn seed_public_key_is_deterministic_and_verifies_matching_signature() {
    let seed = [0x42_u8; 32];
    let message = b"rustyonions-phase19-service-node";

    let first =
        ed25519::public_key(&seed);

    let second =
        ed25519::public_key(&seed);

    assert_eq!(first, second);

    let signature =
        ed25519::sign(&seed, message);

    assert!(
        ed25519::verify(
            &first,
            message,
            &signature,
        )
    );
}

#[test]
fn different_seed_does_not_share_public_identity() {
    let first_seed = [0x11_u8; 32];
    let second_seed = [0x22_u8; 32];

    assert_ne!(
        ed25519::public_key(&first_seed),
        ed25519::public_key(&second_seed),
    );
}
