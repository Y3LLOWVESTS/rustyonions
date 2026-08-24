//! RO:WHAT — Compose svc-passport's canonical constrained identity router inside CrabNode, with CN-4 durable Native Passport startup/recovery when public CrabNode state is injected.
//! RO:WHY — CrabNode identity readiness must depend on restart-stable service signing custody and recoverable RegisterRoot state without inheriting full svc-passport KMS/admin surfaces.
//! RO:INTERACTS — svc_passport constrained router, DurableRonKmsClient, ron-kms durable service key, UsernameClaimStore, ReadyProbes, Omnigate profile proxy.
//! RO:INVARIANTS — loopback-only bind; profile and Native Passport truth remain svc-passport-owned; public CrabNode native state is durable; pending RegisterRoot recovery completes before readiness.
//! RO:METRICS — constrained router registers no svc-passport process-global metrics.
//! RO:CONFIG — RON_PASSPORT_ADDR, RON_PASSPORT_PROFILE_DATA_DIR, and RON_PASSPORT_NATIVE_DATA_DIR injected by public CrabNode.
//! RO:SECURITY — no DevKms, capability issue/verify, key export, KMS admin, wallet, or ledger authority; durable service key remains ron-kms custody.
//! RO:TEST — svc-passport CN-4 native runtime/adapter tests plus macronode crabnode_cn3_ingress and crabnode_cn4_profile_restart.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use tokio::{net::TcpListener, time::sleep};

use tracing::{error, info};

use crate::{
    errors::{Error, Result},
    readiness::ReadyProbes,
    services::ports,
    supervisor::{ManagedTask, ShutdownToken},
};

const HEALTH_ATTEMPTS: usize = 80;
const HEALTH_RETRY: Duration = Duration::from_millis(25);
const HEALTH_TIMEOUT: Duration = Duration::from_millis(250);

const PASSPORT_NATIVE_DATA_DIR_ENV: &str = "RON_PASSPORT_NATIVE_DATA_DIR";

const NATIVE_NETWORK_ID: &str = "rustyonions-devnet";

const NATIVE_ENVIRONMENT: &str = "private-beta";

const NATIVE_AUDIENCE: &str = "svc-passport";

const NATIVE_ISSUING_SERVICE_ID: &str = "svc-passport";

const NATIVE_CHALLENGE_TTL_MS: u64 = 60_000;

const NATIVE_REPLAY_RETENTION_MS: u64 = 120_000;

const NATIVE_INITIAL_ROOT_KEY_EPOCH: u64 = 0;

fn resolve_bind_addr() -> Result<SocketAddr> {
    let raw = std::env::var("RON_PASSPORT_ADDR")
        .unwrap_or_else(|_| ports::DEFAULT_PASSPORT_ADDR_STR.to_owned());

    let addr = raw
        .trim()
        .parse::<SocketAddr>()
        .map_err(|err| Error::config(format!("invalid RON_PASSPORT_ADDR {raw:?}: {err}")))?;

    if !addr.ip().is_loopback() {
        return Err(Error::config(format!(
            "CrabNode internal svc-passport must remain loopback-only; refusing {addr}"
        )));
    }

    Ok(addr)
}

fn resolve_native_data_root() -> Result<Option<PathBuf>> {
    match std::env::var(PASSPORT_NATIVE_DATA_DIR_ENV) {
        Ok(raw) => {
            let root = raw.trim();

            if root.is_empty() {
                return Err(Error::config(
                    "RON_PASSPORT_NATIVE_DATA_DIR must not be empty",
                ));
            }

            Ok(Some(PathBuf::from(root)))
        }

        Err(std::env::VarError::NotPresent) => Ok(None),

        Err(std::env::VarError::NotUnicode(_)) => Err(Error::config(
            "RON_PASSPORT_NATIVE_DATA_DIR must be valid Unicode",
        )),
    }
}

