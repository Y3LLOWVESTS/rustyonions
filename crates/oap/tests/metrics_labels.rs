use oap::{metrics::*, StatusCode};

#[test]
fn reason_and_classification() {
    assert_eq!(reason(StatusCode::Ok), "OK");
    assert!(is_success(StatusCode::Ok));
    assert!(is_client_err(StatusCode::BadRequest));
    assert!(is_server_err(StatusCode::Internal));

    assert_eq!(outcome_from_status(StatusCode::Ok), OutcomeClass::Success);
    assert_eq!(labels_for_outcome(OutcomeClass::Oversize), ("oap","oversize","413"));
}
