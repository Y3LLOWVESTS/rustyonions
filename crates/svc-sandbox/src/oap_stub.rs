use serde::Serialize;

pub const OAP1_MAX_FRAME: usize = 1_048_576; // 1 MiB

#[derive(Debug)]
pub enum HandshakeError {
    Empty,
    TooShort,
    Malformed,
}

impl HandshakeError {
    pub fn code(&self) -> &'static str {
        match self {
            HandshakeError::Empty => "empty",
            HandshakeError::TooShort => "too_short",
            HandshakeError::Malformed => "malformed",
        }
    }
}

#[derive(Serialize)]
pub struct HandshakeAck {
    pub ok: bool,
    pub proto: &'static str,
    pub session_id: String,
}

pub fn handshake_stub(frame: &[u8]) -> Result<HandshakeAck, HandshakeError> {
    if frame.is_empty() {
        return Err(HandshakeError::Empty);
    }
    if frame.len() < 8 {
        return Err(HandshakeError::TooShort);
    }
    // Very lightweight "parse": this is a stub; we intentionally avoid real data-plane coupling.
    // Mix a few bytes to fabricate a session id.
    let sid = blake3_stub(frame);
    Ok(HandshakeAck { ok: true, proto: "OAP/1", session_id: sid })
}

// Small, fast hash stub to avoid pulling blake3 if workspace doesn't have it.
fn blake3_stub(input: &[u8]) -> String {
    let mut x: u64 = 0xcbf29ce484222325;
    for &b in input {
        x ^= b as u64;
        x = x.wrapping_mul(0x100000001b3);
    }
    // hex-16
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = ((x >> ((i % 8) * 8)) & 0xff) as u8;
    }
    hex::encode(&out)
}

mod hex {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let b = bytes.as_ref();
        let mut out = vec![0u8; b.len() * 2];
        for (i, &v) in b.iter().enumerate() {
            out[i * 2] = LUT[(v >> 4) as usize];
            out[i * 2 + 1] = LUT[(v & 0x0f) as usize];
        }
        unsafe { String::from_utf8_unchecked(out) }
    }
}
