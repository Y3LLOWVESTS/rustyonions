use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub struct Readiness {
    config_loaded: AtomicBool,
    listeners_bound: AtomicBool,
}

impl Readiness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_config_loaded(&self, v: bool) {
        self.config_loaded.store(v, Ordering::Relaxed);
    }

    pub fn set_listeners_bound(&self, v: bool) {
        self.listeners_bound.store(v, Ordering::Relaxed);
    }

    /// Minimal health: all invariants that should be up even when not "ready".
    pub fn health_ok(&self) -> bool {
        // For now, "healthy" if the process is running; later include store checks.
        true
    }

    /// Ready when config is loaded and listeners are bound.
    pub fn all_ready(&self) -> bool {
        self.config_loaded.load(Ordering::Relaxed) && self.listeners_bound.load(Ordering::Relaxed)
    }
}
