//! Exercises ACK/END/EVENT helpers from `envelope`.

use bytes::Bytes;
use oap::{prelude::*, Flags};

#[test]
fn ack_end_event_algebra() {
    // EVENT without ACK → fire-and-forget
    let _ = FrameBuilder::request(1, 0xAB, 1)
        .payload(Bytes::from_static(b"evt"))
        .build();
    let flags = Flags::EVENT;
    assert!(is_fire_and_forget(flags));
    assert!(!wants_ack(flags));
    assert!(!is_terminal(flags));

    // REQUEST with ACK
    let f = FrameBuilder::request(1, 0xAB, 2)
        .payload(Bytes::from_static(b"req"))
        .want_ack()
        .build();
    let flags = f.header.flags | Flags::ACK_REQ;
    assert!(wants_ack(flags));
    assert!(!is_fire_and_forget(flags));

    // RESPONSE terminal
    let f = FrameBuilder::response(1, 0xAB, 3, StatusCode::Ok)
        .payload(Bytes::from_static(b"ok"))
        .end()
        .build();
    let flags = f.header.flags;
    assert!(is_terminal(flags));
}
