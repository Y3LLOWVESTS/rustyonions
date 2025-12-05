use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    // TODO: add node registry, metrics buffers, auth caches, etc.
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}
