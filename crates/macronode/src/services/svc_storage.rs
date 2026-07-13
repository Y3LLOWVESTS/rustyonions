// crates/macronode/src/services/svc_storage.rs

//! RO:WHAT — Macronode embedded svc-storage HTTP server with optional Phase 8 seed object.
//! RO:WHY  — Run the real CAS storage plane in-process and prove headless service-node serving.
//! RO:INTERACTS — svc_storage router/backend, ReadyProbes, ShutdownToken.
//! RO:INVARIANTS — local-only default; ready only after bind and requested seed verification.
//! RO:CONFIG — RON_STORAGE_ADDR; RON_SERVICE_NODE_SEED_OBJECT.
//! RO:SECURITY — in-memory/amnesia-first; deterministic seed has no economic authority.
//! RO:TEST — service_node_seeded_object, readiness probe, admin/readiness smokes.

#![forbid(unsafe_code)]

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::body::Bytes;
use tokio::{net::TcpListener, task, time::sleep};
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    services::{
        moderation_policy::{
            load_configured_moderation_policy, ConfiguredModerationPolicy, ModerationActivation,
        },
        ports,
    },
    supervisor::{ManagedTask, ShutdownToken},
    types::RuntimeStatus,
};
use svc_storage::{
    http::{
        extractors::AppState,
        server::{build_router, build_router_with_moderation},
    },
    storage::{MemoryStorage, Storage},
};

/// Environment switch for the deterministic service-node seed object.
const SEED_OBJECT_ENV: &str = "RON_SERVICE_NODE_SEED_OBJECT";

/// Official BLAKE3 `abc` test-vector bytes.
///
/// This is deliberately tiny and non-sensitive. It exists only to prove that
/// an enabled headless service node can expose an object that was present
/// before the test client connected.
const PHASE8_SEED_BYTES: &[u8] = b"abc";

/// Canonical BLAKE3 CID for `PHASE8_SEED_BYTES`.
const PHASE8_SEED_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

/// Resolve the embedded storage bind address.
///
/// Default: `127.0.0.1:5303`
/// Override: `RON_STORAGE_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    match std::env::var("RON_STORAGE_ADDR") {
        Ok(raw) => match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-storage: using RON_STORAGE_ADDR={addr}");
                addr
            }
            Err(err) => {
                error!(
                    "svc-storage: invalid RON_STORAGE_ADDR={raw:?}, falling back to {}: {err}",
                    ports::DEFAULT_STORAGE_ADDR_STR
                );
                ports::default_storage_addr()
            }
        },
        Err(_) => ports::default_storage_addr(),
    }
}

/// Parse the opt-in seed-object switch.
///
/// Missing is false. Recognized true and false spellings are accepted.
/// Any other value fails closed so operators are not given fake readiness.
fn seed_object_requested() -> Result<bool, String> {
    let raw = match std::env::var(SEED_OBJECT_ENV) {
        Ok(raw) => raw,
        Err(std::env::VarError::NotPresent) => return Ok(false),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(format!("{SEED_OBJECT_ENV} must contain valid UTF-8"));
        }
    };

    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" | "" => Ok(false),
        _ => Err(format!(
            "{SEED_OBJECT_ENV} must be one of: 1, true, yes, on, 0, false, no, off"
        )),
    }
}

