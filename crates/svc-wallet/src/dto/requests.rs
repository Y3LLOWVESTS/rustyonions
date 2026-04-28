//! RO:WHAT — Strict request DTOs for wallet v1 balance, issue, transfer, burn, and future hold flows.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/DX. Reject drift at the API boundary before policy or ledger IO.
//! RO:INTERACTS — config, errors, util::parsing, ledger::client, routes/v1.
//! RO:INVARIANTS — deny_unknown_fields; amount strings parse to u128; nonce starts at 1; asset must match config.
//! RO:METRICS — route layer maps validation failures to wallet_rejects_total{reason="BAD_REQUEST"}.
//! RO:CONFIG — WalletConfig asset and amount ceilings.
//! RO:SECURITY — no bearer tokens in DTOs; Authorization stays in headers.
//! RO:TEST — rejects_unknown_fields; amount_serializes_as_string; validation_rejects_zero_amount.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

use crate::{
    config::{WalletConfig, NONCE_START},
    errors::{WalletError, WalletResult},
    util::parsing::{validate_account_id, validate_asset, validate_idempotency_key, validate_memo},
};

/// Amount in minor units, serialized as a decimal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmountMinor(pub u128);

impl AmountMinor {
    /// Build an amount and require it to be non-zero.
    pub fn new(value: u128) -> WalletResult<Self> {
        if value == 0 {
            return Err(WalletError::bad_request("amount_minor must be > 0"));
        }
        Ok(Self(value))
    }

    /// Return raw u128 value.
    pub const fn get(self) -> u128 {
        self.0
    }

    /// Convert to u64 for the current ron-ledger primitive amount type.
    pub fn try_as_u64_for_ledger(self) -> WalletResult<u64> {
        u64::try_from(self.0).map_err(|_| {
            WalletError::limits_exceeded(
                "amount_minor exceeds current ron-ledger u64 adapter ceiling",
            )
        })
    }
}

impl Serialize for AmountMinor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for AmountMinor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AmountVisitor;

        impl<'de> serde::de::Visitor<'de> for AmountVisitor {
            type Value = AmountMinor;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a decimal string or integer amount in minor units")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let parsed = u128::from_str(value).map_err(E::custom)?;
                AmountMinor::new(parsed).map_err(E::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                AmountMinor::new(value as u128).map_err(E::custom)
            }

            fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                AmountMinor::new(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(AmountVisitor)
    }
}

/// Balance query shape used by route extractors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalanceQuery {
    /// Account identifier.
    pub account: String,
    /// Asset identifier.
    pub asset: String,
}

impl BalanceQuery {
    /// Validate against config and account grammar.
    pub fn validate(&self, cfg: &WalletConfig) -> WalletResult<()> {
        validate_account_id(&self.account)?;
        validate_asset(&self.asset, cfg)?;
        Ok(())
    }
}

/// POST /v1/issue request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueRequest {
    /// Destination account.
    pub to: String,
    /// Asset identifier.
    pub asset: String,
    /// Amount in minor units as string.
    pub amount_minor: AmountMinor,
    /// Optional idempotency key when not supplied by header.
    pub idempotency_key: Option<String>,
    /// Optional redacted memo.
    pub memo: Option<String>,
}

impl IssueRequest {
    /// Validate static DTO invariants.
    pub fn validate(&self, cfg: &WalletConfig) -> WalletResult<()> {
        validate_account_id(&self.to)?;
        validate_asset(&self.asset, cfg)?;
        validate_amount(self.amount_minor, cfg)?;
        if let Some(key) = self.idempotency_key.as_deref() {
            validate_idempotency_key(key)?;
        }
        validate_memo(self.memo.as_deref())?;
        Ok(())
    }
}

/// POST /v1/transfer request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferRequest {
    /// Source account.
    pub from: String,
    /// Destination account.
    pub to: String,
    /// Asset identifier.
    pub asset: String,
    /// Amount in minor units as string.
    pub amount_minor: AmountMinor,
    /// Per-source-account strict next nonce.
    pub nonce: u64,
    /// Optional idempotency key when not supplied by header.
    pub idempotency_key: Option<String>,
    /// Optional redacted memo.
    pub memo: Option<String>,
}

impl TransferRequest {
    /// Validate static DTO invariants.
    pub fn validate(&self, cfg: &WalletConfig) -> WalletResult<()> {
        validate_account_id(&self.from)?;
        validate_account_id(&self.to)?;
        if self.from == self.to {
            return Err(WalletError::bad_request("from and to must differ"));
        }
        validate_asset(&self.asset, cfg)?;
        validate_amount(self.amount_minor, cfg)?;
        validate_nonce(self.nonce)?;
        if let Some(key) = self.idempotency_key.as_deref() {
            validate_idempotency_key(key)?;
        }
        validate_memo(self.memo.as_deref())?;
        Ok(())
    }
}

/// POST /v1/burn request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BurnRequest {
    /// Source account.
    pub from: String,
    /// Asset identifier.
    pub asset: String,
    /// Amount in minor units as string.
    pub amount_minor: AmountMinor,
    /// Per-source-account strict next nonce.
    pub nonce: u64,
    /// Optional idempotency key when not supplied by header.
    pub idempotency_key: Option<String>,
    /// Optional redacted memo.
    pub memo: Option<String>,
}

impl BurnRequest {
    /// Validate static DTO invariants.
    pub fn validate(&self, cfg: &WalletConfig) -> WalletResult<()> {
        validate_account_id(&self.from)?;
        validate_asset(&self.asset, cfg)?;
        validate_amount(self.amount_minor, cfg)?;
        validate_nonce(self.nonce)?;
        if let Some(key) = self.idempotency_key.as_deref() {
            validate_idempotency_key(key)?;
        }
        validate_memo(self.memo.as_deref())?;
        Ok(())
    }
}

/// Resolve idempotency key from header or body, preferring the header.
pub fn resolve_idempotency_key(
    header_value: Option<&str>,
    body_value: Option<&str>,
) -> WalletResult<String> {
    let key = header_value.or(body_value).ok_or_else(|| {
        WalletError::bad_request("Idempotency-Key header or body idempotency_key is required")
    })?;
    validate_idempotency_key(key)?;
    Ok(key.to_string())
}

/// Validate an operation amount against config ceilings.
pub fn validate_amount(amount: AmountMinor, cfg: &WalletConfig) -> WalletResult<()> {
    if amount.get() > cfg.max_amount_per_op {
        return Err(WalletError::limits_exceeded(
            "amount_minor exceeds max_amount_per_op",
        ));
    }
    Ok(())
}

/// Validate a strict debit-side nonce.
pub fn validate_nonce(nonce: u64) -> WalletResult<()> {
    if nonce < NONCE_START {
        return Err(WalletError::bad_request("nonce must be >= 1"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_serializes_as_string() {
        let encoded = serde_json::to_string(&AmountMinor::new(42).unwrap()).unwrap();
        assert_eq!(encoded, "\"42\"");
    }

    #[test]
    fn transfer_rejects_unknown_field() {
        let raw = r#"{"from":"a","to":"b","asset":"roc","amount_minor":"1","nonce":1,"extra":1}"#;
        assert!(serde_json::from_str::<TransferRequest>(raw).is_err());
    }

    #[test]
    fn validation_rejects_zero_amount() {
        let raw = r#"{"from":"a","to":"b","asset":"roc","amount_minor":"0","nonce":1}"#;
        assert!(serde_json::from_str::<TransferRequest>(raw).is_err());
    }
}
