use crate::{address::Address, hash::ContentHash, tld::TldType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Blob,       // raw bytes: image, video, etc.
    Text,       // UTF-8 text post/comment
    Directory,  // future multi-file
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentRef {
    /// Address of the parent item (e.g., a comment’s post).
    pub address: String,
    /// Optional typed relation (e.g., "reply_to", "annotation_of")
    #[serde(default)]
    pub relation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Signatures {
    /// Base64 of signature bytes, scheme TBD later.
    #[serde(default)]
    pub owner_sig: Option<String>,
    #[serde(default)]
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u16,              // bump when we change shape
    pub id: Uuid,                          // local build id
    pub tld: TldType,                      // .image, .post, …
    pub address: String,                   // "<sha>.tld"
    pub hash: ContentHash,                 // canonical content hash
    pub kind: ContentKind,                 // blob/text/dir
    pub mime: Option<String>,              // best guess
    pub size: u64,                         // payload size in bytes
    pub created_at: DateTime<Utc>,         // when this manifest was created
    pub origin_pubkey: Option<String>,     // creator/owner pubkey (hex/base58)
    pub owner_addr: Option<String>,        // wallet address (for payouts/attribution)
    pub license: Option<String>,           // SPDX or free-form
    #[serde(default)]
    pub tags: Vec<String>,                 // free-form tags
    #[serde(default)]
    pub parents: Vec<ParentRef>,           // relationships (e.g., comment->post)
    #[serde(default)]
    pub signatures: Signatures,            // optional sigs
}

impl Manifest {
    pub fn to_toml_string(&self) -> String {
        toml::to_string_pretty(self).expect("manifest to toml")
    }
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct PackingPlan {
    /// Directory that will contain `Manifest.toml` and payload.
    pub out_dir: PathBuf,
    /// Basename for payload.
    pub payload_name: String, // e.g. "payload.bin" or "content"
}

impl PackingPlan {
    pub fn default_for(addr: &Address, base_dir: impl AsRef<Path>) -> Self {
        let out_dir = base_dir.as_ref().join(addr.to_string_addr());
        Self { out_dir, payload_name: "payload.bin".into() }
    }
}

/// Build and write a manifest + copy payload into the bundle directory.
/// Returns the path to `Manifest.toml`.
pub fn pack_bundle(
    addr: &Address,
    tld: TldType,
    hash: ContentHash,
    payload_src: impl AsRef<Path>,
    owner_addr: Option<String>,
    origin_pubkey: Option<String>,
    base_dir: impl AsRef<Path>,
) -> Result<PathBuf, PackError> {
    let src = payload_src.as_ref();
    let size = fs::metadata(src)?.len();
    let mime_guess = mime_guess::from_path(src).first_raw().map(|s| s.to_string());

    let manifest = Manifest {
        schema_version: 1,
        id: Uuid::new_v4(),
        tld,
        address: addr.to_string_addr(),
        hash,
        kind: infer_kind_from_tld(tld, mime_guess.as_deref()),
        mime: mime_guess,
        size,
        created_at: chrono::Utc::now(),
        origin_pubkey,
        owner_addr,
        license: None,
        tags: vec![],
        parents: vec![],
        signatures: Default::default(),
    };

    let plan = PackingPlan::default_for(addr, base_dir);
    fs::create_dir_all(&plan.out_dir)?;
    let manifest_path = plan.out_dir.join("Manifest.toml");
    fs::write(&manifest_path, manifest.to_toml_string())?;

    // Copy payload to bundle dir
    let payload_dst = plan.out_dir.join(&plan.payload_name);
    fs::copy(src, payload_dst)?;

    Ok(manifest_path)
}

fn infer_kind_from_tld(tld: TldType, mime: Option<&str>) -> ContentKind {
    use ContentKind::*;
    match tld {
        TldType::Image | TldType::Video | TldType::Audio | TldType::Map | TldType::Route => Blob,
        TldType::Post | TldType::Comment | TldType::News | TldType::Journalist | TldType::Blog | TldType::Passport => {
            match mime {
                Some(m) if m.starts_with("text/") || m == "application/json" => Text,
                _ => Text, // default to Text; upstream can coerce
            }
        }
    }
}
