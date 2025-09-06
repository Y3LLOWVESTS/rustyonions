mod config;
mod http_probe;
mod pack;
mod proc;
mod util;
mod wait;

use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::HashMap, fs, time::Duration};
use tokio::{signal, time::sleep};

use config::Cli;
use http_probe::http_get_status;
use pack::{pack_once_detect_and_parse, resolve_ok, try_index_scan};
use proc::{spawn_logged, ChildProc};
use util::{bin_path, cargo_build, ensure_exists, kv_env, tail_file, tempdir};
use wait::{parse_host_port, pick_ephemeral_port, wait_for_tcp, wait_for_uds};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve workspace root
    let root = fs::canonicalize(&cli.root).context("canonicalizing --root")?;
    ensure_exists(root.join("Cargo.toml"))?;

    if cli.build {
        cargo_build(&root).await?;
    }

    // Binaries
    let tldctl = bin_path(&root, "tldctl");
    let svc_index = bin_path(&root, "svc-index");
    let svc_storage = bin_path(&root, "svc-storage");
    let svc_overlay = bin_path(&root, "svc-overlay");
    let gateway = bin_path(&root, "gateway");
    for (label, p) in [
        ("tldctl", &tldctl),
        ("svc-index", &svc_index),
        ("svc-storage", &svc_storage),
        ("svc-overlay", &svc_overlay),
        ("gateway", &gateway),
    ] {
        ensure_exists(p).with_context(|| format!("missing binary {} at {}", label, p.display()))?;
    }

    // tmp + logs
    let tmp_dir = tempdir("ron_gwsmoke").context("creating tmp dir")?;
    println!("tmp dir    : {}", tmp_dir.display());
    let run_dir = tmp_dir.join("run");
    let log_dir = cli.log_dir.clone().unwrap_or_else(|| tmp_dir.join("logs"));
    fs::create_dir_all(&run_dir)?;
    fs::create_dir_all(&log_dir)?;

    // paths
    let idx_db = cli.index_db.clone().unwrap_or_else(|| tmp_dir.join("index"));
    fs::create_dir_all(&idx_db)?;
    let out_dir = if cli.out_dir.is_relative() { root.join(&cli.out_dir) } else { cli.out_dir.clone() };
    fs::create_dir_all(&out_dir)?;

    // common env
    let mut common_env = HashMap::new();
    common_env.insert("RON_INDEX_DB".to_string(), idx_db.display().to_string());
    common_env.insert("RUST_LOG".to_string(), cli.rust_log.clone());
    for kv in cli.env.iter() {
        if let Some((k, v)) = kv.split_once('=') { common_env.insert(k.to_string(), v.to_string()); }
    }

    // 1) pack .post
    let pack_out = tmp_dir.join("pack_post.out");
    let post_txt = tmp_dir.join("post.txt");
    fs::write(&post_txt, "Hello from RustyOnions gateway test (.post)\n")?;

    let addr_post = match pack_once_detect_and_parse(
        &tldctl, &out_dir, &idx_db, &cli.algo, "post", &post_txt, &pack_out
    ).await {
        Ok(addr) => addr,
        Err(e) => {
            let maybe = fs::read_to_string(&pack_out).unwrap_or_default();
            eprintln!("pack(post) failed: {e}\n--- pack_post.out ---\n{maybe}\n---------------------");
            return Err(e);
        }
    };
    println!("post addr  : {}", addr_post);

    // 2) resolve; if missing, attempt an index scan if the CLI supports it
    if !resolve_ok(&tldctl, &idx_db, &addr_post).await? {
        println!(
            "Index DB {} did not contain {}; attempting index scan (if supported)…",
            idx_db.display(),
            addr_post
        );
        match try_index_scan(&tldctl, &idx_db, &out_dir).await {
            Ok(true) => {
                if !resolve_ok(&tldctl, &idx_db, &addr_post).await? {
                    eprintln!("Index scan ran, but resolve still fails — will continue and start services to capture logs.");
                } else {
                    println!("reindex -> resolve OK");
                }
            }
            Ok(false) => {
                eprintln!("No index subcommand found in this tldctl build — continuing without reindex to capture service logs.");
            }
            Err(e) => {
                eprintln!("Index scan attempt errored: {e} — continuing anyway to capture service logs.");
            }
        }
    }

    // 3) start services (UDS)
    let idx_sock = run_dir.join("svc-index.sock");
    let sto_sock = run_dir.join("svc-storage.sock");
    let ovl_sock = run_dir.join("svc-overlay.sock");

    let env_index = kv_env(&common_env, &[("RON_INDEX_SOCK", idx_sock.display().to_string())]);
    let env_storage = kv_env(&common_env, &[("RON_STORAGE_SOCK", sto_sock.display().to_string())]);
    let env_overlay = kv_env(&common_env, &[
        ("RON_OVERLAY_SOCK", ovl_sock.display().to_string()),
        ("RON_INDEX_SOCK", idx_sock.display().to_string()),
        ("RON_STORAGE_SOCK", sto_sock.display().to_string()),
    ]);

    let svc_index_child: ChildProc = spawn_logged(
        "svc-index", &svc_index, &log_dir.join("svc-index.log"), &env_index, &[], cli.stream
    ).await?;
    let svc_storage_child: ChildProc = spawn_logged(
        "svc-storage", &svc_storage, &log_dir.join("svc-storage.log"), &env_storage, &[], cli.stream
    ).await?;
    let svc_overlay_child: ChildProc = spawn_logged(
        "svc-overlay", &svc_overlay, &log_dir.join("svc-overlay.log"), &env_overlay, &[], cli.stream
    ).await?;

    wait_for_uds(&idx_sock, Duration::from_secs(5)).await.context("waiting for svc-index UDS")?;
    wait_for_uds(&sto_sock, Duration::from_secs(5)).await.context("waiting for svc-storage UDS")?;
    wait_for_uds(&ovl_sock, Duration::from_secs(5)).await.context("waiting for svc-overlay UDS")?;

    // 4) start gateway
    let (bind_host, bind_port) = parse_host_port(&cli.bind)?;
    let port = if bind_port == 0 { pick_ephemeral_port(bind_host).await? } else { bind_port };
    let bind = format!("{bind_host}:{port}");

    let env_gateway = kv_env(&common_env, &[
        ("RON_OVERLAY_SOCK", ovl_sock.display().to_string()),
        ("RON_INDEX_DB", idx_db.display().to_string()),
    ]);

    let gateway_child: ChildProc = spawn_logged(
        "gateway", &gateway, &log_dir.join("gateway.log"), &env_gateway,
        &["--bind", &bind, "--index-db", &idx_db.display().to_string()],
        cli.stream
    ).await?;

    wait_for_tcp(&bind, Duration::from_secs(cli.http_wait_sec))
        .await
        .with_context(|| format!("waiting for HTTP accept at {}", bind))?;
    println!("gateway    : http://{}", bind);

    // 5) GET manifest (will likely fail with 404 if index still missing, but we now have logs)
    let url = format!("http://{}/o/{}/Manifest.toml", bind, addr_post);
    println!("GET {}", url);
    let code = http_get_status(&url).await?;
    if !(200..300).contains(&code) {
        println!("\nManifest GET failed with HTTP {}", code);
        println!("--- tail gateway.log ---\n{}", tail_file(&log_dir.join("gateway.log"), 200));
        println!("--- tail svc-overlay.log ---\n{}", tail_file(&log_dir.join("svc-overlay.log"), 200));
        println!("--- tail svc-index.log ---\n{}", tail_file(&log_dir.join("svc-index.log"), 200));
        // Return an error after showing logs — harness did its job producing diagnostics.
        anyhow::bail!("HTTP {} for {}", code, url);
    }
    println!("Manifest OK (HTTP {})", code);

    // Summary
    println!("\n=== Gateway Test Summary ===");
    println!("Gateway   : http://{}", bind);
    println!("OUT_DIR   : {}", out_dir.display());
    println!("INDEX_DB  : {}", idx_db.display());
    println!("POST addr : {}", addr_post);
    println!("Manifest  : {}", url);
    println!("Logs      : {}", log_dir.display());

    // quick grace
    tokio::select! {
        _ = signal::ctrl_c() => { println!("\n(ctrl-c) stopping…"); }
        _ = sleep(Duration::from_millis(10)) => {}
    }

    // stop children
    let _ = gateway_child.kill_and_wait().await;
    let _ = svc_overlay_child.kill_and_wait().await;
    let _ = svc_storage_child.kill_and_wait().await;
    let _ = svc_index_child.kill_and_wait().await;

    if cli.keep_tmp {
        println!("(Keeping TMP: {})", tmp_dir.display());
    } else {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    Ok(())
}
