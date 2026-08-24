//! RO:WHAT — Shared strict Site-manifest read DTOs used by Site resolution and paid Site visits.
//! RO:WHY — FINAL_BETA Phase 16 removes parser drift that previously caused valid created Site manifests to fail paid-visit hydration.
//! RO:INTERACTS — routes/v1/sites.rs, routes/v1/site_visit.rs, svc-storage manifest bytes.
//! RO:INVARIANTS — one strict manifest shape; unknown top-level fields reject; B3/name/business validation remains owned by route callers.
//! RO:METRICS — none directly.
//! RO:CONFIG — none.
//! RO:SECURITY — parsing grants no ownership, payment, receipt, wallet, ledger, or Passport authority.
//! RO:TEST — final_beta_phase16_site_manifest_shared.rs plus existing site_launch.rs and site_visit.rs.

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Canonical read shape for one stored named-Site manifest.
///
/// This is intentionally shared by normal Site hydration and paid Site-visit
/// hydration so those routes cannot drift into accepting different manifest
/// schemas.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SiteManifestDocument {
    pub(super) version: u16,
    pub(super) site_name: String,
    pub(super) root_document_cid: String,

    #[serde(default)]
    pub(super) asset_map: BTreeMap<String, String>,

    #[serde(default)]
    pub(super) route_map: BTreeMap<String, String>,

    #[serde(default)]
    pub(super) owner: Option<SiteManifestOwner>,

    #[serde(default)]
    pub(super) payout: Option<SiteManifestPayout>,

    #[serde(default)]
    pub(super) metadata: Option<SiteManifestMetadata>,

    #[allow(dead_code)]
    #[serde(default)]
    pub(super) rendering: Option<Value>,

    #[allow(dead_code)]
    #[serde(default)]
    pub(super) provenance: Option<Value>,

    #[allow(dead_code)]
    #[serde(default)]
    pub(super) storage: Option<Value>,

    #[serde(default)]
    pub(super) receipts: Vec<Value>,
}

/// Display/reference owner metadata carried by a Site manifest.
///
/// These strings are references only. Parsing them does not prove caller
/// ownership of the referenced Passport or wallet.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SiteManifestOwner {
    #[serde(default)]
    pub(super) passport_subject: Option<String>,

    #[serde(default)]
    pub(super) wallet_account: Option<String>,
}

/// Site payout configuration consumed by existing backend payment flows.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SiteManifestPayout {
    #[serde(default)]
    pub(super) default_action: Option<String>,

    #[serde(default)]
    pub(super) recipient_account: Option<String>,

    #[allow(dead_code)]
    #[serde(default)]
    pub(super) splits: Vec<Value>,
}

/// Public Site metadata consumed by Site hydration and paid-Site context reads.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SiteManifestMetadata {
    #[serde(default)]
    pub(super) title: Option<String>,

    #[serde(default)]
    pub(super) description: Option<String>,

    #[serde(default)]
    pub(super) tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::SiteManifestDocument;

    #[test]
    fn full_created_site_manifest_shape_parses_through_one_shared_contract() {
        let value = serde_json::json!({
            "version": 1,
            "site_name": "rusty-forum",
            "root_document_cid":
                "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "asset_map": {
                "/hero":
                    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            },
            "route_map": {
                "/": "root"
            },
            "owner": {
                "passport_subject": "passport:main:creator",
                "wallet_account": "acct_creator"
            },
            "payout": {
                "default_action": "site_visit",
                "recipient_account": "acct_creator",
                "splits": []
            },
            "metadata": {
                "title": "Rusty Forum",
                "description": "Structured Site",
                "tags": ["forum"]
            },
            "rendering": {
                "mode": "structured"
            },
            "provenance": {
                "template_id": "forum",
                "template_version": 1,
                "renderer_version": "crablink.safe-html.v3"
            },
            "storage": {
                "kind": "b3"
            },
            "receipts": []
        });

        let parsed: SiteManifestDocument =
            serde_json::from_value(value).expect("full generated Site manifest must parse");

        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.site_name, "rusty-forum");
        assert_eq!(
            parsed
                .payout
                .as_ref()
                .and_then(|value| value.default_action.as_deref()),
            Some("site_visit"),
        );
        assert_eq!(
            parsed
                .owner
                .as_ref()
                .and_then(|value| value.wallet_account.as_deref()),
            Some("acct_creator"),
        );
    }

    #[test]
    fn unknown_site_manifest_fields_still_fail_closed() {
        let value = serde_json::json!({
            "version": 1,
            "site_name": "rusty-forum",
            "root_document_cid":
                "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "asset_map": {},
            "route_map": {},
            "owner": null,
            "payout": null,
            "metadata": null,
            "rendering": null,
            "provenance": null,
            "storage": null,
            "receipts": [],
            "wallet_authority": true
        });

        let error = serde_json::from_value::<SiteManifestDocument>(value)
            .expect_err("unknown authority-shaped fields must reject");

        assert!(error.to_string().contains("unknown field"));
    }
}
