//! RO:WHAT — Unit-style tests for svc-storage paid-write verifier seams.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Production wallet verification must fail closed unless lookup agrees.
//! RO:INTERACTS — svc_storage::policy::paid_write::{DevHeaderVerifier, WalletReceiptVerifier, WalletReceiptLookup}.
//! RO:INVARIANTS — dev verifier accepts explicit hold proof; wallet verifier requires matching looked-up hold receipt.
//! RO:METRICS — none; this validates policy-layer admission semantics only.
//! RO:CONFIG — no route config switch yet; /paid/o remains on DevHeaderVerifier in this batch.
//! RO:SECURITY — mock lookup only; no real bearer token, macaroon, wallet network call, or external chain.
//! RO:TEST — cargo test -p svc-storage --test paid_write_verifier.

use axum::http::{HeaderMap, HeaderName, HeaderValue};
use serde_json::json;
use svc_storage::policy::paid_write::{
    DevHeaderVerifier, PaidWriteVerificationError, PaidWriteVerifier, WalletReceipt,
    WalletReceiptLookup, WalletReceiptVerifier,
};

const VALID_RECEIPT_HASH: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut headers = HeaderMap::new();

    for (name, value) in pairs {
        headers.insert(
            HeaderName::from_bytes(name.as_bytes()).expect("header name should be valid"),
            HeaderValue::from_str(value).expect("header value should be valid"),
        );
    }

    headers
}

fn valid_headers() -> HeaderMap {
    headers(&[
        ("x-ron-paid-op", "hold"),
        ("x-ron-paid-asset", "roc"),
        ("x-ron-paid-estimate-minor", "70"),
        ("x-ron-wallet-txid", "tx_test_hold_1"),
        ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
        ("x-ron-wallet-from", "acct_user"),
        ("x-ron-wallet-to", "escrow_paid_write"),
    ])
}

