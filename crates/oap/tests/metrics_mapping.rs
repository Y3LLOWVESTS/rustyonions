//! Outcome classification & labels; decode-error mapping.

use oap::{metrics::*, OapDecodeError, StatusCode};

#[test]
fn status_outcomes_and_labels() {
    assert_eq!(outcome_from_status(StatusCode::Ok), OutcomeClass::Success);
    assert_eq!(
        outcome_from_status(StatusCode::BadRequest),
        OutcomeClass::ClientError
    );
    assert_eq!(
        outcome_from_status(StatusCode::Internal),
        OutcomeClass::ServerError
    );

    assert_eq!(
        labels_for_outcome(OutcomeClass::Success),
        ("oap", "ok", "2xx")
    );
    assert_eq!(
        labels_for_outcome(OutcomeClass::Oversize),
        ("oap", "oversize", "413")
    );
}

#[test]
fn decode_error_to_outcome() {
    let e = OapDecodeError::FrameTooLarge {
        len: 2_000_000,
        max: 1_048_576,
    };
    assert_eq!(outcome_from_decode(&e), OutcomeClass::Oversize);

    let e = OapDecodeError::BadFlags(0xFFFF);
    assert_eq!(outcome_from_decode(&e), OutcomeClass::DecodeError);
}
