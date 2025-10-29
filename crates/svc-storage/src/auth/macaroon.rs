//! RO:WHAT — Minimal macaroon-ish token + Axum extractor, signed with keyed BLAKE3.
//! RO:WHY  — Gate write endpoints without heavy deps or external KMS.
//! RO:NOTE — If `RON_STORAGE_MACAROON_SECRET` is unset, extractor is permissive (dev mode).

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;

#[derive(Debug)]
pub enum MacaroonError {
    Missing,
    Malformed,
    Expired,
    BadSig,
    Internal,
}

impl IntoResponse for MacaroonError {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            MacaroonError::Missing => (StatusCode::UNAUTHORIZED, "missing authorization"),
            MacaroonError::Malformed => (StatusCode::UNAUTHORIZED, "malformed authorization"),
            MacaroonError::Expired => (StatusCode::UNAUTHORIZED, "token expired"),
            MacaroonError::BadSig => (StatusCode::FORBIDDEN, "bad signature"),
            MacaroonError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "auth internal"),
        };
        (code, msg).into_response()
    }
}

#[derive(Debug, Clone)]
pub struct MacaroonClaims {
    pub issued_at: u64,
    pub expires_at: u64,
}

/// 32-byte key from env (base64url, no padding). None => dev/permissive.
fn secret_from_env() -> Option<[u8; 32]> {
    let raw = std::env::var("RON_STORAGE_MACAROON_SECRET").ok()?;
    let mut key = [0u8; 32];
    let decoded = B64.decode(raw).ok()?;
    if decoded.len() != 32 {
        return None;
    }
    key.copy_from_slice(&decoded);
    Some(key)
}

fn leeway_from_env() -> u64 {
    std::env::var("RON_STORAGE_MACAROON_LEEWAY")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
}

fn now_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Token inner text: `v=1;ts=<u64>;exp=<u64>;sig=<hex>`
fn parse_token_fields(token: &str) -> Option<(u64, u64, &str)> {
    let mut v = None;
    let mut ts = None;
    let mut exp = None;
    let mut sig = None;
    for part in token.split(';') {
        let (k, val) = part.split_once('=')?;
        match k {
            "v" if val == "1" => v = Some(1u8),
            "ts" => ts = val.parse::<u64>().ok(),
            "exp" => exp = val.parse::<u64>().ok(),
            "sig" => sig = Some(val),
            _ => {}
        }
    }
    match (v, ts, exp, sig) {
        (Some(_), Some(ts), Some(exp), Some(sig)) => Some((ts, exp, sig)),
        _ => None,
    }
}

/// BLAKE3 keyed MAC as lowercase hex string.
fn mac_hex(key: &[u8; 32], msg: &str) -> String {
    let mac = blake3::keyed_hash(key, msg.as_bytes());
    mac.to_hex().to_string()
}

fn verify_impl_with_header(authz: &str) -> Result<MacaroonClaims, MacaroonError> {
    // Accept "Macaroon ..." or "Bearer ..."
    let token = authz
        .strip_prefix("Macaroon ")
        .or_else(|| authz.strip_prefix("Bearer "))
        .ok_or(MacaroonError::Malformed)?;

    let decoded = B64.decode(token).map_err(|_| MacaroonError::Malformed)?;
    let text = std::str::from_utf8(&decoded).map_err(|_| MacaroonError::Malformed)?;

    let (ts, exp, sig_hex) = parse_token_fields(text).ok_or(MacaroonError::Malformed)?;
    let now = now_unix();
    let leeway = leeway_from_env();

    if exp + leeway < now {
        return Err(MacaroonError::Expired);
    }
    if ts > now + 24 * 3600 {
        return Err(MacaroonError::Malformed);
    }

    // Canonical string to MAC
    let msg = format!("v=1|ts={}|exp={}", ts, exp);

    let key = secret_from_env().ok_or(MacaroonError::Internal)?;
    let expect = mac_hex(&key, &msg);

    // Constant-time-ish comparison
    if expect.len() != sig_hex.len() {
        return Err(MacaroonError::BadSig);
    }
    let mut diff = 0u8;
    for (a, b) in expect.as_bytes().iter().zip(sig_hex.as_bytes()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return Err(MacaroonError::BadSig);
    }

    Ok(MacaroonClaims {
        issued_at: ts,
        expires_at: exp,
    })
}

#[async_trait]
impl<S> FromRequestParts<S> for MacaroonClaims
where
    S: Send + Sync,
{
    type Rejection = MacaroonError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // DEV-PERMISSIVE PATH: if no secret is configured, allow even with no header.
        if secret_from_env().is_none() {
            let n = now_unix();
            return Ok(MacaroonClaims {
                issued_at: n,
                expires_at: n + 300,
            });
        }

        // ENFORCED PATH: secret is configured -> require header and verify.
        let auth = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or(MacaroonError::Missing)?;
        let auth = auth.to_str().map_err(|_| MacaroonError::Malformed)?;
        verify_impl_with_header(auth)
    }
}
