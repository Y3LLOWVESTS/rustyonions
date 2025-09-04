#![forbid(unsafe_code)]
use oap::*;
use serde_json::Value;
use tokio::io::duplex;

#[tokio::test]
async fn quota_error_frame_roundtrip() {
    let (mut a, mut b) = duplex(4096);
    let send = async {
        let f = quota_error_frame("rps exceeded");
        write_frame(&mut a, &f, DEFAULT_MAX_FRAME).await.unwrap();
        Ok::<_, OapError>(())
    };
    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap();
        assert!(matches!(fr.typ, FrameType::Error));
        let j: Value = serde_json::from_slice(&fr.payload).unwrap();
        assert_eq!(j["code"], "quota");
        assert_eq!(j["msg"], "rps exceeded");
        Ok::<_, OapError>(())
    };
    futures::future::try_join(send, recv).await.unwrap();
}
