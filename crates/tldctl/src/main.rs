use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use naming::{
    address::Address,
    hash::{DefaultHasher, HashAlgo, Hasher},
    manifest::pack_bundle,
    tld::TldType,
    Address as Addr,
};
use std::{path::{Path, PathBuf}, str::FromStr};

#[derive(Debug, Parser)]
#[command(name="tldctl", version, about="Hash -> <sha>.<tld> -> Manifest.toml packager")]
struct Cli {
    /// Where to write bundles (each address becomes a directory here)
    #[arg(long, default_value = ".onions")]
    out: PathBuf,

    /// Optional index DB path (Sled). If set (or if --index), we write to it.
    #[arg(long, default_value = ".data/index")]
    index_db: PathBuf,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Hash a file and print <sha>.<tld>
    Hash {
        /// File to hash
        #[arg(long)]
        file: PathBuf,
        /// TLD kind: image, post, comment, map, route, news, journalist, blog, passport
        #[arg(long)]
        tld: String,
        /// Algorithm
        #[arg(long, default_value = "sha256")]
        algo: AlgoOpt,
    },

    /// Hash + build a bundle (Manifest.toml + payload copy)
    Pack {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        tld: String,
        #[arg(long, default_value = "sha256")]
        algo: AlgoOpt,
        /// Optional owner payout address (wallet)
        #[arg(long)]
        owner_addr: Option<String>,
        /// Optional origin/author public key (signing key)
        #[arg(long)]
        origin_pubkey: Option<String>,
        /// Also register this address → bundle path into the Sled index
        #[arg(long)]
        index: bool,
    },

    /// Validate a string address and echo normalized form
    Parse {
        /// e.g. "deadbeef... .image"
        addr: String,
    },

    /// Lookup an address in the index DB and print the bundle directory
    Resolve {
        /// Address like "<sha>.<tld>"
        addr: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum AlgoOpt { Sha256, Blake3 }

impl From<AlgoOpt> for HashAlgo {
    fn from(a: AlgoOpt) -> Self {
        match a { AlgoOpt::Sha256 => HashAlgo::Sha256, AlgoOpt::Blake3 => HashAlgo::Blake3 }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Hash { file, tld, algo } => {
            let tld: TldType = tld.parse().context("unknown tld")?;
            let ch = DefaultHasher::hash_file(algo.into(), &file)?;
            let addr = Address::new(ch.to_hex(), tld);
            println!("{}", addr);
        }
        Cmd::Pack { file, tld, algo, owner_addr, origin_pubkey, index } => {
            let tld: TldType = tld.parse().context("unknown tld")?;
            let ch = DefaultHasher::hash_file(algo.into(), &file)?;
            let addr = Address::new(ch.to_hex(), tld);
            let manifest_path = pack_bundle(
                &addr, tld, ch, &file, owner_addr, origin_pubkey, &cli.out
            )?;
            println!("OK: {}", manifest_path.display());

            if index {
                let bundle_dir = manifest_path.parent().expect("manifest has parent");
                let db = index::Index::open(&cli.index_db)?;
                db.put_address(&addr, bundle_dir)?;
                println!("Indexed {} -> {}", addr, bundle_dir.display());
            }
        }
        Cmd::Parse { addr } => {
            let a = Addr::from_str(&addr).context("bad address")?;
            println!("{}", a);
        }
        Cmd::Resolve { addr } => {
            let a = Addr::from_str(&addr).context("bad address")?;
            let db = index::Index::open(&cli.index_db)?;
            match db.get_address(&a)? {
                Some(ent) => println!("{}", ent.bundle_dir.display()),
                None => {
                    eprintln!("not found");
                    std::process::exit(2);
                }
            }
        }
    }

    Ok(())
}
