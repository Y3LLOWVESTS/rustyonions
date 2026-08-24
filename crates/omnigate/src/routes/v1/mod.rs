//! RO:QUICKCHAIN-PREFLIGHT — no ledger mutation here; wallet mutations are proxied only through svc-wallet.
//! RO:QUICKCHAIN-PREFLIGHT — omnigate is hydration/product coordination, not chain/runtime/root/finality authority.
//! RO:WHAT   v1 API surface aggregator for health, facets, paid routes, crab assets, text assets, content views, sites, identity, passport profile, and wallet façade routes.
//! RO:WHY    P6/P7/P12; Concerns: DX/SEC/ECON. Keep top-level routing slim while exposing stable product contracts.
//! RO:INTERACTS — routes/v1/* modules, svc-gateway product proxy, CrabLink extension.
//! RO:INVARIANTS — DTO-stable shapes; no ledger mutation here; wallet mutations are proxied only through svc-wallet.
//! RO:METRICS — route-specific middleware wraps this subtree from the app bootstrap.
//! RO:CONFIG — child modules read their own env/config knobs.
//! RO:SECURITY — no ambient authority; child routes enforce/forward capability context.
//! RO:TEST — omnigate route tests plus svc-gateway proxy tests and CrabLink smoke scripts.

pub mod app;
pub mod assets;
pub mod chat;
pub mod content_view;
pub mod crab;
pub mod creator_publications;
pub mod dht;
pub mod explore_discovery;
pub mod facet;
pub(crate) mod header_policy;
pub mod identity;
pub mod index;
pub mod mailbox;
pub mod objects;
pub mod paid;
pub mod profile;
pub mod publication_relations;
mod site_manifest;
pub mod site_publications;
pub mod site_visit;
pub mod sites;
pub mod streams;
pub mod text_assets;
pub mod wallet;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

const NATIVE_PASSPORT_FIXED_BODY_LIMIT_BYTES: usize = 16_384;

/// Compose the whole v1 subtree.
///
/// Mount with:
///
/// ```ignore
/// .nest("/v1", routes::v1::router())
/// ```
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(index::router()) // includes /ping and /index/healthz
        .nest("/objects", objects::router())
        .nest("/mailbox", mailbox::router())
        .nest("/dht", dht::router())
        .nest("/facet", facet::router())
        .nest("/app", app::router())
        .nest("/paid", paid::router())
        .nest("/assets", assets::router().merge(text_assets::router()))
        .nest("/content", content_view::router())
        .nest("/chat", chat::router())
        .nest("/streams", streams::router())
        .nest("/identity", identity::router())
        .route(
            "/identity/passport/register/challenge",
            post(profile::register_root_challenge).route_layer(DefaultBodyLimit::max(
                NATIVE_PASSPORT_FIXED_BODY_LIMIT_BYTES,
            )),
        )
        .route(
            "/identity/passport/register/proof",
            post(profile::register_root_proof).route_layer(DefaultBodyLimit::max(
                NATIVE_PASSPORT_FIXED_BODY_LIMIT_BYTES,
            )),
        )
        .route(
            "/identity/passport/device/authorize",
            post(profile::device_authorize).route_layer(DefaultBodyLimit::max(
                NATIVE_PASSPORT_FIXED_BODY_LIMIT_BYTES,
            )),
        )
        .route(
            "/identity/passport/challenge",
            post(profile::device_session_challenge).route_layer(DefaultBodyLimit::max(
                NATIVE_PASSPORT_FIXED_BODY_LIMIT_BYTES,
            )),
        )
        .route(
            "/identity/passport/prove",
            post(profile::device_session_proof).route_layer(DefaultBodyLimit::max(
                NATIVE_PASSPORT_FIXED_BODY_LIMIT_BYTES,
            )),
        )
        .route(
            "/identity/passport/profile/claim",
            post(profile::claim_profile),
        )
        .route(
            "/identity/passport/profile/:username",
            get(profile::get_profile),
        )
        .route("/explore", get(explore_discovery::get_explore_discovery))
        .route(
            "/publication-relations",
            get(publication_relations::list_publication_relations),
        )
        .route(
            "/site-publications",
            get(site_publications::list_site_publications),
        )
        .route(
            "/creators/:username/publications",
            get(creator_publications::list_creator_publications),
        )
        .route(
            "/creators/:username/publications/:publication_id",
            get(creator_publications::get_creator_publication),
        )
        .route("/wallet/:account/balance", get(wallet::balance))
        .route("/wallet/hold", post(wallet::hold))
        .route("/sites/prepare", post(sites::site_prepare))
        .route("/sites", post(sites::site_create))
        .route("/sites/:name", get(sites::site_resolve))
        .route(
            "/sites/:name/visit/quote",
            post(site_visit::site_visit_quote),
        )
        .route("/sites/:name/visit/pay", post(site_visit::site_visit_pay))
        // WEB3_2 product-proof asset-page routes.
        .nest("/crab", crab::router())
        .route("/b3/:asset", get(crab::resolve_b3_asset))
}
