// crates/ryker/src/lib.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Result};
use naming::manifest::Payment;

/// Price model understood by Ryker. Mirrors `Payment.price_model`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceModel {
    PerMiB,
    Flat,
    PerRequest,
}

impl PriceModel {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "per_mib" => Some(Self::PerMiB),
            "flat" => Some(Self::Flat),
            "per_request" => Some(Self::PerRequest),
            _ => None,
        }
    }
}

/// Compute the cost (in the `Payment.currency`) for serving `n_bytes` under a Payment policy.
/// Returns `None` if no payment policy is present or is marked not required.
pub fn compute_cost(n_bytes: u64, p: &Payment) -> Option<f64> {
    // If the policy isn’t required, we treat it as informational (no charge).
    if !p.required {
        return None;
    }
    let model = PriceModel::parse(&p.price_model)?;
    let price = p.price;

    match model {
        PriceModel::PerMiB => {
            let mibs = (n_bytes as f64) / (1024.0 * 1024.0);
            Some(price * mibs)
        }
        PriceModel::Flat | PriceModel::PerRequest => Some(price),
    }
}

/// Very lightweight wallet check. This is intentionally permissive;
/// you can tighten per scheme later (LNURL, BTC on-chain, SOL, etc.).
pub fn validate_wallet_string(wallet: &str) -> Result<()> {
    if wallet.trim().is_empty() {
        return Err(anyhow!("wallet is empty"));
    }
    // Example heuristics you can extend:
    // - LNURL often starts with 'lnurl' (bech32) or 'LNURL'.
    // - BTC on-chain: base58/bech32; SOL: base58, fixed length; ETH: 0x + 40 hex.
    // We do not enforce scheme here yet—just presence.
    Ok(())
}

/// Convenience: check that a `Payment` block is internally consistent enough to consider enforceable.
pub fn validate_payment_block(p: &Payment) -> Result<()> {
    // Required fields when we intend to enforce:
    // currency, price_model (parseable), wallet (non-empty).
    PriceModel::parse(&p.price_model).ok_or_else(|| anyhow!("unknown price_model"))?;
    validate_wallet_string(&p.wallet)?;
    if p.price < 0.0 {
        return Err(anyhow!("price must be non-negative"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use naming::manifest::{Payment, RevenueSplit};

    fn base(required: bool, model: &str, price: f64) -> Payment {
        Payment {
            required,
            currency: "USD".to_string(),
            price_model: model.to_string(),
            price,
            wallet: "lnurl1deadbeef".to_string(),
            settlement: "offchain".to_string(),
            splits: vec![RevenueSplit {
                account: "creator".into(),
                pct: 100.0,
            }],
        }
    }

    #[test]
    fn cost_per_mib() -> Result<()> {
        let p = base(true, "per_mib", 0.01); // 1 cent / MiB
        let c = compute_cost(2 * 1024 * 1024, &p).ok_or_else(|| anyhow!("expected Some(cost)"))?;
        assert!((c - 0.02).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn cost_flat() -> Result<()> {
        let p = base(true, "flat", 0.5);
        let c1 = compute_cost(10, &p).ok_or_else(|| anyhow!("expected Some(cost)"))?;
        let c2 = compute_cost(10_000_000, &p).ok_or_else(|| anyhow!("expected Some(cost)"))?;
        // Avoid float direct equality per Clippy; use a small epsilon.
        assert!((c1 - 0.5).abs() < 1e-12);
        assert!((c2 - 0.5).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn not_required_yields_none() {
        let p = base(false, "per_mib", 0.01);
        assert!(compute_cost(1024, &p).is_none());
    }

    #[test]
    fn validate_payment_ok() -> Result<()> {
        let p = base(true, "per_request", 0.001);
        validate_payment_block(&p)?;
        Ok(())
    }
}
