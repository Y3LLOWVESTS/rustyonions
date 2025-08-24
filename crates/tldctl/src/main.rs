// crates/tldctl/src/main.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use blake3;
use clap::{Parser, Subcommand};
use index::Index;
use infer;
use naming::manifest::{
    write_manifest, Encoding, ManifestV2, Payment, Relations, RevenueSplit,
};
use naming::Address;
use ryker::validate_payment_block;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use toml::{map::Map as TomlMap, Value as TomlValue}; // ← use toml::map::Map here
use zstd::Encoder as ZstdEncoder;

/// Pack source files into the RustyOnions object store and index them by BLAKE3 address.
#[derive(Parser, Debug)]
#[command(name = "tldctl", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Pack a single file into the store and index it.
    Pack {
        /// TLD kind (e.g., text, image, video)
        #[arg(long)]
        tld: String,

        /// Path to the input file
        #[arg(long)]
        input: PathBuf,

        /// Path to the index database directory (sled)
        #[arg(long)]
        index_db: PathBuf,

        /// Root of the content-addressed store
        #[arg(long)]
        store_root: PathBuf,

        // ---------- Payments (optional) ----------
        /// (Micropayments) Mark payment as required to access this object
        #[arg(long, default_value_t = false)]
        required: bool,

        /// Price model: per_mib | flat | per_request
        #[arg(long, value_name = "MODEL", default_value = "")]
        price_model: String,

        /// Price value (units depend on model)
        #[arg(long, value_name = "NUM", default_value_t = 0.0)]
        price: f64,

        /// Currency code (e.g., USD, sats, ETH, SOL)
        #[arg(long, value_name = "CODE", default_value = "")]
        currency: String,

        /// Wallet address / LNURL / pay endpoint
        #[arg(long, value_name = "WALLET", default_value = "")]
        wallet: String,

        /// Settlement type: onchain | offchain | custodial (advisory)
        #[arg(long, value_name = "KIND", default_value = "")]
        settlement: String,

        // ---------- Relations / License / Extensions (PR-8) ----------
        /// Parent object address (b3:<hex>.<tld>) → [relations].parent
        #[arg(long)]
        parent: Option<String>,

        /// Thread/root object address (b3:<hex>.<tld>) → [relations].thread
        #[arg(long)]
        thread: Option<String>,

        /// SPDX or human-readable license → top-level `license`
        #[arg(long)]
        license: Option<String>,

        /// Repeatable: ns:key=value → [ext.<ns>].<key>="value"
        /// Example: --ext image:width=800 --ext image:height=600 --ext seo:title="Hello"
        #[arg(long = "ext")]
        ext: Vec<String>,
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
            required,
            price_model,
            price,
            currency,
            wallet,
            settlement,
            parent,
            thread,
            license,
            ext,
        } => {
            let addr = pack(
                &tld,
                &input,
                &index_db,
                &store_root,
                required,
                &price_model,
                price,
                &currency,
                &wallet,
                &settlement,
                parent.as_deref(),
                thread.as_deref(),
                license.as_deref(),
                &ext,
            )?;
            // Print only the canonical address (scripts capture this)
            println!("{addr}");
            Ok(())
        }
    }
}

