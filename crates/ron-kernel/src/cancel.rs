use tokio_util::sync::CancellationToken;

/// Graceful shutdown token shared across services.
#[derive(Clone)]
pub struct Shutdown {
    token: CancellationToken,
}

impl Shutdown {
    pub fn new() -> Self {
        Self { token: CancellationToken::new() }
    }

    pub fn child(&self) -> Self {
        Self { token: self.token.child_token() }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }
}
