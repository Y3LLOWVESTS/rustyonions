// crates/tldctl/src/main.rs
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use blake3::Hasher;
use brotli::enc::BrotliEncoderParams;
use clap::{Parser, Subcommand};
use index::Index;
use naming::Address;
use naming::manifest::{write_manifest, Encoding, ManifestV2};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use zstd::stream::Encoder as ZstdEncoder;
use std::collections::BTreeMap;

/// CLI for packing files into RustyOnions object bundles and indexing them.
#[derive(Parser, Debug)]
#[command(name = "tldctl", version, about = "RustyOnions TLD pack/inspect tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Pack a file as an object under a given TLD and index it.
    Pack {
        /// TLD/kind for this object (e.g. image, video, text, json, etc.)
        #[arg(long)]
        tld: String,

        /// Input file path (original bytes are hashed canonically).
        #[arg(long)]
        input: PathBuf,

        /// Path to the index database (Sled dir). Example: .data/index
        #[arg(long, default_value = ".data/index")]
        index_db: PathBuf,

        /// Store root directory. Bundles land under store/objects/<tld>/<shard2>/<hex>.<tld>/
        #[arg(long, default_value = "store")]
        store_root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Pack {
            tld,
            input,
            index_db,
            store_root,
        } => cmd_pack(&tld, &input, &index_db, &store_root),
    }
}

fn cmd_pack(tld: &str, input: &Path, index_db: &Path, store_root: &Path) -> Result<()> {
    // Enforce media policy for .image / .video
    match tld {
        "image" => ensure_is_avif(input)
            .with_context(|| format!("policy: .image requires AVIF → {}", input.display()))?,
        "video" => ensure_is_av1(input).with_context(|| {
            format!(
                "policy: .video requires AV1 in MP4/WebM (av01 or V_AV1) → {}",
                input.display()
            )
        })?,
        _ => {}
    }

    // Read original bytes and compute canonical BLAKE3
    let (orig_bytes, orig_len) = read_all(input)
        .with_context(|| format!("read input {}", input.display()))?;
    let hash_hex = blake3_hex(&orig_bytes);
    let addr_str = format!("b3:{}.{}", hash_hex, tld);

    // Validate computed address parses with naming::Address
    let address = Address::parse(&addr_str)
        .with_context(|| format!("computed invalid address {}", addr_str))?;

    // Bundle directory
    let shard2 = &hash_hex[..2];
    let bundle_dir = store_root
        .join("objects")
        .join(tld)
        .join(shard2)
        .join(format!("{}.{}", hash_hex, tld));
    fs::create_dir_all(&bundle_dir)
        .with_context(|| format!("create {}", bundle_dir.display()))?;

    // Write payload.bin
    let stored_filename = "payload.bin";
    let payload_path = bundle_dir.join(stored_filename);
    fs::write(&payload_path, &orig_bytes)
        .with_context(|| format!("write {}", payload_path.display()))?;

    // Build manifest v2 (encodings filled below if we precompress)
    let created_utc = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let mime = sniff_mime(tld, input, &orig_bytes);

    let mut manifest = ManifestV2 {
        schema_version: 2,
        tld: tld.to_string(),
        address: addr_str.clone(),
        hash_algo: "b3".to_string(),
        hash_hex: hash_hex.clone(),
        bytes: orig_len as u64,
        created_utc,
        mime: mime.clone(),
        stored_filename: stored_filename.to_string(),
        original_filename: input
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        encodings: Vec::<Encoding>::new(),
                payment: None,
        relations: None,
        ext: BTreeMap::new(),

    };

    // Precompress "texty" assets and populate encodings
    if is_texty_mime(&mime) {
        // Zstd (level ~15)
        let zst_path = payload_path.with_extension("bin.zst");
        let zst_level = 15;
        compress_zstd(&orig_bytes, &zst_path, zst_level)?;
        let zst_bytes = fs::metadata(&zst_path)?.len();
        let zst_hash = blake3_hex(&fs::read(&zst_path)?);
        manifest.encodings.push(Encoding {
            coding: "zstd".into(),
            level: zst_level as i32,
            bytes: zst_bytes as u64,
            filename: zst_path.file_name().unwrap().to_string_lossy().to_string(),
            hash_hex: zst_hash,
        });

        // Brotli (q ~9)
        let br_path = payload_path.with_extension("bin.br");
        let br_quality = 9i32;
        compress_brotli(&orig_bytes, &br_path, br_quality)?;
        let br_bytes = fs::metadata(&br_path)?.len();
        let br_hash = blake3_hex(&fs::read(&br_path)?);
        manifest.encodings.push(Encoding {
            coding: "br".into(),
            level: br_quality,
            bytes: br_bytes as u64,
            filename: br_path.file_name().unwrap().to_string_lossy().to_string(),
            hash_hex: br_hash,
        });
    }

    // Write Manifest.toml
    let _manifest_path = write_manifest(&bundle_dir, &manifest)?;

    // Index write via index::Index
    let idx = Index::open(index_db)
        .with_context(|| format!("open index {}", index_db.display()))?;
    idx.put_address(&address, bundle_dir.clone())
        .context("write address entry to index")?;

    // Print the canonical address
    println!("{}", addr_str);
    Ok(())
}

