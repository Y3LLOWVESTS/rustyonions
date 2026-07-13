//! RO:WHAT — Exact-b3 moderation policy states and deterministic serve decisions.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 10 needs one canonical source for deny, block,
//! allow, tombstone, and quarantine posture.
//!
//! RO:INTERACTS — future macronode serve gate, crabnode moderation CLI, and
//! signed global policy lists.
//!
//! RO:INVARIANTS — exact full-hash lookup only; stronger refusal states cannot
//! be bypassed by a local allow.
//!
//! RO:SECURITY — declarative decision only; no storage deletion, provider
//! mutation, rewards, wallet, or ledger authority.
//!
//! RO:TEST — `tests/moderation_policy.rs`.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Canonical `b3:<64 lowercase hex>` object identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct B3Id(String);

impl B3Id {
    /// Return the canonical identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for B3Id {
    type Err = B3IdError;

    /// Parse a strict lowercase BLAKE3 content identifier.
    ///
    /// # Errors
    ///
    /// Returns [`B3IdError`] unless the value is exactly `b3:` followed by
    /// 64 lowercase hexadecimal characters.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(hex) = value.strip_prefix("b3:") else {
            return Err(B3IdError);
        };

        if hex.len() != 64
            || !hex
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(B3IdError);
        }

        Ok(Self(value.to_string()))
    }
}

impl fmt::Display for B3Id {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for B3Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for B3Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// Strict `b3` identifier parsing failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct B3IdError;

impl fmt::Display for B3IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected b3:<64 lowercase hexadecimal characters>")
    }
}

impl Error for B3IdError {}

/// Invalid separation between signed-global and operator-local moderation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionError {
    /// A signed global snapshot attempted to carry operator-local state.
    SignedGlobalContainsOperatorState,
    /// An operator-local policy attempted to carry globally governed state.
    LocalPolicyContainsGlobalState,
}

impl fmt::Display for CompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SignedGlobalContainsOperatorState => formatter.write_str(
                "signed global moderation policy must not contain local block, allow, or quarantine state",
            ),
            Self::LocalPolicyContainsGlobalState => formatter.write_str(
                "local moderation policy must not contain global deny or owner tombstone state when composed with signed global policy",
            ),
        }
    }
}

impl Error for CompositionError {}

/// Resulting serve posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Content may proceed to later serve checks.
    Serve,
    /// Content must not be served.
    Refuse,
    /// Content must not be served and requires explicit review.
    Quarantine,
}

impl Effect {
    /// Whether this effect permits serving.
    #[must_use]
    pub const fn permits_serve(self) -> bool {
        matches!(self, Self::Serve)
    }
}

/// Stable, low-cardinality moderation reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    /// No moderation rule matched.
    NoRule,
    /// Operator explicitly allowed this exact object.
    LocalAllow,
    /// Exact object appears on the global denylist.
    GlobalDeny,
    /// Exact object was tombstoned by its owner.
    OwnerTombstone,
    /// Operator blocked this exact object.
    LocalBlock,
    /// Exact object is isolated pending review.
    Quarantined,
}

impl ReasonCode {
    /// Return the canonical low-cardinality label for this reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoRule => "no_rule",
            Self::LocalAllow => "local_allow",
            Self::GlobalDeny => "global_deny",
            Self::OwnerTombstone => "owner_tombstone",
            Self::LocalBlock => "local_block",
            Self::Quarantined => "quarantined",
        }
    }

    /// Return a metric-safe label only when this reason refuses serving.
    #[must_use]
    pub const fn refusal_label(self) -> Option<&'static str> {
        match self {
            Self::GlobalDeny | Self::OwnerTombstone | Self::LocalBlock | Self::Quarantined => {
                Some(self.as_str())
            }
            Self::NoRule | Self::LocalAllow => None,
        }
    }
}

/// Deterministic moderation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// Serve posture.
    pub effect: Effect,
    /// Stable reason for the selected posture.
    pub reason: ReasonCode,
}

impl Decision {
    /// Whether the decision permits serving.
    #[must_use]
    pub const fn permits_serve(self) -> bool {
        self.effect.permits_serve()
    }

    const fn serve(reason: ReasonCode) -> Self {
        Self {
            effect: Effect::Serve,
            reason,
        }
    }

    const fn refuse(reason: ReasonCode) -> Self {
        Self {
            effect: Effect::Refuse,
            reason,
        }
    }

    const fn quarantine() -> Self {
        Self {
            effect: Effect::Quarantine,
            reason: ReasonCode::Quarantined,
        }
    }
}