fn pack(
    tld: &str,
    input: &Path,
    index_db: &Path,
    store_root: &Path,
    required: bool,
    price_model: &str,
    price: f64,
    currency: &str,
    wallet: &str,
    settlement: &str,
    parent: Option<&str>,
    thread: Option<&str>,
    license: Option<&str>,
    ext_kvs: &[String],
) -> Result<String> {
    // ---------- Read original bytes ----------
    let data = fs::read(input).with_context(|| format!("read input {}", input.display()))?;
    let orig_len = data.len();

    // ---------- Policy enforcement ----------
    match tld {
        "image" => ensure_is_avif(&data, input)?,
        "video" => ensure_is_av1(&data, input)?,
        _ => {}
    }

    // ---------- Canonical address ----------
    let hash = blake3::hash(&data);
    let hash_hex = hash.to_hex().to_string();
    let addr_str = format!("b3:{hash_hex}.{tld}");

    // ---------- Store path ----------
    let shard2 = &hash_hex[0..2];
    let bundle_dir = store_root
        .join("objects")
        .join(tld)
        .join(shard2)
        .join(format!("{hash_hex}.{tld}"));
    fs::create_dir_all(&bundle_dir)
        .with_context(|| format!("create bundle dir {}", bundle_dir.display()))?;

    // ---------- Write payload.bin ----------
    let stored_filename = "payload.bin";
    let payload_path = bundle_dir.join(stored_filename);
    fs::write(&payload_path, &data).context("write payload.bin")?;

    // ---------- Guess MIME ----------
    let mime = guess_mime(&data, input);

    // ---------- Build Manifest v2 ----------
    let created_utc =
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
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
        license: None,
        ext: BTreeMap::new(),
    };

    // ---------- Precompress texty assets ----------
    if is_compressible_mime(&mime) {
        // Zstd (lvl 15)
        let zst_path = bundle_dir.join("payload.bin.zst");
        {
            let mut out = fs::File::create(&zst_path).context("create .zst")?;
            let mut enc = ZstdEncoder::new(&mut out, 15).context("zstd encoder")?;
            enc.write_all(&data).context("zstd write")?;
            enc.finish().context("zstd finish")?;
        }
        let zst_bytes = fs::read(&zst_path).context("read .zst back")?;
        let zst_hash = blake3::hash(&zst_bytes).to_hex().to_string();
        manifest.encodings.push(Encoding {
            coding: "zstd".into(),
            level: 15,
            bytes: zst_bytes.len() as u64,
            filename: "payload.bin.zst".into(),
            hash_hex: zst_hash,
        });

        // Brotli (q 9, window lgwin 22)
        let br_path = bundle_dir.join("payload.bin.br");
        {
            let mut out = fs::File::create(&br_path).context("create .br")?;
            // brotli::CompressorWriter(buf_size, quality, lgwin)
            let mut w = brotli::CompressorWriter::new(&mut out, 4096, 9, 22);
            w.write_all(&data).context("brotli write")?;
            w.flush().ok();
            // Drop `w` to flush final bytes
        }
        let br_bytes = fs::read(&br_path).context("read .br back")?;
        let br_hash = blake3::hash(&br_bytes).to_hex().to_string();
        manifest.encodings.push(Encoding {
            coding: "br".into(),
            level: 9,
            bytes: br_bytes.len() as u64,
            filename: "payload.bin.br".into(),
            hash_hex: br_hash,
        });
    }

    // ---------- Optional [payment] ----------
    let any_payment_flag = !wallet.is_empty() || required;
    if any_payment_flag {
        let p = Payment {
            required,
            currency: currency.to_string(),
            price_model: price_model.to_string(),
            price,
            wallet: wallet.to_string(),
            settlement: settlement.to_string(),
            splits: Vec::<RevenueSplit>::new(),
        };
        // Validate for internal consistency (fail early if user passed invalid combo)
        validate_payment_block(&p).context("invalid [payment] flags")?;
        manifest.payment = Some(p);
    }

    // ---------- PR-8: relations / license / ext.* ----------
    // relations
    if parent.is_some() || thread.is_some() {
        let mut r = manifest.relations.take().unwrap_or(Relations {
            parent: None,
            thread: None,
            source: None,
        });
        if let Some(p) = parent {
            r.parent = Some(p.to_string());
        }
        if let Some(t) = thread {
            r.thread = Some(t.to_string());
        }
        manifest.relations = Some(r);
    }

    // license (top-level now)
    if let Some(lic) = license {
        if !lic.trim().is_empty() {
            manifest.license = Some(lic.trim().to_string());
        }
    }

    // ext parsing: --ext ns:key=value
    if !ext_kvs.is_empty() {
        // manifest.ext is a BTreeMap<String, toml::Value>; each ns becomes a toml table.
        let mut ext = manifest.ext; // take existing (empty by default)
        for raw in ext_kvs {
            if let Some((ns, rest)) = raw.split_once(':') {
                if let Some((k, v)) = rest.split_once('=') {
                    let ns = ns.trim().to_string();
                    let k = k.trim().to_string();
                    // Keep value as TOML string for simplicity; could parse bool/num later if desired.
                    let v_str = v.trim().trim_matches('"').to_string();

                    // Get or create the [ext.<ns>] table using toml::map::Map
                    let table = ext
                        .entry(ns.clone())
                        .or_insert_with(|| TomlValue::Table(TomlMap::new()));

                    // Ensure it's a table
                    let tbl = table.as_table_mut().ok_or_else(|| {
                        anyhow!("ext namespace '{}' was not a table in manifest", ns)
                    })?;

                    tbl.insert(k, TomlValue::String(v_str));
                    continue;
                }
            }
            eprintln!("[tldctl] --ext expects ns:key=value, got: {raw}");
        }
        manifest.ext = ext;
    }

    // ---------- Write Manifest.toml ----------
    let _ = write_manifest(&bundle_dir, &manifest).context("write Manifest.toml")?;

    // ---------- Update index ----------
    let idx = Index::open(index_db).context("open index")?;
    let addr = Address::parse(&addr_str).context("parse address")?;
    idx.put_address(&addr, bundle_dir.clone())
        .context("index put_address")?;

    Ok(addr_str)
}

// ---------- Helpers ----------

fn ensure_is_avif(data: &[u8], path: &Path) -> Result<()> {
    // quick signature check (ftypavif in ISOBMFF brands)
    let is_avif_brand = data.windows(8).any(|w| w == b"ftypavif");
    // infer MIME
    let ok_infer = infer::get(data)
        .map(|k| k.mime_type() == "image/avif")
        .unwrap_or(false);
    if !(is_avif_brand || ok_infer) {
        return Err(anyhow!(
            "policy: .image requires AVIF → {}",
            path.display()
        )
        .context("not AVIF: missing ftyp/avif brand"));
    }
    Ok(())
}

fn ensure_is_av1(data: &[u8], path: &Path) -> Result<()> {
    // look for 'av01' (mp4) or 'V_AV1' (webm)
    let has_av1 = data.windows(4).any(|w| w == b"av01")
        || data.windows(5).any(|w| w == b"V_AV1");
    if !has_av1 {
        return Err(anyhow!(
            "policy: .video requires AV1 → {}",
            path.display()
        ));
    }
    Ok(())
}

fn guess_mime(data: &[u8], path: &Path) -> String {
    if let Some(k) = infer::get(data) {
        return k.mime_type().to_string();
    }
    // crude fallback: utf-8 → text/plain
    if std::str::from_utf8(data).is_ok() {
        return "text/plain; charset=utf-8".to_string();
    }
    // extension hints
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext {
            "json" => return "application/json".into(),
            "js" => return "application/javascript".into(),
            "svg" => return "image/svg+xml".into(),
            "md" | "txt" => return "text/plain; charset=utf-8".into(),
            _ => {}
        }
    }
    "application/octet-stream".to_string()
}

fn is_compressible_mime(mime: &str) -> bool {
    mime.starts_with("text/")
        || matches!(
            mime,
            "application/json" | "application/javascript" | "image/svg+xml"
        )
}
