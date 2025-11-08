// crates/svc-passport/tests/issuer.rs
use svc_passport::state::issuer::IssuerState;

#[path = "../src/test_support.rs"]
mod test_support;

use test_support::issuer_state_for_tests;

#[tokio::test]
async fn sign_and_verify_round_trip_ok() {
    let issuer: IssuerState = issuer_state_for_tests();

    let msg = br#"{"hello":"world"}"#.to_vec();
    let (kid, sig) = issuer.sign(&msg).await.expect("sign");
    let ok = issuer.verify(&kid, &msg, &sig).await.expect("verify");

    assert!(ok, "signature should verify");
}

#[tokio::test]
async fn verify_fails_when_tampered() {
    let issuer: IssuerState = issuer_state_for_tests();

    let msg = br#"{"hello":"world"}"#.to_vec();
    let (kid, mut sig) = issuer.sign(&msg).await.expect("sign");

    // Tamper a single byte in the signature buffer (if long enough).
    if !sig.is_empty() {
        sig[0] ^= 0x01;
    }
    let ok = issuer
        .verify(&kid, &msg, &sig)
        .await
        .expect("verify call ok");
    assert!(!ok, "tampered signature must fail");
}
