//! RO:WHAT — Ergonomic re-exports for common ryker types.
//! RO:WHY  — DX; fewer deep module paths for apps embedding ryker.
//! RO:INTERACTS — re-exports from config, runtime, mailbox, supervisor.
//! RO:INVARIANTS — re-export only stable, documented surface.

pub use crate::config::RykerConfig;
pub use crate::errors::{Error, Result};
pub use crate::mailbox::{Mailbox, MailboxBuilder};
pub use crate::runtime::Runtime;
pub use crate::supervisor::Supervisor;
