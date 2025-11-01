//! RO:WHAT   Core enums/types for downstream services.
//! RO:WHY    Keep labels low-cardinality and stable across releases.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceKind {
    Index,
    Storage,
    Dht,
    Naming,
    Overlay,
    Policy, // if ever queried (bundle host, etc.)
}

impl ServiceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceKind::Index => "index",
            ServiceKind::Storage => "storage",
            ServiceKind::Dht => "dht",
            ServiceKind::Naming => "naming",
            ServiceKind::Overlay => "overlay",
            ServiceKind::Policy => "policy",
        }
    }
}
