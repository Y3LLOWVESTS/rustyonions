//! RO:WHAT   Index service client (thin wrapper over reqwest).
//! RO:WHY    Keep service-specific paths/types out of generic code.
//! RO:INTERACTS  GET /healthz, generic JSON GET/POST for later expansion.

use super::{build_client, DsError, RetryPolicy, full_jitter_backoff};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::time::Duration;

#[derive(Clone)]
pub struct IndexClient {
    base_url: String,
    client: reqwest::Client,
    retry: RetryPolicy,
    connect_timeout: Duration,
    req_timeout: Duration,
}

impl Default for IndexClient {
    fn default() -> Self {
        Self::new("http://127.0.0.1:5304")
    }
}

impl IndexClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_owned(),
            client: build_client(),
            retry: RetryPolicy::default(),
            connect_timeout: Duration::from_millis(200),
            req_timeout: Duration::from_secs(2),
        }
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self { self.retry = retry; self }

    pub async fn healthz(&self) -> Result<String, DsError> {
        self.get_text("/healthz", None).await
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, DsError> {
        self.exec::<(), T>("GET", path, None, None).await
    }

    pub async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, DsError> {
        self.exec("POST", path, Some(body), None).await
    }

    pub async fn get_text(&self, path: &str, corr_id: Option<&str>) -> Result<String, DsError> {
        self.exec::<(), String>("GET", path, None, corr_id).await
    }

    async fn exec<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
        corr_id: Option<&str>,
    ) -> Result<T, DsError> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path.trim_start_matches('/'));
        let mut attempt = 1u32;
        let mut rng = StdRng::from_entropy();

        loop {
            let res = {
                let mut req = self.client
                    .request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), &url)
                    .connect_timeout(self.connect_timeout)
                    .timeout(self.req_timeout);

                if let Some(id) = corr_id {
                    if let Ok(v) = reqwest::header::HeaderValue::from_str(id) {
                        let mut h = req.headers_mut();
                        h.insert("x-correlation-id", v);
                    }
                }
                if let Some(b) = body {
                    req = req.json(b);
                }
                req.send().await
            };

            match res {
                Ok(r) if r.status().is_success() => {
                    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>() {
                        // Fast path: text
                        let txt = r.text().await.map_err(DsError::Net)?;
                        // SAFETY: we know T=String at this call site
                        let any = unsafe { std::mem::transmute::<String, T>(txt) };
                        return Ok(any);
                    } else {
                        let txt = r.text().await.map_err(DsError::Net)?;
                        let out = serde_json::from_str::<T>(&txt)?;
                        return Ok(out);
                    }
                }
                Ok(r) => {
                    let status = r.status().as_u16();
                    let body = r.text().await.unwrap_or_default();
                    let err = DsError::Http { status, body };
                    if attempt >= self.retry.max_attempts || !err.is_retryable() {
                        return Err(err);
                    }
                    let sleep = full_jitter_backoff(attempt, self.retry.base_delay, self.retry.max_delay, &mut rng);
                    tokio::time::sleep(sleep).await;
                }
                Err(e) => {
                    let err = DsError::from(e);
                    if attempt >= self.retry.max_attempts || !err.is_retryable() {
                        return Err(err);
                    }
                    let sleep = full_jitter_backoff(attempt, self.retry.base_delay, self.retry.max_delay, &mut rng);
                    tokio::time::sleep(sleep).await;
                }
            }

            attempt += 1;
        }
    }
}
