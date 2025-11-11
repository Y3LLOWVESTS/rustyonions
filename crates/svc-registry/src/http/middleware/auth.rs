//! Auth stub (foundation keeps public GETs unauthenticated).
#[derive(Clone, Default)]
pub struct AuthCfg {
    pub enabled: bool,
}

// Placeholder for future macaroon / UDS gating.
