//! RO:WHAT — Writer helpers: encode frames and flush.

use bytes::BytesMut;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::error::ConnResult;
use crate::protocol::oap::Frame; // <- keep only ConnResult

/// Encode a frame into a scratch buffer and write it out atomically.
pub async fn write_frame<W>(wr: &mut W, frame: &Frame, scratch: &mut BytesMut) -> ConnResult<()>
where
    W: AsyncWrite + Unpin,
{
    scratch.clear();
    frame.encode_to(scratch)?;
    wr.write_all(scratch).await?;
    wr.flush().await?;
    Ok(())
}
