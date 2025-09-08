#![forbid(unsafe_code)]

use oap::*;
use tokio::io::duplex;

#[tokio::test]
async fn quota_error_roundtrip() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(4096);

    let send = async {
        let f = quota_error_frame("over_quota");
        write_frame(&mut a, &f, DEFAULT_MAX_FRAME).await?;
        Ok::<(), OapError>(())
    };

    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(fr.typ, FrameType::Error));
        let j: serde_json::Value = serde_json::from_slice(&fr.payload)?;
        assert_eq!(j.get("code").and_then(|v| v.as_str()), Some("quota"));
        assert_eq!(j.get("msg").and_then(|v| v.as_str()), Some("over_quota"));
        Ok::<(), OapError>(())
    };

    futures::future::try_join(send, recv).await?;
    Ok(())
}
