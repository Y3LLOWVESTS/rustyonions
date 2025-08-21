use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgo {
    Sha256,
    Blake3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentHash {
    pub algo: HashAlgo,
    #[serde(with = "hex::serde")]
    pub digest: Vec<u8>,
}

impl ContentHash {
    pub fn to_hex(&self) -> String {
        hex::encode(&self.digest)
    }
}

#[derive(Debug, Error)]
pub enum HashError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub trait Hasher {
    fn hash_bytes(algo: HashAlgo, bytes: &[u8]) -> ContentHash;
    fn hash_file<P: AsRef<Path>>(algo: HashAlgo, path: P) -> Result<ContentHash, HashError>;
}

pub struct DefaultHasher;

impl Hasher for DefaultHasher {
    fn hash_bytes(algo: HashAlgo, bytes: &[u8]) -> ContentHash {
        match algo {
            HashAlgo::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(bytes);
                let digest = hasher.finalize().to_vec();
                ContentHash { algo, digest }
            }
            HashAlgo::Blake3 => {
                let digest = blake3::hash(bytes).as_bytes().to_vec();
                ContentHash { algo, digest }
            }
        }
    }

    fn hash_file<P: AsRef<Path>>(algo: HashAlgo, path: P) -> Result<ContentHash, HashError> {
        let mut f = fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(Self::hash_bytes(algo, &buf))
    }
}