fn valid_wallet_receipt() -> WalletReceipt {
    WalletReceipt {
        txid: "tx_test_hold_1".to_string(),
        op: "hold".to_string(),
        from: Some("acct_user".to_string()),
        to: Some("escrow_paid_write".to_string()),
        asset: "roc".to_string(),
        amount_minor: "70".to_string(),
        nonce: Some(1),
        idem: Some("idem_hold_1".to_string()),
        ts: Some(1),
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: Some(
            "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        ),
        receipt_hash: VALID_RECEIPT_HASH.to_string(),
    }
}

#[derive(Debug, Clone)]
struct StaticLookup {
    receipt: Result<WalletReceipt, PaidWriteVerificationError>,
}

impl StaticLookup {
    fn ok(receipt: WalletReceipt) -> Self {
        Self {
            receipt: Ok(receipt),
        }
    }

    fn missing() -> Self {
        Self {
            receipt: Err(PaidWriteVerificationError::PaymentRequired(
                "wallet receipt not found".to_string(),
            )),
        }
    }
}

impl WalletReceiptLookup for StaticLookup {
    fn lookup_receipt(&self, txid: &str) -> Result<WalletReceipt, PaidWriteVerificationError> {
        let receipt = self.receipt.clone()?;

        if receipt.txid != txid {
            return Err(PaidWriteVerificationError::PaymentRequired(format!(
                "wallet lookup returned wrong txid: expected {txid}, got {}",
                receipt.txid
            )));
        }

        Ok(receipt)
    }
}

#[test]
fn dev_header_verifier_accepts_valid_hold_receipt_metadata() {
    let verifier = DevHeaderVerifier;
    let verified = verifier
        .verify(&valid_headers())
        .expect("dev header verifier should accept valid proof headers");

    assert_eq!(verified.verifier, "dev-header");
    assert_eq!(verified.proof.txid, "tx_test_hold_1");
    assert_eq!(verified.proof.receipt_hash, VALID_RECEIPT_HASH);
    assert_eq!(verified.proof.payer, "acct_user");
    assert_eq!(verified.proof.escrow, "escrow_paid_write");
    assert_eq!(verified.proof.asset, "roc");
    assert_eq!(verified.proof.estimate_minor, 70);
}

#[test]
fn dev_header_verifier_rejects_non_hold_operation() {
    let bad = headers(&[
        ("x-ron-paid-op", "transfer"),
        ("x-ron-paid-asset", "roc"),
        ("x-ron-paid-estimate-minor", "70"),
        ("x-ron-wallet-txid", "tx_test_hold_1"),
        ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
        ("x-ron-wallet-from", "acct_user"),
        ("x-ron-wallet-to", "escrow_paid_write"),
    ]);

    let err = DevHeaderVerifier
        .verify(&bad)
        .expect_err("non-hold paid proof must reject");

    assert!(err.reason().contains("wallet hold receipt"));
}

#[test]
fn dev_header_verifier_rejects_bad_receipt_hash_shape() {
    let bad = headers(&[
        ("x-ron-paid-op", "hold"),
        ("x-ron-paid-asset", "roc"),
        ("x-ron-paid-estimate-minor", "70"),
        ("x-ron-wallet-txid", "tx_test_hold_1"),
        ("x-ron-wallet-receipt-hash", "not-a-b3-cid"),
        ("x-ron-wallet-from", "acct_user"),
        ("x-ron-wallet-to", "escrow_paid_write"),
    ]);

    let err = DevHeaderVerifier
        .verify(&bad)
        .expect_err("bad receipt hash must reject");

    assert!(err.reason().contains("b3:<64 lowercase hex>"));
}

#[test]
fn wallet_receipt_contract_accepts_hold_roc_positive_amount() {
    let receipt = valid_wallet_receipt();

    let proof = receipt
        .validate_as_paid_write_hold()
        .expect("valid wallet hold receipt should become paid-write proof");

    assert_eq!(proof.txid, "tx_test_hold_1");
    assert_eq!(proof.receipt_hash, VALID_RECEIPT_HASH);
    assert_eq!(proof.payer, "acct_user");
    assert_eq!(proof.escrow, "escrow_paid_write");
    assert_eq!(proof.asset, "roc");
    assert_eq!(proof.estimate_minor, 70);
}

#[test]
fn wallet_receipt_contract_deserializes_full_wallet_receipt_shape() {
    let receipt: WalletReceipt = serde_json::from_value(json!({
        "txid": "tx_test_hold_1",
        "op": "hold",
        "from": "acct_user",
        "to": "escrow_paid_write",
        "asset": "roc",
        "amount_minor": "70",
        "nonce": 1,
        "idem": "idem_hold_1",
        "ts": 1,
        "ledger_seq_start": 1,
        "ledger_seq_end": 2,
        "ledger_root": "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "receipt_hash": VALID_RECEIPT_HASH,
        "extra_future_wallet_field": "ignored-by-storage"
    }))
    .expect("storage wallet receipt subset should tolerate future wallet fields");

    let proof = receipt
        .validate_as_paid_write_hold()
        .expect("deserialized hold receipt should validate");

    assert_eq!(proof.payer, "acct_user");
    assert_eq!(proof.escrow, "escrow_paid_write");
    assert_eq!(proof.estimate_minor, 70);
}

#[test]
fn wallet_receipt_contract_rejects_wrong_operation() {
    let mut receipt = valid_wallet_receipt();
    receipt.op = "transfer".to_string();

    let err = receipt
        .validate_as_paid_write_hold()
        .expect_err("non-hold receipt must reject");

    assert!(err.reason().contains("wallet hold receipt"));
}

#[test]
fn wallet_receipt_contract_rejects_wrong_asset() {
    let mut receipt = valid_wallet_receipt();
    receipt.asset = "sol".to_string();

    let err = receipt
        .validate_as_paid_write_hold()
        .expect_err("non-roc receipt must reject");

    assert!(err.reason().contains("asset must be roc"));
}

#[test]
fn wallet_receipt_contract_rejects_zero_or_non_integer_amount() {
    let mut zero = valid_wallet_receipt();
    zero.amount_minor = "0".to_string();

    let zero_err = zero
        .validate_as_paid_write_hold()
        .expect_err("zero hold amount must reject");

    assert!(zero_err.reason().contains("greater than zero"));

    let mut non_integer = valid_wallet_receipt();
    non_integer.amount_minor = "70.5".to_string();

    let non_integer_err = non_integer
        .validate_as_paid_write_hold()
        .expect_err("non-integer hold amount must reject");

    assert!(non_integer_err.reason().contains("integer"));
}

#[test]
fn wallet_receipt_contract_rejects_missing_payer_or_escrow() {
    let mut missing_payer = valid_wallet_receipt();
    missing_payer.from = None;

    let payer_err = missing_payer
        .validate_as_paid_write_hold()
        .expect_err("missing payer must reject");

    assert!(payer_err.reason().contains("payer"));

    let mut missing_escrow = valid_wallet_receipt();
    missing_escrow.to = None;

    let escrow_err = missing_escrow
        .validate_as_paid_write_hold()
        .expect_err("missing escrow must reject");

    assert!(escrow_err.reason().contains("escrow"));
}

#[test]
fn wallet_receipt_contract_rejects_bad_receipt_hash() {
    let mut receipt = valid_wallet_receipt();
    receipt.receipt_hash = "not-a-b3-cid".to_string();

    let err = receipt
        .validate_as_paid_write_hold()
        .expect_err("bad receipt hash must reject");

    assert!(err.reason().contains("b3:<64 lowercase hex>"));
}

#[test]
fn wallet_receipt_verifier_fails_closed_until_lookup_is_wired() {
    let verifier = WalletReceiptVerifier::new("http://127.0.0.1:8088");
    assert_eq!(verifier.base_url(), "http://127.0.0.1:8088");

    let err = verifier
        .verify(&valid_headers())
        .expect_err("future wallet verifier must fail closed until wired");

    assert!(err.reason().contains("not wired yet"));
}

#[test]
fn wallet_receipt_verifier_accepts_matching_lookup_receipt() {
    let verifier = WalletReceiptVerifier::with_lookup(
        "http://127.0.0.1:8088",
        StaticLookup::ok(valid_wallet_receipt()),
    );

    let verified = verifier
        .verify(&valid_headers())
        .expect("matching wallet receipt should authorize paid write");

    assert_eq!(verifier.base_url(), "http://127.0.0.1:8088");
    assert_eq!(verified.verifier, "wallet-receipt");
    assert_eq!(verified.proof.txid, "tx_test_hold_1");
    assert_eq!(verified.proof.receipt_hash, VALID_RECEIPT_HASH);
    assert_eq!(verified.proof.payer, "acct_user");
    assert_eq!(verified.proof.escrow, "escrow_paid_write");
    assert_eq!(verified.proof.asset, "roc");
    assert_eq!(verified.proof.estimate_minor, 70);
}

#[test]
fn wallet_receipt_verifier_rejects_missing_lookup_receipt() {
    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::missing());

    let err = verifier
        .verify(&valid_headers())
        .expect_err("missing wallet receipt must fail closed");

    assert!(err.reason().contains("not found"));
}

