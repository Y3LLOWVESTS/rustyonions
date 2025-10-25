//! Conformance checks against protocol invariants (header size, START+cap rules).

use bytes::BytesMut;
use oap::{flags::Flags, prelude::*};
use tokio_util::codec::Decoder as _;

#[test]
fn header_size_constant() {
    // Defensive: if the header layout ever changes, this test will flag it.
    assert_eq!(Header::WIRE_SIZE, 4 + 2 + 2 + 2 + 2 + 16 + 2 + 8);
}

#[test]
fn cap_requires_start_flag() {
    // Craft a frame with cap_len>0 but without START flag → decoder must reject.
    let hdr = Header {
        len: (Header::WIRE_SIZE + 3) as u32,
        ver: OAP_VERSION,
        flags: Flags::REQ, // no START
        code: 0,
        app_proto_id: 123,
        tenant_id: 0xABCD,
        cap_len: 3,
        corr_id: 99,
    };
    let mut buf = BytesMut::new();
    // Put header, then 3 bytes of "cap".
    hdr.put_to(&mut buf);
    buf.extend_from_slice(b"cap");

    let mut dec = OapDecoder::default();
    let err = dec.decode(&mut buf).expect_err("should fail without START");
    match err {
        oap::OapDecodeError::CapOnNonStart => {}
        e => panic!("unexpected error: {e:?}"),
    }
}
