use svc_admin::server;
use svc_admin::config::{Config, ServerCfg, AuthCfg, UiCfg, NodesCfg};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn healthz_smoke() {
    let cfg = Config {
        server: ServerCfg {
            bind_addr: "127.0.0.1:5300".into(),
            metrics_addr: "127.0.0.1:5310".into(),
        },
        auth: AuthCfg { mode: "none".into() },
        ui: UiCfg {
            default_theme: "light".into(),
            default_language: "en-US".into(),
            read_only: true,
        },
        nodes: NodesCfg {},
    };

    tokio::spawn(async move {
        let _ = server::run(cfg).await;
    });

    sleep(Duration::from_millis(200)).await;

    let body = reqwest::get("http://127.0.0.1:5310/healthz")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body, "ok");
}
