//! Minimal actor helpers used by the demos.
//! Design goals:
//! - No async_trait needed
//! - No reliance on Sender::close / close_channel (just drop to close)
//! - A small “mailbox” with three message kinds: String, Ask-env, and a generic user message M

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// Ask pattern: send a request and receive a typed response.
pub struct Ask<Req, Resp> {
    pub req: Req,
    pub tx: oneshot::Sender<Resp>,
}

/// Minimal actor runtime context (extend as needed later).
#[derive(Default)]
pub struct Context;

impl Context {
    pub fn new() -> Self {
        Context
    }
}

/// Messages carried by the mailbox.
/// - String: basic fire-and-forget string
/// - AskEnv: ask for an env value (&'static str -> String)
/// - Custom(M): user-defined message type
pub enum Mailbox<M> {
    Str(String),
    AskEnv(Ask<&'static str, String>),
    Custom(M),
}

/// Address/handle to an actor’s mailbox.
pub struct Addr<M> {
    tx: mpsc::Sender<Mailbox<M>>,
}

impl<M> Clone for Addr<M> {
    fn clone(&self) -> Self {
        Addr {
            tx: self.tx.clone(),
        }
    }
}

impl<M: Send + 'static> Addr<M> {
    /// Send a user-defined message.
    pub async fn send(&self, msg: M) -> Result<()> {
        self.tx
            .send(Mailbox::Custom(msg))
            .await
            .map_err(|e| anyhow::anyhow!("mailbox closed: {e}"))?;
        Ok(())
    }

    /// Send a string message.
    pub async fn send_str(&self, s: impl Into<String>) -> Result<()> {
        self.tx
            .send(Mailbox::Str(s.into()))
            .await
            .map_err(|e| anyhow::anyhow!("mailbox closed: {e}"))?;
        Ok(())
    }

    /// Ask for an env var (demo of request/response pattern).
    pub async fn ask_env(&self, key: &'static str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Mailbox::AskEnv(Ask { req: key, tx }))
            .await
            .map_err(|e| anyhow::anyhow!("mailbox closed: {e}"))?;
        let v = rx
            .await
            .map_err(|e| anyhow::anyhow!("actor dropped before responding: {e}"))?;
        Ok(v)
    }

    /// Whether the channel is closed (all receivers dropped).
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// Actor trait:
/// - `handle_message` handles the user message type `M`
/// - `handle_string` handles string messages
/// - `handle_ask_env` handles the Ask<&'static str, String> pattern
///
/// All have default no-op implementations so you can implement only what you need.
pub trait Actor: Send + 'static {
    fn handle_message<'a, M: Send + 'static>(
        &'a mut self,
        _ctx: &'a mut Context,
        _msg: M,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn handle_string<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        _msg: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn handle_ask_env<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        _ask: Ask<&'static str, String>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// Spawn an actor with a mailbox for messages of type `M`.
pub fn spawn<M, A>(mut actor: A) -> (Addr<M>, JoinHandle<()>)
where
    M: Send + 'static,
    A: Actor + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel::<Mailbox<M>>(64);
    let addr = Addr { tx };

    let handle = tokio::spawn(async move {
        let mut ctx = Context::new();

        while let Some(msg) = rx.recv().await {
            match msg {
                Mailbox::Str(m) => {
                    if let Err(e) = actor.handle_string(&mut ctx, m).await {
                        tracing::warn!("actor string handler error: {e:?}");
                    }
                }
                Mailbox::AskEnv(ask) => {
                    if let Err(e) = actor.handle_ask_env(&mut ctx, ask).await {
                        tracing::warn!("actor ask handler error: {e:?}");
                    }
                }
                Mailbox::Custom(m) => {
                    if let Err(e) = actor.handle_message(&mut ctx, m).await {
                        tracing::warn!("actor custom handler error: {e:?}");
                    }
                }
            }
        }
        // mailboxes closed; actor task ends
    });

    (addr, handle)
}