/// Exact-hash moderation policy.
///
/// Precedence is intentionally fixed:
///
/// 1. global deny
/// 2. owner tombstone
/// 3. local block
/// 4. quarantine
/// 5. local allow
/// 6. no rule
///
/// A local allow never overrides a stronger refusal state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    global_deny: BTreeSet<B3Id>,
    local_block: BTreeSet<B3Id>,
    local_allow: BTreeSet<B3Id>,
    owner_tombstone: BTreeSet<B3Id>,
    quarantine: BTreeSet<B3Id>,
}

impl Policy {
    /// Compose authenticated global state with operator-local state.
    ///
    /// Signed global input may contain only `global_deny` and
    /// `owner_tombstone`. Local input may contain only `local_block`,
    /// `local_allow`, and `quarantine`. Existing precedence remains unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`CompositionError`] if either source crosses its authority
    /// boundary.
    pub fn compose_signed_global_with_local(
        signed_global: &Self,
        local: &Self,
    ) -> Result<Self, CompositionError> {
        if !signed_global.local_block.is_empty()
            || !signed_global.local_allow.is_empty()
            || !signed_global.quarantine.is_empty()
        {
            return Err(CompositionError::SignedGlobalContainsOperatorState);
        }

        if !local.global_deny.is_empty() || !local.owner_tombstone.is_empty() {
            return Err(CompositionError::LocalPolicyContainsGlobalState);
        }

        Ok(Self {
            global_deny: signed_global.global_deny.clone(),
            local_block: local.local_block.clone(),
            local_allow: local.local_allow.clone(),
            owner_tombstone: signed_global.owner_tombstone.clone(),
            quarantine: local.quarantine.clone(),
        })
    }

    /// Evaluate one exact canonical object identifier.
    #[must_use]
    pub fn evaluate(&self, object: &B3Id) -> Decision {
        if self.global_deny.contains(object) {
            return Decision::refuse(ReasonCode::GlobalDeny);
        }

        if self.owner_tombstone.contains(object) {
            return Decision::refuse(ReasonCode::OwnerTombstone);
        }

        if self.local_block.contains(object) {
            return Decision::refuse(ReasonCode::LocalBlock);
        }

        if self.quarantine.contains(object) {
            return Decision::quarantine();
        }

        if self.local_allow.contains(object) {
            return Decision::serve(ReasonCode::LocalAllow);
        }

        Decision::serve(ReasonCode::NoRule)
    }

    /// Add an exact object to the global deny set.
    #[must_use]
    pub fn insert_global_deny(&mut self, object: B3Id) -> bool {
        self.global_deny.insert(object)
    }

    /// Add an exact object to the operator-local block set.
    #[must_use]
    pub fn insert_local_block(&mut self, object: B3Id) -> bool {
        self.local_block.insert(object)
    }

    /// Add an exact object to the operator-local allow set.
    #[must_use]
    pub fn insert_local_allow(&mut self, object: B3Id) -> bool {
        self.local_allow.insert(object)
    }

    /// Add an exact owner tombstone.
    #[must_use]
    pub fn insert_owner_tombstone(&mut self, object: B3Id) -> bool {
        self.owner_tombstone.insert(object)
    }

    /// Add an exact object to quarantine.
    #[must_use]
    pub fn insert_quarantine(&mut self, object: B3Id) -> bool {
        self.quarantine.insert(object)
    }

    /// Remove an exact object from the operator-local block set.
    #[must_use]
    pub fn remove_local_block(&mut self, object: &B3Id) -> bool {
        self.local_block.remove(object)
    }

    /// Remove an exact object from the operator-local allow set.
    #[must_use]
    pub fn remove_local_allow(&mut self, object: &B3Id) -> bool {
        self.local_allow.remove(object)
    }

    /// Remove an exact object from quarantine.
    #[must_use]
    pub fn remove_quarantine(&mut self, object: &B3Id) -> bool {
        self.quarantine.remove(object)
    }

    /// Number of globally denied objects.
    #[must_use]
    pub fn global_deny_count(&self) -> usize {
        self.global_deny.len()
    }

    /// Number of locally blocked objects.
    #[must_use]
    pub fn local_block_count(&self) -> usize {
        self.local_block.len()
    }

    /// Number of locally allowed objects.
    #[must_use]
    pub fn local_allow_count(&self) -> usize {
        self.local_allow.len()
    }

    /// Number of owner tombstones.
    #[must_use]
    pub fn owner_tombstone_count(&self) -> usize {
        self.owner_tombstone.len()
    }

    /// Number of quarantined objects.
    #[must_use]
    pub fn quarantine_count(&self) -> usize {
        self.quarantine.len()
    }
}