pub async fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> Result<ManagedTask> {
    probes.set_passport_bound(false);

    let expected_bind = resolve_bind_addr()?;

    let listener = TcpListener::bind(expected_bind).await?;

    let actual_bind = listener.local_addr()?;

    if actual_bind != expected_bind {
        return Err(Error::config(format!(
            "svc-passport canonical bind mismatch: expected {expected_bind}, got {actual_bind}"
        )));
    }

    let (profile_store, persistence_mode) = match std::env::var("RON_PASSPORT_PROFILE_DATA_DIR") {
        Ok(raw) => {
            let root = raw.trim();

            if root.is_empty() {
                return Err(Error::config(
                    "RON_PASSPORT_PROFILE_DATA_DIR must not be empty",
                ));
            }

            let store =
                svc_passport::profile::UsernameClaimStore::open_durable(root).map_err(|err| {
                    Error::config(format!(
                        "durable svc-passport profile store rejected: {err}"
                    ))
                })?;

            (Arc::new(store), "durable")
        }

        Err(std::env::VarError::NotPresent) => (
            Arc::new(svc_passport::profile::UsernameClaimStore::new()),
            "ephemeral_internal",
        ),

        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(Error::config(
                "RON_PASSPORT_PROFILE_DATA_DIR must be valid Unicode",
            ));
        }
    };

    let native_data_root = resolve_native_data_root()?;

    let (app, native_runtime_mode) = if let Some(root) = native_data_root {
        let service_key_root = root.join("service-key");

        let challenge_root = root.join("challenges");

        let registry_root = root.join("registry");

        let transaction_root = root.join("root-registration-redo");

        /*
         * ron-kms owns persistent secret custody. A corrupt existing key
         * fails closed and is never replaced merely to make startup pass.
         */
        let durable_key = ron_kms::DurableEd25519ServiceKey::open_or_create(
            &service_key_root,
            "crabnode",
            "svc-passport",
        )
        .map_err(|err| {
            Error::config(format!(
                "durable CrabNode svc-passport service key rejected: {err}"
            ))
        })?;

        let adapter =
            svc_passport::kms::DurableRonKmsClient::new(Arc::new(durable_key)).map_err(|err| {
                Error::config(format!(
                    "durable CrabNode svc-passport KMS adapter rejected: {err}"
                ))
            })?;

        let kms: Arc<dyn svc_passport::kms::client::KmsClient> = Arc::new(adapter);

        let runtime_config = svc_passport::native::NativePassportServerRuntimeMountConfigV1 {
            challenge_root,
            registry_root,
            transaction_root,

            network_id: NATIVE_NETWORK_ID.to_owned(),

            environment: NATIVE_ENVIRONMENT.to_owned(),

            audience: NATIVE_AUDIENCE.to_owned(),

            issuing_service_id: NATIVE_ISSUING_SERVICE_ID.to_owned(),

            challenge_ttl_ms: NATIVE_CHALLENGE_TTL_MS,

            replay_retention_ms: NATIVE_REPLAY_RETENTION_MS,

            trusted_initial_root_key_epoch: NATIVE_INITIAL_ROOT_KEY_EPOCH,
        };

        let app = svc_passport::http::router::build_native_profile_router_with_store_and_kms(
            profile_store,
            kms,
            runtime_config,
        )
        .await
        .map_err(|err| {
            Error::config(format!(
                "CrabNode Native Passport recovery-gated mount rejected: {err}"
            ))
        })?;

        (app, "durable_recovery_gated")
    } else {
        /*
         * Direct/internal macronode development may still use the
         * historical profile-only composition. Public CrabNode always
         * injects its canonical native state directory below.
         */
        (
            svc_passport::http::router::build_profile_router_with_store(profile_store),
            "not_mounted_internal",
        )
    };

    let service_shutdown = shutdown.clone();

    let handle = tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                while !service_shutdown.is_triggered() {
                    sleep(Duration::from_millis(50)).await;
                }
            })
            .await;

        if let Err(err) = result {
            error!(
                %err,
                "CrabNode constrained svc-passport server failed"
            );
        }
    });

    let client = reqwest::Client::builder()
        .timeout(HEALTH_TIMEOUT)
        .build()
        .map_err(|err| {
            Error::Other(anyhow::anyhow!(
                "svc-passport health client build failed: {err}"
            ))
        })?;

    let health_url = format!("http://{actual_bind}/healthz");

    let mut healthy = false;

    for _ in 0..HEALTH_ATTEMPTS {
        if handle.is_finished() {
            break;
        }

        if let Ok(response) = client.get(&health_url).send().await {
            if response.status().is_success() {
                healthy = true;
                break;
            }
        }

        sleep(HEALTH_RETRY).await;
    }

    if !healthy {
        handle.abort();

        let _ = handle.await;

        return Err(Error::config(format!(
            "svc-passport constrained health probe failed at {health_url}"
        )));
    }

    /*
     * For public CrabNode, this bit can only become true after durable KMS
     * open, Native Passport store open/recovery, router construction, listener
     * serve, and an actual HTTP health response have all succeeded.
     */
    probes.set_passport_bound(true);

    info!(
        %actual_bind,
        persistence_mode,
        native_runtime_mode,
        "CrabNode svc-passport identity plane ready"
    );

    Ok(ManagedTask::new("svc-passport", handle))
}
