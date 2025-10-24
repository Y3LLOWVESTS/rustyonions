use serde_json as json;

#[test]
fn golden_oap_hello_v1_loads() {
    // Embedded golden vector
    const HELLO_JSON: &str = r#"
    {
      "protocol": "OAP/1",
      "version": 1,
      "features": []
    }
    "#;

    let hello: ron_proto::oap::hello::Hello = json::from_str(HELLO_JSON).unwrap();
    assert_eq!(hello.protocol, "OAP/1");
    assert_eq!(hello.version, ron_proto::version::PROTO_VERSION);
}

#[test]
fn golden_oap_data_min_loads() {
    // IMPORTANT: serde_json + serde_bytes represent Vec<u8> as an array of integers
    // (not base64) by default. Use numeric bytes here for portable goldens.
    const DATA_JSON: &str = r#"
    {
      "obj": "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "seq": 1,
      "bytes": [104,101,108,108,111]
    }
    "#;

    let data: ron_proto::oap::data::Data = json::from_str(DATA_JSON).unwrap();
    assert_eq!(data.seq, 1);
    assert_eq!(data.bytes, b"hello");
}
