//! Transport load generator
//! Usage:
//!   cargo run -p ron-kernel --bin transport_load -- <ADDR> [CONNS] [MSGS_PER_CONN] [CONCURRENCY]
//! Example:
//!   cargo run -p ron-kernel --bin transport_load -- 127.0.0.1:50225 1000 10 200

use std::{
    env,
    io,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Semaphore,
    task::JoinSet,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let addr = args.next().unwrap_or_else(|| {
        eprintln!("USAGE: transport_load <ADDR> [CONNS] [MSGS_PER_CONN] [CONCURRENCY]");
        std::process::exit(2);
    });
    let total_conns: usize = args.next().unwrap_or_else(|| "1000".into()).parse().unwrap_or(1000);
    let msgs_per_conn: usize = args.next().unwrap_or_else(|| "10".into()).parse().unwrap_or(10);
    let concurrency: usize = args.next().unwrap_or_else(|| "200".into()).parse().unwrap_or(200);

    let limiter = Arc::new(Semaphore::new(concurrency));
    let started = Instant::now();

    // Counters must be Arc to satisfy 'static for spawned tasks.
    let ok_conns = Arc::new(AtomicU64::new(0));
    let fail_conns = Arc::new(AtomicU64::new(0));
    let bytes_written = Arc::new(AtomicU64::new(0));
    let bytes_read = Arc::new(AtomicU64::new(0));

    let mut joinset: JoinSet<io::Result<()>> = JoinSet::new();

    for _ in 0..total_conns {
        let limiter = limiter.clone();
        let addr = addr.clone();
        let bw = bytes_written.clone();
        let br = bytes_read.clone();

        joinset.spawn(async move {
            // Acquire a concurrency slot for this connection
            let _permit = limiter.acquire_owned().await.unwrap();
            run_one(&addr, msgs_per_conn, &bw, &br).await
        });
    }

    while let Some(res) = joinset.join_next().await {
        match res {
            Ok(Ok(())) => {
                ok_conns.fetch_add(1, Ordering::Relaxed);
            }
            Ok(Err(_)) | Err(_) => {
                fail_conns.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    let elapsed = started.elapsed().as_secs_f64();
    let oks = ok_conns.load(Ordering::Relaxed);
    let fails = fail_conns.load(Ordering::Relaxed);
    let bw = bytes_written.load(Ordering::Relaxed);
    let br = bytes_read.load(Ordering::Relaxed);
    let reqs = oks as f64 * msgs_per_conn as f64;

    println!("=== transport_load summary ===");
    println!("address:          {}", addr);
    println!("connections:      ok={} fail={}", oks, fails);
    println!("messages sent:    {:.0}", reqs);
    println!("bytes written:    {}", bw);
    println!("bytes read:       {}", br);
    println!("elapsed (s):      {:.3}", elapsed);
    println!("msgs/sec:         {:.1}", reqs / elapsed);
    println!("bytes/sec (in):   {:.1}", br as f64 / elapsed);
    println!("bytes/sec (out):  {:.1}", bw as f64 / elapsed);

    Ok(())
}

async fn run_one(
    addr: &str,
    msgs: usize,
    bytes_written: &Arc<AtomicU64>,
    bytes_read: &Arc<AtomicU64>,
) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr).await?;

    let mut buf = vec![0u8; 1024];

    for _ in 0..msgs {
        let payload = b"ping\n";
        stream.write_all(payload).await?;
        bytes_written.fetch_add(payload.len() as u64, Ordering::Relaxed);

        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server closed"));
        }
        bytes_read.fetch_add(n as u64, Ordering::Relaxed);
    }

    let _ = stream.shutdown().await;
    Ok(())
}
