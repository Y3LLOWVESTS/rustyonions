// crates/omnigate/tests/policy_metrics.rs
// Proves policy_middleware_shortcircuits_total increments when policy denies a request.
// We scrape the text exposition to avoid private proto APIs.

use std::sync::Arc;

use axum::{body::Body, extract::Request, routing::get, Json, Router};
use prometheus::{gather, Encoder, TextEncoder};
use regex::Regex;
use ron_policy::PolicyBundle;
use serde_json::json;
use tower::ServiceExt;

async fn ping() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

// Strict bundle: default deny; allow only GET.
fn strict_bundle() -> PolicyBundle {
    let json = r#"
    {
      "version": 1,
      "defaults": { "default_action": "deny" },
      "rules": [
        { "id": "allow-gets", "when": { "method": "GET" }, "action": "allow" }
      ]
    }"#;
    serde_json::from_str::<PolicyBundle>(json).unwrap()
}

// Scrape the text exposition and sum all samples for a counter (any labels).
fn scrape_counter_sum(name: &str) -> f64 {
    let mut buf = Vec::new();
    TextEncoder::new().encode(&gather(), &mut buf).ok();
    let text = String::from_utf8_lossy(&buf);

    // Matches lines like:
    // policy_middleware_shortcircuits_total 3
    // policy_middleware_shortcircuits_total{status="403"} 2
    let re = Regex::new(&format!(
        r#"(?m)^{}\s*(?:\{{[^}}]*\}})?\s+([0-9]+(?:\.[0-9]+)?)\s*$"#,
        regex::escape(name)
    ))
    .unwrap();

    let mut sum = 0.0;
    for cap in re.captures_iter(&text) {
        if let Some(m) = cap.get(1) {
            if let Ok(v) = m.as_str().parse::<f64>() {
                sum += v;
            }
        }
    }
    sum
}

#[tokio::test]
async fn policy_deny_bumps_counter() {
    // Router with GET /v1/ping
    let base = Router::new().route("/v1/ping", get(ping));

    // Correct layering: middleware first (adds PolicyLayer), then Extension(bundle)
    let router =
        omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(strict_bundle())));

    // Read counter before
    let before = scrape_counter_sum("policy_middleware_shortcircuits_total");

    // Issue a denied PUT (default deny)
    let req = Request::builder()
        .method("PUT")
        .uri("/v1/ping")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status().as_u16(), 403, "policy should deny PUT");

    // Counter should have increased
    let after = scrape_counter_sum("policy_middleware_shortcircuits_total");
    assert!(
        after > before,
        "expected policy_middleware_shortcircuits_total to increase; before={before}, after={after}"
    );
}
