// Config loading (stub)
use super::types::Config;

pub fn load() -> Config {
    Config {
        http_addr: "0.0.0.0:8080".into(),
        metrics_addr: "0.0.0.0:9909".into(),
        max_inflight: 512,
    }
}
