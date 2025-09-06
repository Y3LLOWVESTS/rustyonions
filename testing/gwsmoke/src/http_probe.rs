use anyhow::{Context, Result};

pub async fn http_get_status(url: &str) -> Result<u16> {
    let client = reqwest::Client::builder()
        .build()
        .context("build HTTP client")?;
    let resp = client.get(url).send().await.context("HTTP GET")?;
    Ok(resp.status().as_u16())
}
