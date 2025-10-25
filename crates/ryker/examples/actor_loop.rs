// RO:WHAT  — Minimal actor loop using Runtime + Mailbox + Supervisor.
// RO:HOW   — cargo run -p ryker --example actor_loop

use ryker::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone)]
struct Msg(&'static str);

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cfg = ryker::config::from_env_validated()?;
    let rt = Runtime::new(cfg);

    // Build a small mailbox to demo Busy behavior.
    let mb: ryker::mailbox::Mailbox<Msg> = rt
        .mailbox("demo-actor")
        .capacity(4)
        .deadline(Duration::from_millis(250))
        .build();

    // Enqueue a few messages; you should see Busy when capacity is exceeded.
    for i in 0..8 {
        match mb.try_send(Msg("hello")) {
            Ok(_) => println!("[main] enqueued i={i}"),
            Err(ryker::mailbox::MailboxError::Busy) => {
                println!("[main] queue Busy at i={i} (reject-new)")
            }
            Err(e) => println!("[main] enqueue error at i={i}: {e}"),
        }
    }

    // Hand-off ownership of the mailbox to the supervised actor exactly once.
    // The FnMut factory can be called multiple times by Supervisor after failures;
    // we use Option.take() so only the first call consumes the mailbox.
    let mut rx_opt = Some(mb);
    let sup = Supervisor::new(rt.config());
    let _handle = sup.spawn(move || {
        // Take the mailbox on the first invocation; None thereafter (no restart).
        let rx_taken = rx_opt.take();
        async move {
            if let Some(mut rx) = rx_taken {
                loop {
                    match rx.pull().await {
                        Ok(Msg(s)) => println!("[actor] handled: {s}"),
                        Err(ryker::mailbox::MailboxError::Timeout) => {
                            println!("[actor] idle timeout");
                        }
                        Err(ryker::mailbox::MailboxError::Closed) => break,
                        Err(e) => eprintln!("[actor] error: {e}"),
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        }
    });

    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}
