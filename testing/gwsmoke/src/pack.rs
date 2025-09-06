use anyhow::{anyhow, Result};
use regex::Regex;
use std::{
    collections::HashMap,
    path::Path,
};
use crate::util::{run_capture, run_capture_to_file};

/// Build argv for `tldctl pack`, adapting to detected flags (never using `--index`).
fn pack_argv_detected(
    help: &str,
    out_dir: &Path,
    index_db: &Path,
    algo: &str,
    tld: &str,
    input_file: &Path,
) -> Result<Vec<String>> {
    let has = |flag: &str| help.contains(flag);

    let mut argv: Vec<String> = vec!["pack".into()];
    if has("--tld") {
        argv.push("--tld".into());
        argv.push(tld.into());
    } else {
        argv.push(tld.into()); // legacy positional tld
    }

    if has("--input") {
        argv.push("--input".into());
        argv.push(input_file.display().to_string());
    } else if has("--file") {
        argv.push("--file".into());
        argv.push(input_file.display().to_string());
    } else {
        return Err(anyhow!(
            "tldctl pack: neither --input nor --file supported in this build"
        ));
    }

    if has("--algo") {
        argv.push("--algo".into());
        argv.push(algo.into());
    }

    // IMPORTANT: don't add --index (unsupported in your build); --index-db is sufficient
    if has("--index-db") {
        argv.push("--index-db".into());
        argv.push(index_db.display().to_string());
    }

    if has("--store-root") {
        argv.push("--store-root".into());
        argv.push(out_dir.display().to_string());
    } else if has("--out") {
        argv.push("--out".into());
        argv.push(out_dir.display().to_string());
    } else {
        return Err(anyhow!(
            "tldctl pack: neither --store-root nor --out supported in this build"
        ));
    }

    Ok(argv)
}

fn strip_b3(addr: &str) -> String {
    addr.strip_prefix("b3:").unwrap_or(addr).to_string()
}

/// Parse a tldctl pack output into "<hex>.<tld>" (without "b3:" scheme).
/// Accepts either:
///   1) `OK: .../<hex>.<tld>/Manifest.toml`
///   2) a single line address: `[b3:]<hex>.<tld>`
fn parse_pack_output(tld: &str, output: &str) -> Option<String> {
    // 1) Old "OK:" path line
    let ok_re = Regex::new(&format!(
        r#"^OK:\s+.*/([^/]+)\.{}\s*/Manifest\.toml\s*$"#,
        regex::escape(tld)
    )).ok()?;
    if let Some(cap) = output.lines().filter_map(|l| ok_re.captures(l)).next() {
        return Some(cap[1].to_string() + "." + tld);
    }

    // 2) Single-line address (with or without b3:)
    let addr_re = Regex::new(r#"^(?:b3:)?([0-9a-f]{8,}\.[a-z0-9]+)\s*$"#).ok()?;
    if let Some(cap) = output.lines().filter_map(|l| addr_re.captures(l)).next() {
        return Some(cap[1].to_string()); // already without scheme
    }

    None
}

/// Run a single pack, write output to file, return "<hex>.<tld>" (no "b3:" prefix).
pub async fn pack_once_detect_and_parse(
    tldctl: &Path,
    out_dir: &Path,
    index_db: &Path,
    algo: &str,
    tld: &str,
    input_file: &Path,
    capture_out: &Path,
) -> Result<String> {
    let help = run_capture::<&str>(tldctl, &["pack", "--help"], None).await?;
    let argv = pack_argv_detected(help.as_str(), out_dir, index_db, algo, tld, input_file)?;
    let output = run_capture_to_file(tldctl, &argv, None, capture_out).await?;

    match parse_pack_output(tld, &output) {
        Some(addr) => Ok(strip_b3(&addr)),
        None => Err(anyhow!("pack output did not contain a recognizable .{} address", tld)),
    }
}

/// Try resolve with or without the b3: prefix
pub async fn resolve_ok(tldctl: &Path, index_db: &Path, addr: &str) -> Result<bool> {
    let mut envs = HashMap::new();
    envs.insert("RON_INDEX_DB".to_string(), index_db.display().to_string());

    let try1 = run_capture::<&str>(tldctl, &["resolve", addr], Some(&envs)).await;
    if try1.is_ok() {
        return Ok(true);
    }
    let prefixed = format!("b3:{}", addr);
    let try2 = run_capture::<&str>(tldctl, &["resolve", prefixed.as_str()], Some(&envs)).await;
    Ok(try2.is_ok())
}

/// Attempt to (re)index OUT_DIR into index_db using *whatever* your tldctl supports.
/// Returns `Ok(true)` if an indexing command was found and executed, `Ok(false)` if no suitable
/// subcommand exists, and `Err` only on an actual execution error of a detected command.
pub async fn try_index_scan(tldctl: &Path, index_db: &Path, out_dir: &Path) -> Result<bool> {
    let mut envs = HashMap::new();
    envs.insert("RON_INDEX_DB".to_string(), index_db.display().to_string());

    // 1) Check if `tldctl index scan --help` exists and mentions "scan"
    let idx_help = run_capture::<&str>(tldctl, &["index", "--help"], None).await;
    if let Ok(h) = &idx_help {
        if h.contains("scan") {
            let out_dir_s = out_dir.display().to_string();
            let _ = run_capture::<&str>(
                tldctl,
                &["index", "scan", "--store-root", out_dir_s.as_str()],
                Some(&envs),
            )
            .await?; // execution error bubbles up
            return Ok(true);
        }
    }

    // 2) Some builds expose a top-level `scan`
    let top_help = run_capture::<&str>(tldctl, &["--help"], None).await;
    if let Ok(h) = &top_help {
        // very lenient check
        if h.lines().any(|l| l.contains("scan")) {
            let out_dir_s = out_dir.display().to_string();
            let ok = run_capture::<&str>(
                tldctl,
                &["scan", "--store-root", out_dir_s.as_str()],
                Some(&envs),
            )
            .await;
            if ok.is_ok() {
                return Ok(true);
            }
        }
    }

    // 3) No known index command available
    Ok(false)
}
