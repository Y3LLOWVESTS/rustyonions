use rand::{Rng, RngCore};
use std::collections::HashMap;

/// A single decoy asset that looks like a real object.
#[derive(Clone)]
pub struct DecoyAsset {
    pub id: String,           // e.g., "b3:<hex>.tld"
    pub content_type: String, // plausible MIME
    pub bytes: Vec<u8>,       // small but streamable
}

#[derive(Default)]
pub struct DecoyCatalog {
    by_id: HashMap<String, DecoyAsset>,
}

impl DecoyCatalog {
    pub fn generate<R: Rng + ?Sized>(rng: &mut R, count: usize) -> Self {
        let mut by_id = HashMap::with_capacity(count);
        for _ in 0..count {
            let size = rng.gen_range(32_000..96_000); // 32–96 KiB
            let mut buf = vec![0u8; size];
            rng.fill_bytes(&mut buf);
            // fabricate a plausible b3-like id (no need to compute real BLAKE3 here)
            let mut id_bytes = [0u8; 32];
            rng.fill_bytes(&mut id_bytes);
            let hex = hex::encode(id_bytes);
            let id = format!("b3:{hex}.tld");
            let content_type = if rng.gen_bool(0.5) { "application/octet-stream" } else { "application/x-ron-bundle" }.to_string();
            let asset = DecoyAsset { id: id.clone(), content_type, bytes: buf };
            by_id.insert(id, asset);
        }
        Self { by_id }
    }

    pub fn get(&self, id: &str) -> Option<DecoyAsset> {
        self.by_id.get(id).cloned()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }
}

// Minimal embedded hex encoder to avoid extra deps if workspace lacks `hex`.
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