#[test]
fn wallet_receipt_verifier_rejects_lookup_receipt_with_wrong_hash() {
    let mut receipt = valid_wallet_receipt();
    receipt.receipt_hash =
        "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string();

    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::ok(receipt));

    let err = verifier
        .verify(&valid_headers())
        .expect_err("receipt hash mismatch must reject");

    assert!(err.reason().contains("receipt_hash mismatch"));
}

#[test]
fn wallet_receipt_verifier_rejects_lookup_receipt_with_wrong_payer() {
    let mut receipt = valid_wallet_receipt();
    receipt.from = Some("acct_attacker".to_string());

    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::ok(receipt));

    let err = verifier
        .verify(&valid_headers())
        .expect_err("payer mismatch must reject");

    assert!(err.reason().contains("payer mismatch"));
}

#[test]
fn wallet_receipt_verifier_rejects_lookup_receipt_with_wrong_escrow() {
    let mut receipt = valid_wallet_receipt();
    receipt.to = Some("escrow_wrong".to_string());

    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::ok(receipt));

    let err = verifier
        .verify(&valid_headers())
        .expect_err("escrow mismatch must reject");

    assert!(err.reason().contains("escrow mismatch"));
}

#[test]
fn wallet_receipt_verifier_rejects_lookup_receipt_with_wrong_amount() {
    let mut receipt = valid_wallet_receipt();
    receipt.amount_minor = "71".to_string();

    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::ok(receipt));

    let err = verifier
        .verify(&valid_headers())
        .expect_err("amount mismatch must reject");

    assert!(err.reason().contains("amount mismatch"));
}

#[test]
fn wallet_receipt_verifier_rejects_lookup_receipt_with_wrong_asset() {
    let mut receipt = valid_wallet_receipt();
    receipt.asset = "sol".to_string();

    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::ok(receipt));

    let err = verifier
        .verify(&valid_headers())
        .expect_err("asset mismatch or invalid receipt must reject");

    assert!(err.reason().contains("asset must be roc"));
}

#[test]
fn wallet_receipt_verifier_rejects_lookup_receipt_with_wrong_operation() {
    let mut receipt = valid_wallet_receipt();
    receipt.op = "capture".to_string();

    let verifier =
        WalletReceiptVerifier::with_lookup("http://127.0.0.1:8088", StaticLookup::ok(receipt));

    let err = verifier
        .verify(&valid_headers())
        .expect_err("non-hold looked-up receipt must reject");

    assert!(err.reason().contains("wallet hold receipt"));
}
