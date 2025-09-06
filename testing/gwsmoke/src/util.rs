use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::process::Command;

pub fn ensure_exists<P: AsRef<Path>>(p: P) -> Result<()> {
    if !p.as_ref().exists() {
        Err(anyhow!("not found: {}", p.as_ref().display()))
    } else {
        Ok(())
    }
}

pub async fn cargo_build(root: &Path) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p").arg("tldctl")
        .arg("-p").arg("svc-index")
        .arg("-p").arg("svc-storage")
        .arg("-p").arg("svc-overlay")
        .arg("-p").arg("gateway")
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let out = cmd.output().await.context("spawning cargo build")?;
    if !out.status.success() {
        let mut msg = String::from_utf8_lossy(&out.stderr).into_owned();
        if msg.trim().is_empty() {
            msg = String::from_utf8_lossy(&out.stdout).into_owned();
        }
        return Err(anyhow!("cargo build failed:\n{}", msg));
    }
    Ok(())
}

pub fn bin_path(root: &Path, name: &str) -> PathBuf {
    root.join("target").join("debug").join(name)
}

pub fn tempdir(prefix: &str) -> Result<PathBuf> {
    let base = std::env::temp_dir();
    for _ in 0..16 {
        let p = base.join(format!("{}.{}", prefix, nanoid()));
        if !p.exists() {
            fs::create_dir_all(&p)?;
            return Ok(p);
        }
    }
    Err(anyhow!("failed to create temp dir"))
}

fn nanoid() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", n)
}

pub async fn run_capture<P: AsRef<OsStr>>(
    bin: &Path,
    args: &[P],
    envs: Option<&HashMap<String, String>>,
) -> Result<String> {
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(a);
    }
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }
    let out = cmd.output().await?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let mut msg = String::from_utf8_lossy(&out.stderr).into_owned();
        if msg.trim().is_empty() {
            msg = String::from_utf8_lossy(&out.stdout).into_owned();
        }
        Err(anyhow!(
            "{} {} failed (code {:?}):\n{}",
            bin.display(),
            pretty_args(args),
            out.status.code(),
            msg
        ))
    }
}

pub async fn run_capture_to_file<P: AsRef<OsStr>>(
    bin: &Path,
    args: &[P],
    envs: Option<&HashMap<String, String>>,
    file: &Path,
) -> Result<String> {
    let s = run_capture(bin, args, envs).await?;
    fs::write(file, &s)?;
    Ok(s)
}

pub fn tail_file(path: &Path, last_lines: usize) -> String {
    match fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(last_lines);
            lines[start..].join("\n")
        }
        Err(_) => String::new(),
    }
}

pub fn pretty_args<P: AsRef<OsStr>>(args: &[P]) -> String {
    args.iter()
        .map(|a| {
            let s = a.as_ref().to_string_lossy();
            shell_quote(&s)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

pub fn kv_env(base: &HashMap<String, String>, add: &[(&str, String)]) -> HashMap<String, String> {
    let mut m = base.clone();
    for (k, v) in add {
        m.insert((*k).to_string(), v.clone());
    }
    m
}
