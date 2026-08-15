//! RO:WHAT — Store abstraction; sled-backed or in-memory.
//! RO:WHY — svc-index owns mutable manifest pointers, not raw bytes or wallet/ledger truth.
//! RO:INTERACTS — store::{keys,sled_store}, types::{AssetManifestPointer, SiteManifestPointer}.
//! RO:INVARIANTS — raw content bytes are never stored here; pointer values are strict JSON.
//! RO:METRICS — none directly.
//! RO:CONFIG — `Config.enable_sled`, `RON_INDEX_DB` through sled backend.
//! RO:SECURITY — caller must validate keys and DTOs before storage.
//! RO:TEST — integration.rs.

pub mod keys;

mod sled_store;

use crate::{
    publications::{
        build_publication_page,
        normalize_publication_id,
        normalize_username,
        PublicationPageRequest,
        PublicationPageV1,
        PublicationProjectionError,
        PublicationSummaryV1,
    },
    types::{
        AssetManifestPointer,
        SiteManifestPointer,
    },
};

/// svc-index storage backend.
#[derive(Clone)]
pub enum Store {
    /// Sled-backed persistent store.
    #[cfg(feature = "sled-store")]
    Sled(self::sled_store::SledStore),
    /// In-memory store for dev/test/amnesia-friendly use.
    Memory(self::sled_store::MemStore),
}

impl Store {
    /// Open the configured store.
    pub fn new(enable_sled: bool) -> anyhow::Result<Self> {
        if cfg!(feature = "sled-store") && enable_sled {
            Ok(Self::Sled(self::sled_store::SledStore::open()?))
        } else {
            Ok(Self::Memory(self::sled_store::MemStore::default()))
        }
    }

    /// Get a legacy manifest pointer value.
    pub fn get_manifest(&self, key: &str) -> Option<String> {
        self.get_value(key)
    }

    /// Put a legacy manifest pointer value.
    pub fn put_manifest(&self, key: &str, cid: &str) {
        self.put_value(key, cid);
    }

    /// Store an asset manifest pointer record.
    pub fn put_asset_manifest_pointer(&self, pointer: &AssetManifestPointer) -> anyhow::Result<()> {
        let key = keys::asset_manifest_key(&pointer.asset_cid);
        let value = serde_json::to_string(pointer)?;
        self.put_value(&key, &value);
        Ok(())
    }

    /// Fetch an asset manifest pointer record.
    pub fn get_asset_manifest_pointer(&self, asset_cid: &str) -> Option<AssetManifestPointer> {
        let key = keys::asset_manifest_key(asset_cid);
        self.get_value(&key)
            .and_then(|value| serde_json::from_str::<AssetManifestPointer>(&value).ok())
    }

    /// Store a site manifest pointer record.
    pub fn put_site_manifest_pointer(&self, pointer: &SiteManifestPointer) -> anyhow::Result<()> {
        let key = keys::site_manifest_key(&pointer.name);
        let value = serde_json::to_string(pointer)?;
        self.put_value(&key, &value);
        Ok(())
    }

    /// Fetch a site manifest pointer record.
    pub fn get_site_manifest_pointer(&self, name: &str) -> Option<SiteManifestPointer> {
        let key = keys::site_manifest_key(name);
        self.get_value(&key)
            .and_then(|value| serde_json::from_str::<SiteManifestPointer>(&value).ok())
    }


    /// Store one validated creator-publication projection.
    pub fn put_creator_publication(
        &self,
        publication: &PublicationSummaryV1,
    ) -> anyhow::Result<()> {
        publication
            .validate()
            .map_err(anyhow::Error::new)?;

        let key =
            keys::creator_publication_key(
                &publication.creator.username,
                &publication.publication_id,
            );

        let value =
            serde_json::to_string(
                publication,
            )?;

        self.put_value(
            &key,
            &value,
        );

        Ok(())
    }

    /// Fetch one validated creator-publication projection.
    pub fn get_creator_publication(
        &self,
        username: &str,
        publication_id: &str,
    ) -> Result<
        Option<PublicationSummaryV1>,
        PublicationProjectionError,
    > {
        let username =
            normalize_username(username)?;

        let publication_id =
            normalize_publication_id(
                publication_id,
            )?;

        let key =
            keys::creator_publication_key(
                &username,
                &publication_id,
            );

        let publication =
            self
                .get_value(&key)
                .and_then(
                    |value| {
                        serde_json::from_str::<
                            PublicationSummaryV1,
                        >(&value)
                        .ok()
                    },
                )
                .filter(
                    |value| {
                        value.validate().is_ok()
                    },
                )
                .filter(
                    |value| {
                        value.creator.username
                            == username
                    },
                )
                .filter(
                    |value| {
                        value.publication_id
                            == publication_id
                    },
                );

        Ok(publication)
    }

    /// List one creator's public publications with bounded pagination.
    pub fn list_creator_publications(
        &self,
        username: &str,
        request: &PublicationPageRequest,
    ) -> Result<
        PublicationPageV1,
        PublicationProjectionError,
    > {
        let username =
            normalize_username(username)?;

        let prefix =
            keys::creator_publication_prefix(
                &username,
            );

        let publications =
            self
                .scan_prefix_values(
                    &prefix,
                )
                .into_iter()
                .filter_map(
                    |value| {
                        serde_json::from_str::<
                            PublicationSummaryV1,
                        >(&value)
                        .ok()
                    },
                )
                .filter(
                    |value| {
                        value.creator.username
                            == username
                    },
                )
                .collect();

        build_publication_page(
            publications,
            request,
        )
    }

