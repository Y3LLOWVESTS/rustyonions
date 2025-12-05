use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub display_name: String,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaneStatus {
    pub name: String,
    pub health: String,
    pub ready: bool,
    pub restart_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStatusView {
    pub id: String,
    pub display_name: String,
    pub profile: Option<String>,
    pub version: Option<String>,
    pub planes: Vec<PlaneStatus>,
}

impl AdminStatusView {
    pub fn placeholder() -> Self {
        Self {
            id: "example-node".into(),
            display_name: "Example Node".into(),
            profile: Some("macronode".into()),
            version: Some("0.0.0".into()),
            planes: vec![],
        }
    }
}
