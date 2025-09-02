#![forbid(unsafe_code)]

use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;

use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;

use super::OverlayClient;

impl OverlayClient {
    /// Simple one-shot request (REQ|START|END) returning single RESP.
    ///
    /// Use this for small control-plane ops (e.g., tile lookup headers or mailbox commands).
    pub async fn request_oneshot(
        &mut self,
        app_proto_id: u16,
        tenant_id: u128,
        payload: impl Into<Bytes>,
    ) -> Result<Bytes> {
        let corr = self.next_corr();
        let req = OapFrame::oneshot_req(app_proto_id, tenant_id, corr, payload.into());
        self.framed.send(req).await?;

        let resp = timeout(Duration::from_secs(10), self.framed.next()).await
            .map_err(|_| Error::Timeout)?
            .ok_or_else(|| Error::Protocol("connection closed".into()))??;

        if !resp.flags.contains(OapFlags::RESP) {
            return Err(Error::Protocol("expected RESP".into()));
        }
        if resp.corr_id != corr {
            return Err(Error::Protocol("corr_id mismatch".into()));
        }
        // Map nonzero status to error families later; Bronze returns payload.
        Ok(resp.payload)
    }
}