    /// List every validated public creator-publication
    /// projection for deterministic Explore composition.
    ///
    /// Ordering is intentionally left to the discovery
    /// projection because storage backends do not own
    /// public product ordering.
    pub fn list_public_creator_publications_for_discovery(
        &self,
    ) -> Vec<PublicationSummaryV1> {
        self
            .scan_prefix_values(
                keys::CREATOR_PUBLICATION_PREFIX,
            )
            .into_iter()
            .filter_map(
                |value| {
                    serde_json::from_str::<
                        PublicationSummaryV1,
                    >(
                        &value,
                    )
                    .ok()
                },
            )
            .filter(
                |value| {
                    value
                        .validate()
                        .is_ok()
                },
            )
            .filter(
                PublicationSummaryV1::
                    is_public_timeline_item,
            )
            .collect()
    }


    /// Store one validated publication relation projection.
    ///
    /// This persists relation metadata only. It does not store Comment bytes,
    /// mutate publication bytes, or grant moderation/economic authority.
    pub fn put_publication_relation(
        &self,
        relation: &crate::relations::PublicationRelationV1,
    ) -> anyhow::Result<()> {
        relation
            .validate()
            .map_err(anyhow::Error::new)?;

        let parent =
            crate::relations::
                normalize_relation_parent_crab_url(
                    &relation.parent_crab_url,
                )
                .map_err(anyhow::Error::new)?;

        let key =
            keys::publication_relation_key(
                &parent,
                &relation
                    .publication
                    .publication_id,
            );

        let value =
            serde_json::to_string(
                relation,
            )?;

        self.put_value(
            &key,
            &value,
        );

        Ok(())
    }

    /// List one exact parent's safe public relation projection.
    ///
    /// Private and unlisted relations are filtered by the relation page model.
    /// Public, deleted, blocked, and moderated records remain available so
    /// product readers can project truthful placeholders.
    pub fn list_publication_relations(
        &self,
        parent_crab_url: &str,
        request: &crate::relations::PublicationRelationPageRequest,
    ) -> Result<
        crate::relations::PublicationRelationPageV1,
        crate::relations::PublicationRelationError,
    > {
        let parent =
            crate::relations::
                normalize_relation_parent_crab_url(
                    parent_crab_url,
                )?;

        let prefix =
            keys::publication_relation_prefix(
                &parent,
            );

        let relations =
            self
                .scan_prefix_values(
                    &prefix,
                )
                .into_iter()
                .filter_map(
                    |value| {
                        serde_json::from_str::<
                            crate::relations::
                                PublicationRelationV1,
                        >(
                            &value,
                        )
                        .ok()
                    },
                )
                .filter(
                    |relation| {
                        relation
                            .parent_crab_url
                            ==
                            parent
                    },
                )
                .collect();

        crate::relations::
            build_publication_relation_page(
                relations,
                request,
            )
    }


    /// Store one validated named-Site root-publication projection.
    ///
    /// This stores display/index metadata only. It does not create immutable
    /// publication bytes, establish creator identity, or mutate moderation,
    /// wallet, ledger, receipt, entitlement, QuickChain, ROX, or Solana state.
    pub fn put_site_publication(
        &self,

        publication:
            &crate::site_publications::
                SitePublicationV1,
    ) -> anyhow::Result<()> {
        publication
            .validate()
            .map_err(
                anyhow::Error::new,
            )?;

        let site =
            crate::site_publications::
                normalize_site_publication_site_crab_url(
                    &publication
                        .site_crab_url,
                )
                .map_err(
                    anyhow::Error::new,
                )?;

        let key =
            keys::site_publication_key(
                &site,
                &publication
                    .publication_id,
            );

        let value =
            serde_json::to_string(
                publication,
            )?;

        self.put_value(
            &key,
            &value,
        );

        Ok(())
    }

    /// List one exact named Site's bounded public root-publication projection.
    pub fn list_site_publications(
        &self,

        site_crab_url:
            &str,

        request:
            &crate::site_publications::
                SitePublicationPageRequest,
    ) -> Result<
        crate::site_publications::
            SitePublicationPageV1,

        crate::site_publications::
            SitePublicationError,
    > {
        let site =
            crate::site_publications::
                normalize_site_publication_site_crab_url(
                    site_crab_url,
                )?;

        let prefix =
            keys::site_publication_prefix(
                &site,
            );

        let publications =
            self
                .scan_prefix_values(
                    &prefix,
                )
                .into_iter()
                .filter_map(
                    |value| {
                        serde_json::from_str::<
                            crate::site_publications::
                                SitePublicationV1
                        >(
                            &value,
                        )
                        .ok()
                    },
                )
                .filter(
                    |publication| {
                        publication
                            .validate()
                            .is_ok()
                    },
                )
                .filter(
                    |publication| {
                        publication
                            .site_crab_url
                            ==
                            site
                    },
                )
                .collect();

        crate::site_publications::
            build_site_publication_page(
                publications,
                request,
            )
    }


    fn get_value(&self, key: &str) -> Option<String> {
        match self {
            #[cfg(feature = "sled-store")]
            Store::Sled(store) => store.get_manifest(key),
            Store::Memory(store) => store.get_manifest(key),
        }
    }


    fn scan_prefix_values(
        &self,
        prefix: &str,
    ) -> Vec<String> {
        match self {
            #[cfg(feature = "sled-store")]
            Store::Sled(store) => {
                store.scan_prefix(prefix)
            }
            Store::Memory(store) => {
                store.scan_prefix(prefix)
            }
        }
    }

    fn put_value(&self, key: &str, value: &str) {
        match self {
            #[cfg(feature = "sled-store")]
            Store::Sled(store) => store.put_manifest(key, value),
            Store::Memory(store) => store.put_manifest(key, value),
        }
    }
}
