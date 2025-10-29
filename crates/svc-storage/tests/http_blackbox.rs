use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[tokio::test]
async fn blackbox_put_head_get_range() {
    // Start server
    let addr = "127.0.0.1:5303";
    let mut child = Command::new("cargo")
        .args(["run", "-p", "svc-storage"])
        .env("ADDR", addr)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn svc-storage");

    // Wait for it to start (poll / or /healthz)
    let client = reqwest::Client::new();
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(8) {
            let _ = child.kill();
            panic!("svc-storage did not start");
        }
        if client
            .get(format!("http://{addr}/healthz"))
            .send()
            .await
            .is_ok()
        {
            break;
        }
        if client.get(format!("http://{addr}/")).send().await.is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    // POST hello world
    let resp = client
        .post(format!("http://{addr}/o"))
        .body("hello world")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let v: serde_json::Value = resp.json().await.unwrap();
    let cid = v["cid"].as_str().unwrap().to_string();
    assert!(cid.starts_with("b3:"));

    // HEAD
    let resp = client
        .head(format!("http://{addr}/o/{cid}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let len = resp
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();
    assert_eq!(len, 11);
    assert!(resp.headers().contains_key("etag"));

    // GET full
    let body = client
        .get(format!("http://{addr}/o/{cid}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(body, "hello world");

    // Range
    let resp = client
        .get(format!("http://{addr}/o/{cid}"))
        .header(reqwest::header::RANGE, "bytes=0-4")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 206);
    let cr = resp
        .headers()
        .get("content-range")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(cr.ends_with("/11"), "content-range={cr}");
    let part = resp.text().await.unwrap();
    assert_eq!(part, "hello");

    // shutdown server
    let _ = child.kill();
}
