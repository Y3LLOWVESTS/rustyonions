use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub alg: String,
    pub kid: String,
    pub sig_b64: String,
    pub msg_b64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    pub envelope: Envelope,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub ok: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyBatchRequest {
    pub envelopes: Vec<Envelope>,
}

#[derive(Debug, Serialize)]
pub struct VerifyBatchResponse {
    pub results: Vec<VerifyResult>,
}

#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub ok: bool,
    pub reason: Option<String>,
}
