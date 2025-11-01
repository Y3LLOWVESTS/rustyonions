//! RO:WHAT   DHT service client (thin).
//! RO:WHY    Encapsulate K/V provider lookups etc.

use super::{build_client, DsError};
use std::time::Duration;

#[derive(Clone)]
pub struct DhtClient {
    base_url: String,
    client: reqwest::Client,
    connect_timeout: Duration,
    req_timeout: Duration,
}

impl Default for DhtClient {
    fn default() -> Self { Self::new("http://127.0.0.1:5301") }
}

impl DhtClient {
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
