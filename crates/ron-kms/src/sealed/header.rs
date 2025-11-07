#![cfg(feature = "soft-seal")]
#![forbid(unsafe_code)]

// Fixed header: [MAGIC:4][VER:1][TS_MS:8][NONCE:12]
pub const MAGIC: &[u8; 4] = b"RKMS";
pub const VERSION: u8 = 1;

#[derive(Clone, Copy)]
pub struct Header {
    pub magic: [u8; 4],
    pub version: u8,
    pub ts_ms: i64,
    pub nonce: [u8; 12],
}

impl Header {
    pub const LEN: usize = 4 + 1 + 8 + 12;

    pub fn encode_into(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.magic);
        buf.push(self.version);
        buf.extend_from_slice(&self.ts_ms.to_be_bytes());
        buf.extend_from_slice(&self.nonce);
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() != Self::LEN {
            return None;
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buf[0..4]);
        let version = buf[4];
        let mut tsb = [0u8; 8];
        tsb.copy_from_slice(&buf[5..13]);
        let ts_ms = i64::from_be_bytes(tsb);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&buf[13..25]);
        Some(Self {
            magic,
            version,
            ts_ms,
            nonce,
        })
    }
}
