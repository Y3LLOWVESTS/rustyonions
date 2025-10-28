//! RO:WHAT — Minimal CLI for naming hygiene: parse/normalize/encode.
//! RO:WHY  — Folded from tldctl into ron-naming (canon); DX helper only.
//! RO:INTERACTS — address, normalize, wire::{json,cbor}
//! RO:INVARIANTS — No network; stdout-only. Errors are structured.

#![cfg(feature = "cli")]

use base64::Engine; // bring trait in-scope for .encode()
use clap::{Parser, Subcommand};
use ron_naming::{
    address::ParseAddressError, normalize::normalize_fqdn_ascii, wire, Address, NameRecord,
};

/// tldctl — RON naming toolbox (normalize, parse, encode)
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Normalize a domain name to ASCII (UTS-46/IDNA)
    Normalize { name: String },

    /// Parse a user address string (b3:... or name[@ver]) and print JSON DTO
    Parse { addr: String },

    /// Encode a NameRecord as JSON
    Json {
        name: String,
        /// Optional semantic version like 1.2.3
        #[arg(long)]
        version: Option<String>,
        /// Content id in the form b3:<64hex>
        #[arg(long)]
        content: String,
    },

    /// Encode a NameRecord as CBOR (base64 to stdout)
    Cbor {
        name: String,
        /// Optional semantic version like 1.2.3
        #[arg(long)]
        version: Option<String>,
        /// Content id in the form b3:<64hex>
        #[arg(long)]
        content: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Normalize { name } => {
            let nfqdn = normalize_fqdn_ascii(&name)?;
            println!("{}", (nfqdn.0).0);
        }
        Cmd::Parse { addr } => {
            let a = Address::parse(&addr).map_err(map_addr_err)?;
            let bytes = wire::json::to_json_bytes(&a)?;
            println!("{}", String::from_utf8(bytes).unwrap());
        }
        Cmd::Json {
            name,
            version,
            content,
        } => {
            let a = Address::parse(&format!(
                "{}{}",
                name,
                version
                    .as_deref()
                    .map(|v| format!("@{v}"))
                    .unwrap_or_default()
            ))?;
            let nr = match a {
                Address::Name { fqdn, version } => NameRecord {
                    name: fqdn,
                    version,
                    content: ron_naming::types::ContentId(content),
                },
                Address::Content { .. } => anyhow::bail!(
                    "content id form is not allowed for NameRecord 'name' (expect a domain)"
                ),
            };
            let bytes = wire::json::to_json_bytes(&nr)?;
            println!("{}", String::from_utf8(bytes).unwrap());
        }
        Cmd::Cbor {
            name,
            version,
            content,
        } => {
            let a = Address::parse(&format!(
                "{}{}",
                name,
                version
                    .as_deref()
                    .map(|v| format!("@{v}"))
                    .unwrap_or_default()
            ))?;
            let nr = match a {
                Address::Name { fqdn, version } => NameRecord {
                    name: fqdn,
                    version,
                    content: ron_naming::types::ContentId(content),
                },
                Address::Content { .. } => anyhow::bail!(
                    "content id form is not allowed for NameRecord 'name' (expect a domain)"
                ),
            };
            let bytes = wire::cbor::to_cbor_bytes(&nr)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            println!("{b64}");
        }
    }
    Ok(())
}

fn map_addr_err(e: ParseAddressError) -> anyhow::Error {
    match e {
        ParseAddressError::InvalidContentId => {
            anyhow::anyhow!("invalid content id (expect b3:<64 hex>)")
        }
        ParseAddressError::InvalidName => {
            anyhow::anyhow!("invalid name (IDNA/ASCII hygiene failed)")
        }
        ParseAddressError::InvalidVersion => {
            anyhow::anyhow!("invalid version (semver)")
        }
    }
}
