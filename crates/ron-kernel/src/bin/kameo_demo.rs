use anyhow::Result;
use kameo::{spawn, Actor, Ask, Context};
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

// A simple message type for our demo.
#[derive(Debug)]
struct Bump(u64);

// A demo actor with a counter.
struct Demo {
    count: u64,
}

impl Demo {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for Demo {
    fn handle_string<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        msg: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            println!("[actor] got string: {msg}");
            Ok(())
        })
    }

    fn handle_ask_env<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        ask: Ask<&'static str, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let val = std::env::var(ask.req).unwrap_or_default();
            let _ = ask.tx.send(val);
            Ok(())
        })
    }

    fn handle_message<'a, M: Send + 'static>(
        &'a mut self,
        _ctx: &'a mut Context,
        msg: M,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // handle our Bump message; ignore unknown types (they won't be sent in this demo)
            if let Some(Bump(n)) = any_as_ref::<M, Bump>(&msg) {
                self.count += n;
                println!("[actor] bump by {n}, total={}", self.count);
            }
            Ok(())
        })
    }
}

// Tiny helper to downcast-by-reference for demo purposes.
fn any_as_ref<T: 'static, U: 'static>(t: &T) -> Option<&U> {
    use std::any::Any;
    (t as &dyn Any).downcast_ref::<U>()
}

#[tokio::main]
async fn main() -> Result<()> {
    // logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // spawn actor
    let (addr, _task) = spawn::<Bump, _>(Demo::new());

    // send a few messages
    addr.send_str("hello actor").await?;
    addr.send(Bump(5)).await?;
    addr.send(Bump(7)).await?;

    // ask for an env var
    std::env::set_var("DEMO_ENV", "kameo-works");
    let v = addr.ask_env("DEMO_ENV").await?;
    println!("[main] ask_env(DEMO_ENV) -> {v}");

    // give actor a moment to print
    sleep(Duration::from_millis(50)).await;

    Ok(())
}
