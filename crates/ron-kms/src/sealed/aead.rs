#![cfg(feature = "soft-seal")]
#![forbid(unsafe_code)]

use crate::sealed::header::{Header, MAGIC, VERSION};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand_core::{OsRng, RngCore};
use std::time::{SystemTime, UNIX_EPOCH};

/// Seal `plaintext` with ChaCha20-Poly1305 under `key` and optional `aad`.
/// Returns `[header || ciphertext || tag]`.
#[must_use]
pub fn seal(key: &[u8; 32], plaintext: &[u8], aad: &[u8]) -> Vec<u8> {
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);

    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let hdr = Header {
        magic: *MAGIC,
        version: VERSION,
        ts_ms,
        nonce: nonce_bytes,
    };

    // Encode header once
    let mut hdr_buf = Vec::with_capacity(Header::LEN);
    hdr.encode_into(&mut hdr_buf);

    // AAD = header bytes || external AAD (must match unseal’s construction)
    let mut aad_buf = Vec::with_capacity(Header::LEN + aad.len());
    aad_buf.extend_from_slice(&hdr_buf);
    aad_buf.extend_from_slice(aad);

    // AEAD (expects references to Key/Nonce); avoid deprecated from_slice()
    let key_ga: Key = Key::from(*key);
    let nonce_ga: Nonce = Nonce::from(nonce_bytes);
    let aead = ChaCha20Poly1305::new(&key_ga);

    let payload = Payload {
        msg: plaintext,
        aad: &aad_buf,
    };
    let ct = aead
        .encrypt(&nonce_ga, payload)
        .expect("ChaCha20Poly1305 encrypt");

    // Final output = header || ciphertext||tag
    let mut out = hdr_buf;
    out.extend_from_slice(&ct);
    out
}

/// Unseal `[header || ciphertext || tag]` with `key` and `aad`.
/// On success, returns the plaintext bytes.
pub fn unseal(key: &[u8; 32], sealed: &[u8], aad: &[u8]) -> Result<Vec<u8>, UnsealError> {
    if sealed.len() < Header::LEN + 16 {
        return Err(UnsealError::Format);
    }

    let (hdr_bytes, body) = sealed.split_at(Header::LEN);
    let hdr = Header::decode(hdr_bytes).ok_or(UnsealError::Format)?;

    if hdr.magic != *MAGIC || hdr.version != VERSION {
        return Err(UnsealError::Format);
    }

    // Optional anti-rollback hook (currently permissive).
    if !super::anti_rollback::check_ts(hdr.ts_ms) {
        return Err(UnsealError::Rejected);
    }

    // AEAD (expects references to Key/Nonce); avoid deprecated from_slice()
    let key_ga: Key = Key::from(*key);
    let nonce_ga: Nonce = Nonce::from(hdr.nonce);
    let aead = ChaCha20Poly1305::new(&key_ga);

    // AAD must exactly match what was used during seal.
    let mut aad_buf = Vec::with_capacity(Header::LEN + aad.len());
    aad_buf.extend_from_slice(hdr_bytes);
    aad_buf.extend_from_slice(aad);

    let payload = Payload {
        msg: body,
        aad: &aad_buf,
    };
    let pt = aead
        .decrypt(&nonce_ga, payload)
        .map_err(|_| UnsealError::Auth)?;
    Ok(pt)
}

#[derive(Debug, thiserror::Error)]
pub enum UnsealError {
    #[error("format error")]
    Format,
    #[error("authentication failed")]
    Auth,
    #[error("rejected by policy")]
    Rejected,
}
