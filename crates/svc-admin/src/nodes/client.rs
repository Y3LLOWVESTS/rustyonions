use crate::error::Result;

// TODO: wrap reqwest::Client and talk to node admin endpoints.
pub struct NodeClient;

impl NodeClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn ping_node(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
