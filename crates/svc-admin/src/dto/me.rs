use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub subject: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub auth_mode: String,
    pub login_url: Option<String>,
}

impl MeResponse {
    pub fn dev_default() -> Self {
        Self {
            subject: "dev-operator".into(),
            display_name: "Dev Operator".into(),
            roles: vec!["admin".into()],
            auth_mode: "none".into(),
            login_url: None,
        }
    }
}
