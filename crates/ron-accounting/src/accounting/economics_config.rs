//! RO:WHAT — Read-only Internal ROC economics config label for accounting snapshots/reports.
//! RO:WHY — ECON/GOV: accounting may label which tokenomics config informed a report, but config labels are not balance, receipt, payout, wallet, ledger, unlock, bridge, staking, liquidity, or external-settlement truth.
//! RO:INTERACTS — `configs/roc-economics.toml`, reward snapshots, svc-rewarder planning inputs.
//! RO:INVARIANTS — label-only; report-only; no wallet side effect; no ledger side effect; no receipt truth; bridge/staking inert.
//! RO:METRICS — callers may count config-label validation failures.
//! RO:CONFIG — parses caller-provided TOML bytes to extract schema/version and disabled future placeholders.
//! RO:SECURITY — rejects authority flags and unsafe future feature posture.
//! RO:TEST — `tests/internal_roc_beta_phase5_config_label_non_authority.rs`.

use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};

/// Accounting label schema for canonical Internal ROC economics config references.
pub const RON_ACCOUNTING_INTERNAL_ROC_ECONOMICS_CONFIG_LABEL_SCHEMA: &str =
    "ron-accounting.internal-roc-economics-config-label.v1";

const INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA: &str = "internal_roc.economics-config.v1";
const INTERNAL_ROC_ECONOMICS_CONFIG_VERSION: u16 = 1;
const MAX_LABEL_BYTES: usize = 160;

/// Read-only label connecting an accounting artifact/report to an economics config version.
///
/// This object is metadata only. It must never become balance truth, receipt truth,
/// payout execution truth, paid unlock authority, wallet authority, or ledger authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocEconomicsConfigLabel {
    /// Label schema.
    pub label_schema: String,
    /// Config schema copied from canonical economics TOML.
    pub config_schema: String,
    /// Config version copied from canonical economics TOML.
    pub config_version: u16,
    /// Human/source label supplied by the caller, such as `configs/roc-economics.toml`.
    pub config_source: String,
    /// BLAKE3 hash of the exact TOML bytes that were labeled.
    pub config_b3: String,
    /// Must remain true: this is accounting/report metadata only.
    pub report_only: bool,
    /// Must remain true: this labels config version/source only.
    pub label_only: bool,
    /// Must remain false: accounting labels are not balance truth.
    pub balance_truth: bool,
    /// Must remain false: accounting labels are not receipt truth.
    pub receipt_truth: bool,
    /// Must remain false: accounting labels do not affect wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting labels do not affect ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting labels do not execute payouts.
    pub payout_side_effect: bool,
    /// Must remain false: accounting labels do not unlock paid content.
    pub paid_unlock_authority: bool,
    /// True only when the future bridge placeholder is disabled/inert.
    pub bridge_inert: bool,
    /// True only when the future staking placeholder is disabled/inert.
    pub staking_inert: bool,
}

impl InternalRocEconomicsConfigLabel {
    /// Build a read-only label from canonical Internal ROC economics TOML bytes.
    ///
    /// # Errors
    ///
    /// Returns `Error::SchemaViolation` if the TOML is malformed, missing the canonical
    /// schema/version, or enables future bridge/staking placeholders.
    pub fn from_toml_bytes(config_source: impl Into<String>, bytes: &[u8]) -> Result<Self> {
        let config_source = config_source.into();
        validate_label_token("config_source", &config_source)?;

        let raw = std::str::from_utf8(bytes)
            .map_err(|err| Error::schema(format!("economics config must be UTF-8: {err}")))?;
        let parsed = raw
            .parse::<toml::Value>()
            .map_err(|err| Error::schema(format!("economics config TOML parse failed: {err}")))?;
        let table = parsed
            .as_table()
            .ok_or_else(|| Error::schema("economics config top-level TOML must be a table"))?;

        let config_schema = string_field(table, "schema")?.to_owned();
        let config_version = u16_field(table, "version")?;

        if config_schema != INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA {
            return Err(Error::schema(
                "economics config label requires canonical internal ROC schema",
            ));
        }

        if config_version != INTERNAL_ROC_ECONOMICS_CONFIG_VERSION {
            return Err(Error::schema(
                "economics config label requires canonical internal ROC version",
            ));
        }

        validate_disabled_placeholder(table, "future_bridge")?;
        validate_disabled_placeholder(table, "future_staking")?;

        let label = Self {
            label_schema: RON_ACCOUNTING_INTERNAL_ROC_ECONOMICS_CONFIG_LABEL_SCHEMA.to_owned(),
            config_schema,
            config_version,
            config_source,
            config_b3: format!("b3:{}", blake3::hash(bytes).to_hex()),
            report_only: true,
            label_only: true,
            balance_truth: false,
            receipt_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            paid_unlock_authority: false,
            bridge_inert: true,
            staking_inert: true,
        };

        label.validate()?;
        Ok(label)
    }

