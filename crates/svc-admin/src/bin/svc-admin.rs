use svc_admin::{cli, server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = cli::parse_args()?;
    server::run(cfg).await?;
    Ok(())
}
