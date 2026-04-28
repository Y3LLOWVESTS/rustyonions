//! RO:WHAT — Integer-only money algebra for reward calculations.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV. ROC math must be deterministic and overflow-checked.
//! RO:INTERACTS — core::compute, core::invariants, outputs::manifest.
//! RO:INVARIANTS — no floats; no wrapping arithmetic; no negative amounts.
//! RO:METRICS — arithmetic errors become quarantine/reject metrics in callers.
//! RO:CONFIG — reward policy caps feed these helpers.
//! RO:SECURITY — avoids precision attacks caused by float/string drift.
//! RO:TEST — unit invariants and proptests later.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Result, RewarderError};

/// ROC minor units. Serialized as decimal string at JSON boundaries.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AmountMinor(pub u128);

impl AmountMinor {
    /// Zero amount.
    pub const ZERO: Self = Self(0);

    /// Return inner value.
    #[must_use]
    pub fn get(self) -> u128 {
        self.0
    }

    /// Checked addition.
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or_else(|| RewarderError::Quarantined("amount addition overflow".into()))
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or_else(|| RewarderError::Quarantined("amount subtraction underflow".into()))
    }

    /// Checked multiplication by a u128 scalar.
    pub fn checked_mul_u128(self, rhs: u128) -> Result<Self> {
        self.0
            .checked_mul(rhs)
            .map(Self)
            .ok_or_else(|| RewarderError::Quarantined("amount multiplication overflow".into()))
    }
}

impl Serialize for AmountMinor {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for AmountMinor {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let value = raw
            .parse::<u128>()
            .map_err(|_| serde::de::Error::custom("amount must be a decimal u128 string"))?;
        Ok(Self(value))
    }
}

/// Compute floor((a * b) / denominator) with checked arithmetic.
pub fn checked_mul_div_floor(a: u128, b: u128, denominator: u128) -> Result<u128> {
    if denominator == 0 {
        return Err(RewarderError::Quarantined("division by zero".into()));
    }
    let product = a
        .checked_mul(b)
        .ok_or_else(|| RewarderError::Quarantined("mul/div overflow".into()))?;
    Ok(product / denominator)
}
