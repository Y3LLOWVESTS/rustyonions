//! RO:WHAT — Verifies ryker mailbox byte-limit posture is carried to runtime.
//! RO:WHY — Prevents configured max_msg_bytes from becoming a dead or decorative field.
//! RO:INTERACTS — ryker::runtime::Runtime, ryker::mailbox::MailboxBuilder.
//! RO:INVARIANTS — mailbox byte ceilings are inspectable; generic queues do not fake byte measurement.
//! RO:TEST — cargo test -p ryker --test mailbox_limits.

use ryker::{config::RykerConfig, runtime::Runtime};

#[test]
fn mailbox_reports_default_max_msg_bytes() {
    let cfg = RykerConfig::default();
    let expected = cfg.defaults.max_msg_bytes;

    let runtime = Runtime::new(cfg);
    let mailbox = runtime.mailbox_default::<Vec<u8>>("default-limit");

    assert_eq!(mailbox.max_msg_bytes(), expected);
}

#[test]
fn mailbox_reports_builder_override_max_msg_bytes() {
    let runtime = Runtime::new(RykerConfig::default());

    let mailbox = runtime
        .mailbox::<Vec<u8>>("override-limit")
        .max_msg_bytes(4 * 1024)
        .build();

    assert_eq!(mailbox.max_msg_bytes(), 4 * 1024);
}