    /// Validate the label and all no-authority flags.
    ///
    /// # Errors
    ///
    /// Returns `Error::SchemaViolation` if the label claims any economic authority.
    pub fn validate(&self) -> Result<()> {
        if self.label_schema != RON_ACCOUNTING_INTERNAL_ROC_ECONOMICS_CONFIG_LABEL_SCHEMA {
            return Err(Error::schema("invalid economics config label schema"));
        }
        if self.config_schema != INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA {
            return Err(Error::schema("invalid labeled economics config schema"));
        }
        if self.config_version != INTERNAL_ROC_ECONOMICS_CONFIG_VERSION {
            return Err(Error::schema("invalid labeled economics config version"));
        }

        validate_label_token("config_source", &self.config_source)?;
        validate_b3("config_b3", &self.config_b3)?;

        if !self.report_only {
            return Err(Error::schema(
                "economics config label must remain report-only",
            ));
        }
        if !self.label_only {
            return Err(Error::schema(
                "economics config label must remain label-only",
            ));
        }
        if self.balance_truth {
            return Err(Error::schema(
                "economics config label must not claim balance truth",
            ));
        }
        if self.receipt_truth {
            return Err(Error::schema(
                "economics config label must not claim receipt truth",
            ));
        }
        if self.wallet_side_effect {
            return Err(Error::schema(
                "economics config label must not claim wallet side effect",
            ));
        }
        if self.ledger_side_effect {
            return Err(Error::schema(
                "economics config label must not claim ledger side effect",
            ));
        }
        if self.payout_side_effect {
            return Err(Error::schema(
                "economics config label must not claim payout side effect",
            ));
        }
        if self.paid_unlock_authority {
            return Err(Error::schema(
                "economics config label must not claim paid unlock authority",
            ));
        }
        if !self.bridge_inert {
            return Err(Error::schema(
                "economics config label must keep bridge inert",
            ));
        }
        if !self.staking_inert {
            return Err(Error::schema(
                "economics config label must keep staking inert",
            ));
        }

        Ok(())
    }
}

fn string_field<'a>(table: &'a toml::Table, field: &str) -> Result<&'a str> {
    table
        .get(field)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::schema(format!("economics config missing string field {field}")))
}

fn u16_field(table: &toml::Table, field: &str) -> Result<u16> {
    let value = table
        .get(field)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| Error::schema(format!("economics config missing integer field {field}")))?;

    u16::try_from(value).map_err(|err| {
        Error::schema(format!(
            "economics config field {field} must fit u16: {err}"
        ))
    })
}

fn table_field<'a>(table: &'a toml::Table, field: &str) -> Result<&'a toml::Table> {
    table
        .get(field)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::schema(format!("economics config missing table {field}")))
}

fn bool_field(table: &toml::Table, field: &str) -> Result<bool> {
    table
        .get(field)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| Error::schema(format!("economics config missing bool field {field}")))
}

fn validate_disabled_placeholder(table: &toml::Table, field: &str) -> Result<()> {
    let placeholder = table_field(table, field)?;
    if bool_field(placeholder, "enabled")? {
        return Err(Error::schema(format!(
            "economics config {field}.enabled must remain false/inert"
        )));
    }

    let state = string_field(placeholder, "state")?;
    validate_label_token(&format!("{field}.state"), state)
}

fn validate_label_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_LABEL_BYTES {
        return Err(Error::schema(format!(
            "{field} must be 1..={MAX_LABEL_BYTES} bytes"
        )));
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return Err(Error::schema(format!(
            "{field} contains unsupported characters"
        )));
    }

    Ok(())
}

fn validate_b3(field: &str, value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(Error::schema(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    };

    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::schema(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    }

    Ok(())
}
