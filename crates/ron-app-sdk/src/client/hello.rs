#![forbid(unsafe_code)]

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tracing::warn;

use crate::constants::OAP_VERSION;
use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;
use crate::oap::hello::Hello;

use super::OverlayClient;

impl OverlayClient {
    /// Perform HELLO probe (app_proto_id=0). Saves the server info.
    pub async fn hello(&mut self) -> Result<Hello> {
        // HELLO request is an empty frame with app_proto_id=0.
        let req = OapFrame::hello_request();

        self.framed.send(req).await?;

        // Wait up to 5 seconds for a response
        let resp = timeout(Duration::from_secs(5), self.framed.next())
            .await
            .map_err(|_| Error::Timeout)?
            .ok_or_else(|| Error::Protocol("connection closed".into()))??;

        if !resp.flags.contains(OapFlags::RESP) && resp.app_proto_id != 0 {
            return Err(Error::Protocol("unexpected frame for HELLO".into()));
        }

        // Parse JSON body
        let hello: Hello = serde_json::from_slice(&resp.payload)
            .map_err(|e| Error::Decode(format!("hello json: {e}")))?;

        // Basic version check
        if !hello.oap_versions.contains(&OAP_VERSION) {
            warn!(
                "server does not list OAP/1; reported: {:?}",
                hello.oap_versions
            );
        }

        self.server = Some(hello.clone());
        Ok(hello)
    }
}