/// Insert and independently verify the deterministic Phase 8 seed object.
///
/// The storage backend calculates its strong ETag from the actual stored
/// bytes. Comparing that ETag with the canonical CID prevents a mistyped
/// constant from being reported as a successfully seeded b3 object.
async fn seed_phase8_object(store: &Arc<dyn Storage>) -> Result<(), String> {
    let bytes = Bytes::from_static(PHASE8_SEED_BYTES);

    store
        .put(PHASE8_SEED_CID, bytes.clone())
        .await
        .map_err(|err| format!("seed put failed: {err}"))?;

    let head = store
        .head(PHASE8_SEED_CID)
        .await
        .map_err(|err| format!("seed head verification failed: {err}"))?;

    let expected_etag = format!("\"{}\"", &PHASE8_SEED_CID[3..]);

    if head.etag != expected_etag {
        return Err(format!(
            "seed integrity mismatch: expected ETag {expected_etag}, got {}",
            head.etag
        ));
    }

    if head.len != PHASE8_SEED_BYTES.len() as u64 {
        return Err(format!(
            "seed length mismatch: expected {}, got {}",
            PHASE8_SEED_BYTES.len(),
            head.len
        ));
    }

    let stored = store
        .get_full(PHASE8_SEED_CID)
        .await
        .map_err(|err| format!("seed read-back failed: {err}"))?;

    if stored.as_ref() != PHASE8_SEED_BYTES {
        return Err("seed read-back bytes did not match the canonical seed".to_string());
    }

    Ok(())
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Spawn the embedded svc-storage HTTP server.
pub fn spawn(
    probes: Arc<ReadyProbes>,
    shutdown: ShutdownToken,
    runtime: Arc<RuntimeStatus>,
) -> ManagedTask {
    let handle = task::spawn(async move {
        let addr = resolve_bind_addr();

        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(err) => {
                probes.set_storage_bound(false);
                error!(?err, %addr, "svc-storage: failed to bind");
                return;
            }
        };

        let moderation = match load_configured_moderation_policy() {
            Ok(ConfiguredModerationPolicy::Active(loaded)) => Some(loaded),
            Ok(ConfiguredModerationPolicy::NotConfigured) => {
                runtime.set_moderation_not_configured();
                None
            }
            Err(err) => {
                runtime.set_moderation_load_failed(err.status_source());
                probes.set_storage_bound(false);
                error!(
                    source = err.status_source(),
                    %err,
                    "svc-storage: moderation policy failed closed"
                );
                return;
            }
        };

        // Storage remains amnesia-first in this slice. Moderation loading
        // changes serve admission only; it does not create persistence.
        let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());

        match seed_object_requested() {
            Ok(true) => {
                if let Err(err) = seed_phase8_object(&store).await {
                    probes.set_storage_bound(false);
                    error!(%err, "svc-storage: requested seed object failed verification");
                    return;
                }

                info!(
                    cid = PHASE8_SEED_CID,
                    bytes = PHASE8_SEED_BYTES.len(),
                    "svc-storage: Phase 8 seed object ready"
                );
            }
            Ok(false) => {
                info!("svc-storage: Phase 8 seed object disabled");
            }
            Err(err) => {
                probes.set_storage_bound(false);
                error!(%err, "svc-storage: invalid seed-object configuration");
                return;
            }
        }

        // Register the exact backend used by the embedded HTTP router.
        // The prune coordinator must never create a replacement store.
        runtime.register_prune_storage(store.clone());

        let state = AppState { store };

        let signed_expires_at_unix_s =
            moderation
                .as_ref()
                .and_then(|loaded| match loaded.activation {
                    ModerationActivation::UnsignedLocal => None,
                    ModerationActivation::SignedGlobal {
                        expires_at_unix_s, ..
                    } => Some(expires_at_unix_s),
                });

        let app = match moderation {
            Some(loaded) => {
                let source = loaded.activation.status_source();

                match loaded.activation {
                    ModerationActivation::UnsignedLocal => {
                        runtime.set_moderation_active(loaded.policy.clone(), loaded.counts);
                    }
                    ModerationActivation::SignedGlobal {
                        epoch,
                        expires_at_unix_s,
                        ..
                    } => {
                        runtime.set_signed_moderation_active(
                            source,
                            loaded.policy.clone(),
                            loaded.counts,
                            epoch,
                            expires_at_unix_s,
                        );
                    }
                }

                info!(
                    source = source,
                    signed = matches!(loaded.activation, ModerationActivation::SignedGlobal { .. }),
                    total_entries = loaded.counts.total(),
                    global_deny = loaded.counts.global_deny,
                    local_block = loaded.counts.local_block,
                    local_allow = loaded.counts.local_allow,
                    owner_tombstone = loaded.counts.owner_tombstone,
                    quarantine = loaded.counts.quarantine,
                    "svc-storage: moderation policy loaded"
                );

                build_router_with_moderation(loaded.policy).with_state(state)
            }
            None => {
                info!("svc-storage: no moderation policy configured");
                build_router().with_state(state)
            }
        };

        // Readiness becomes true only after the listener exists and any
        // requested seed object has passed byte/hash/read-back verification.
        probes.set_storage_bound(true);
        info!("svc-storage: listening on {addr} (embedded in macronode)");

        let expiry_runtime = runtime.clone();
        let expiry_probes = probes.clone();

        let result = axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async move {
                while !shutdown.is_triggered() {
                    if signed_expires_at_unix_s
                        .is_some_and(|expires| now_unix_s() >= expires)
                    {
                        expiry_runtime.set_moderation_expired();
                        expiry_probes.set_storage_bound(false);
                        error!(
                            "svc-storage: signed moderation policy expired; storage stopped fail closed"
                        );
                        return;
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            })
            .await;

        runtime.clear_prune_storage();
        runtime.set_moderation_inactive();
        probes.set_storage_bound(false);

        if let Err(err) = result {
            error!(?err, "svc-storage: server error");
        } else {
            info!("svc-storage: server exited cleanly");
        }
    });

    ManagedTask::new("svc-storage", handle)
}
