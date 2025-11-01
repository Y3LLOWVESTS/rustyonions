//! RO:WHAT — Cooperative shutdown primitives.
//! RO:WHY  — A tiny CancellationToken wrapper so workers/supervisor can agree on quit.
//! RO:INVARIANTS — Non-blocking; clone is cheap; `cancelled()` is awaitable.

use tokio_util::sync::CancellationToken;

/// Handle that tasks can hold/clone to observe shutdown.
#[derive(Clone)]
pub struct Shutdown {
    token: CancellationToken,
}

/// Trigger used by the supervisor to request shutdown.
#[derive(Clone)]
pub struct ShutdownTrigger {
    token: CancellationToken,
}

/// Construct a (Shutdown, ShutdownTrigger) pair.
pub fn pair() -> (Shutdown, ShutdownTrigger) {
    let token = CancellationToken::new();
    (
        Shutdown {
            token: token.clone(),
        },
        ShutdownTrigger { token },
    )
}

impl Shutdown {
    /// Wait until shutdown is requested.
    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }
}

impl ShutdownTrigger {
    /// Request shutdown for all holders of the paired `Shutdown`.
    pub fn cancel(&self) {
        self.token.cancel();
    }
}
