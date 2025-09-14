// testing/gwsmoke/src/proc.rs
use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write},
    path::Path,
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    task::JoinHandle,
    time::{timeout, Duration},
};

pub struct ChildProc {
    #[allow(dead_code)] // example harness doesn't read this field yet; keep for diagnostics
    pub name: String,
    pub child: Child,
    _stdout_task: JoinHandle<io::Result<()>>,
    _stderr_task: JoinHandle<io::Result<()>>,
}

impl ChildProc {
    pub async fn kill_and_wait(mut self) -> Result<()> {
        let _ = self.child.start_kill();
        let _ = timeout(Duration::from_secs(3), self.child.wait()).await;
        Ok(())
    }
}

pub async fn spawn_logged(
    name: &str,
    bin: &Path,
    log_path: &Path,
    envs: &HashMap<String, String>,
    args: &[&str],
    stream_to_stdout: bool,
) -> Result<ChildProc> {
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(a);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("spawn {}", name))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("{}: no stdout", name))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("{}: no stderr", name))?;

    // Append logs
    let mut out_file =
        File::create(log_path).with_context(|| format!("open {}", log_path.display()))?;
    let mut err_file = out_file.try_clone()?;

    let name_out = name.to_string();
    let name_err = name.to_string();

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stream_to_stdout {
                println!("[{}] {}", name_out, line);
            }
            let _ = writeln!(out_file, "{}", line);
        }
        Ok::<_, io::Error>(())
    });
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stream_to_stdout {
                eprintln!("[{}] {}", name_err, line);
            }
            let _ = writeln!(err_file, "{}", line);
        }
        Ok::<_, io::Error>(())
    });

    println!(
        "+ {} {}",
        bin.display(),
        args.iter()
            .map(|s| {
                if s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
                {
                    s.to_string()
                } else {
                    format!("'{}'", s.replace('\'', "'\\''"))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    );

    Ok(ChildProc {
        name: name.to_string(),
        child,
        _stdout_task: stdout_task,
        _stderr_task: stderr_task,
    })
}
