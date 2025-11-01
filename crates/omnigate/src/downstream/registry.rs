//! RO:WHAT   In-memory registry mapping ServiceKind -> Endpoint config.
//! RO:WHY    Late-binding of base URLs and timeouts; simple and explicit.
//! RO:INVARS  Base URLs are absolute; timeouts finite; defaults are localhost dev-safe.

use crate::downstream::types::ServiceKind;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EndpointCfg {
    pub base_url: String,          // e.g., "http://127.0.0.1:5304"
    pub connect_timeout: Duration, // TCP connect
    pub timeout: Duration,         // whole request
}

#[derive(Debug, Clone)]
pub struct DownstreamRegistry {
    map: HashMap<ServiceKind, EndpointCfg>,
}

impl DownstreamRegistry {
    pub fn get(&self, k: ServiceKind) -> Option<&EndpointCfg> {
        self.map.get(&k)
    }

    pub fn insert(&mut self, k: ServiceKind, e: EndpointCfg) {
        self.map.insert(k, e);
    }
}

impl Default for DownstreamRegistry {
    fn default() -> Self {
        use ServiceKind::*;
        let mut map = HashMap::new();
        let fast = EndpointCfg {
            base_url: "http://127.0.0.1:5300".into(),
            connect_timeout: Duration::from_millis(200),
            timeout: Duration::from_secs(2),
        };
        // Dev-safe placeholders; change per crate ports as needed.
        map.insert(Index,   EndpointCfg { base_url: "http://127.0.0.1:5304".into(), ..fast.clone() });
        map.insert(Storage, EndpointCfg { base_url: "http://127.0.0.1:5303".into(), ..fast.clone() });
        map.insert(Dht,     EndpointCfg { base_url: "http://127.0.0.1:5301".into(), ..fast.clone() });
        map.insert(Naming,  EndpointCfg { base_url: "http://127.0.0.1:5302".into(), ..fast.clone() });
        map.insert(Overlay, EndpointCfg { base_url: "http://127.0.0.1:5306".into(), ..fast.clone() });
        map.insert(Policy,  EndpointCfg { base_url: "http://127.0.0.1:9609".into(), ..fast.clone() });
        Self { map }
    }
}
