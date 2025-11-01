//! RO:WHAT   Mailbox/notification client (thin).
//! RO:WHY    Keeps notify calls decoupled from core.

use super::{build_client, DsError};
use std::time::Duration;

#[derive(Clone)]
pub struct MailboxClient {
    base_url: String,
    client: reqwest::Client,
    connect_timeout: Duration,
    req_timeout: Duration,
}

impl Default for MailboxClient {
    fn default() -> Self { Self::new("http://127.0.0.1:5310") }
}

impl MailboxClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_owned(),
            client: build_client(),
            connect_timeout: Duration::from_millis(200),
            req_timeout: Duration::from_secs(2),
        }
    }

    pub async fn healthz(&self) -> Result<String, DsError> {
        let url = format!("{}/healthz", self.base_url.trim_end_matches('/'));
        let res = self.client
            .get(url)
            .connect_timeout(self.connect_timeout)
            .timeout(self.req_timeout)
            .send().await?;
        if res.status().is_success() {
            Ok(res.text().await.unwrap_or_default())
        } else {
            Ok(format!("status={}", res.status().as_u16()))
        }
    }
}
