//! RO:WHAT — Cancel tokens & helpers (async-drop friendly).
//! RO:WHY  — Crash-only supervision + graceful shutdown.

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Cancel {
    token: CancellationToken,
}
impl Cancel {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }
    pub fn child(&self) -> Self {
        Self {
            token: self.token.child_token(),
        }
    }
    pub fn cancel(&self) {
        self.token.cancel();
    }
    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }
}
