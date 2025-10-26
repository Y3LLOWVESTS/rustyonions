//! RO:WHAT — ron-transport listener (placeholder).
//! RO:NEXT — Replace delegation with real spawn using ron-transport once stream handoff API is set.

use crate::admin::ReadyProbe;
use crate::config::Config;
use anyhow::Result;

// For now, delegate to plain listener so enabling the feature doesn't break runtime.
// We keep the same public surface (spawn_listener, ListenerHandle).
pub(super) use super::plain::{spawn_listener, ListenerHandle};