fn read_all(path: &Path) -> Result<(Vec<u8>, usize)> {
    let mut f = fs::File::open(path)?;
    let mut buf = Vec::with_capacity(
        f.metadata()
            .map(|m| m.len() as usize)
            .unwrap_or(0usize.saturating_add(1)),
    );
    f.read_to_end(&mut buf)?;
    Ok((buf.clone(), buf.len()))
}

fn blake3_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    hash.to_hex().to_string()
}

/// --- compression helpers ---

fn compress_zstd(input: &[u8], out_path: &Path, level: i32) -> Result<()> {
    let mut f = fs::File::create(out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    let mut enc = ZstdEncoder::new(&mut f, level)
        .with_context(|| "zstd encoder init")?;
    enc.write_all(input)?;
    enc.finish().context("zstd finish")?;
    Ok(())
}

fn compress_brotli(input: &[u8], out_path: &Path, quality: i32) -> Result<()> {
    let mut out = Vec::new();
    let mut params = BrotliEncoderParams::default();
    params.quality = quality;
    brotli::BrotliCompress(&mut &*input, &mut out, &params).context("brotli compress")?;
    fs::write(out_path, out).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

/// --- lightweight MIME + policy helpers ---

fn is_texty_mime(m: &str) -> bool {
    m.starts_with("text/")
        || matches!(
            m,
            "application/json"
                | "application/javascript"
                | "application/xml"
                | "image/svg+xml"
        )
}

fn sniff_mime(tld: &str, path: &Path, data: &[u8]) -> String {
    match tld {
        "text" => return "text/plain; charset=utf-8".into(),
        "json" => return "application/json".into(),
        "image" => {
            if looks_avif(data) {
                return "image/avif".into();
            }
        }
        "video" => {
            if looks_mp4(data) || memmem(data, b"webm") || memmem(data, b"matroska") {
                return "video/mp4".into();
            }
        }
        _ => {}
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        return match ext.as_str() {
            "txt" => "text/plain; charset=utf-8".into(),
            "md" => "text/markdown; charset=utf-8".into(),
            "json" => "application/json".into(),
            "js" => "application/javascript".into(),
            "svg" => "image/svg+xml".into(),
            "avif" => "image/avif".into(),
            "mp4" => "video/mp4".into(),
            "webm" => "video/webm".into(),
            _ => "application/octet-stream".into(),
        };
    }
    if looks_avif(data) {
        "image/avif".into()
    } else if looks_mp4(data) {
        "video/mp4".into()
    } else if memmem(data, b"<svg") {
        "image/svg+xml".into()
    } else {
        "application/octet-stream".into()
    }
}

fn ensure_is_avif(path: &Path) -> Result<()> {
    let data = fs::read(path)?;
    if data.len() < 24 {
        bail!("file too small to be valid AVIF");
    }
    let mut i = 0usize;
    let mut found_ftyp = false;
    let mut has_avif_brand = false;
    while i + 8 <= data.len() {
        let size = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        if size < 8 || i + size > data.len() {
            break;
        }
        let typ = &data[i + 4..i + 8];
        if typ == b"ftyp" {
            found_ftyp = true;
            if size >= 16 {
                let brand_bytes = &data[i + 8..i + size];
                if brand_bytes.windows(4).any(|w| w == b"avif") {
                    has_avif_brand = true;
                }
            }
            break;
        }
        i += size;
    }
    if !found_ftyp || !has_avif_brand {
        bail!("not AVIF: missing ftyp/avif brand");
    }
    Ok(())
}

fn ensure_is_av1(path: &Path) -> Result<()> {
    let data = fs::read(path)?;
    if data.len() < 12 {
        bail!("file too small to be valid video");
    }
    let looks_mp4 = looks_mp4(&data);
    let has_av01 = memmem(&data, b"av01");
    let looks_webm = memmem(&data, b"webm") || memmem(&data, b"matroska");
    let has_v_av1 = memmem(&data, b"V_AV1");
    if (looks_mp4 && has_av01) || (looks_webm && has_v_av1) || has_v_av1 {
        return Ok(());
    }
    bail!("not AV1 video: expected MP4('av01') or WebM('V_AV1')");
}

fn looks_avif(data: &[u8]) -> bool {
    if data.len() < 32 {
        return false;
    }
    let mut i = 0usize;
    while i + 8 <= data.len() && i < 256 {
        let size = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        if size < 8 || i + size > data.len() {
            break;
        }
        if &data[i + 4..i + 8] == b"ftyp" {
            let brand_bytes = &data[i + 8..data.len().min(i + size)];
            return brand_bytes.windows(4).any(|w| w == b"avif");
        }
        i += size;
    }
    false
}

fn looks_mp4(data: &[u8]) -> bool {
    memmem(data, b"ftyp")
}

fn memmem(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}
