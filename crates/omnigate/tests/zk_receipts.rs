use omnigate::zk::{OpClass, OpGuard, Receipt, ReceiptStatus};

#[test]
fn receipt_status_transitions_update_time() {
    let mut r = Receipt::new("r1".into());
    let t0 = r.last_update_ms;
    r.set_status(ReceiptStatus::Processing);
    assert!(r.last_update_ms >= t0);
    let t1 = r.last_update_ms;
    r.set_status(ReceiptStatus::Completed);
    assert!(r.last_update_ms >= t1);
}

#[test]
fn opguard_marks_class() {
    let g = OpGuard::new(OpClass::ReadOnly);
    assert_eq!(g.class(), OpClass::ReadOnly);
    let g2 = OpGuard::new(OpClass::Mutate);
    assert_eq!(g2.class(), OpClass::Mutate);
}
