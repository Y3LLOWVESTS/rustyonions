//! RO:WHAT — Canonical CN-2 CrabNode private-beta topology/profile validator.
//! RO:WHY — One service profile must own a collision-free topology before CN-3 composition.
//! RO:INTERACTS — crabnode lifecycle/state and canonical macronode service ports.
//! RO:INVARIANTS — unique ports, private admin, one client ingress, strict internal binds.
//! RO:CONFIG — existing service-specific environment overrides remain explicit inputs.
//! RO:SECURITY — admin/internal services remain loopback; local state permissions stay private.
//! RO:TEST — crabnode_profile_cli.rs plus profile unit tests.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    env, fs,
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use super::{canonical_ports, crabnode_state::RuntimeState, CrabnodeError};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(350);

pub(super) struct EffectiveTopology {
    admin: SocketAddr,
    gateway: SocketAddr,
    omnigate: SocketAddr,
    passport: SocketAddr,
    overlay: SocketAddr,
    dht: SocketAddr,
    storage: SocketAddr,
    index: SocketAddr,
    mailbox: SocketAddr,
    svc_admin: SocketAddr,
}

pub(super) fn resolve_and_validate(
    state: &RuntimeState,
    admin: SocketAddr,
) -> Result<EffectiveTopology, CrabnodeError> {
    let legacy_gateway = optional_env_addr("RON_GATEWAY_ADDR")?;

    let full_gateway = optional_env_addr("SVC_GATEWAY_BIND_ADDR")?;

    if let (Some(legacy), Some(full)) = (legacy_gateway, full_gateway) {
        if legacy != full {
            return Err(CrabnodeError::config(format!(
                "unsupported CrabNode profile combination: \
                     RON_GATEWAY_ADDR={legacy} disagrees with \
                     SVC_GATEWAY_BIND_ADDR={full}"
            )));
        }
    }

    let topology = EffectiveTopology {
        admin,
        gateway: full_gateway.or(legacy_gateway).unwrap_or(parse_default(
            canonical_ports::DEFAULT_GATEWAY_ADDR_STR,
            "gateway",
        )?),
        omnigate: env_addr_or_default(
            "OMNIGATE_BIND",
            canonical_ports::DEFAULT_OMNIGATE_ADDR_STR,
            "omnigate",
        )?,
        passport: env_addr_or_default(
            "RON_PASSPORT_ADDR",
            canonical_ports::DEFAULT_PASSPORT_ADDR_STR,
            "passport",
        )?,
        overlay: env_addr_or_default(
            "RON_OVERLAY_ADDR",
            canonical_ports::DEFAULT_OVERLAY_ADDR_STR,
            "overlay",
        )?,
        dht: env_addr_or_default("RON_DHT_ADDR", canonical_ports::DEFAULT_DHT_ADDR_STR, "dht")?,
        storage: env_addr_or_default(
            "RON_STORAGE_ADDR",
            canonical_ports::DEFAULT_STORAGE_ADDR_STR,
            "storage",
        )?,
        index: env_addr_or_default(
            "INDEX_BIND",
            canonical_ports::DEFAULT_INDEX_BIND_STR,
            "index",
        )?,
        mailbox: env_addr_or_default(
            "RON_MAILBOX_ADDR",
            canonical_ports::DEFAULT_MAILBOX_ADDR_STR,
            "mailbox",
        )?,
        svc_admin: parse_default(canonical_ports::DEFAULT_SVC_ADMIN_ADDR_STR, "svc-admin")?,
    };

    topology.validate_port_map()?;
    topology.validate_exposure()?;
    validate_operator_state_permissions(state)?;

    Ok(topology)
}

impl EffectiveTopology {
    pub(super) fn print_doctor_contract(&self) {
        println!("cn2_profile=private_beta");
        println!("cn2_port_map=collision_free");
        println!("cn2_public_admin_default=no");
        println!("cn2_single_client_ingress=yes");

        println!("admin_bind={}", self.admin);
        println!("client_ingress_bind={}", self.gateway);
        println!("omnigate_bind={}", self.omnigate);
        println!("passport_bind={}", self.passport);
        println!("overlay_bind={}", self.overlay);
        println!("dht_bind={}", self.dht);
        println!("storage_bind={}", self.storage);
        println!("index_bind={}", self.index);
        println!("mailbox_bind={}", self.mailbox);

        println!("client_ingress_active_owner=svc-gateway");
        println!("client_ingress_route_surface=full_product_router");
        println!("client_ingress_handoff=complete_cn3");

        println!("omnigate_runtime_owner=canonical_in_process_builder");
        println!("passport_runtime_owner=embedded_svc-passport_profile_only");
        println!("storage_runtime_owner=embedded_svc-storage");
        println!("index_runtime_owner=embedded_svc-index");
        println!("dht_runtime_owner=embedded_svc-dht");
        println!("overlay_runtime_owner=embedded_host_shell_until_cn8");
        println!("mailbox_runtime_owner=optional_host_shell");
        println!("svc_admin_runtime_owner=optional_local_ui");
        println!("operator_state_permissions=verified");
    }

    pub(super) fn validate_running_required_services(&self) -> Result<(), CrabnodeError> {
        for (name, addr) in [
            ("client_ingress", self.gateway),
            ("omnigate", self.omnigate),
            ("passport", self.passport),
            ("storage", self.storage),
            ("index", self.index),
        ] {
            TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|err| {
                CrabnodeError::io(format!(
                    "doctor: required internal service {name} \
                     is unreachable at {addr}: {err}"
                ))
            })?;
        }

        Ok(())
    }

    fn validate_port_map(&self) -> Result<(), CrabnodeError> {
        let entries = [
            ("admin", self.admin),
            ("svc-admin", self.svc_admin),
            ("gateway", self.gateway),
            ("omnigate", self.omnigate),
            ("passport", self.passport),
            ("overlay", self.overlay),
            ("dht", self.dht),
            ("storage", self.storage),
            ("index", self.index),
            ("mailbox", self.mailbox),
        ];

        let mut owners = BTreeMap::<u16, &'static str>::new();

        for (name, addr) in entries {
            if let Some(previous) = owners.insert(addr.port(), name) {
                return Err(CrabnodeError::config(format!(
                    "CrabNode profile port collision: \
                         {previous} and {name} both claim port {}",
                    addr.port()
                )));
            }
        }

        Ok(())
    }

    fn validate_exposure(&self) -> Result<(), CrabnodeError> {
        if !self.admin.ip().is_loopback() {
            return Err(CrabnodeError::config(format!(
                "CrabNode private-beta admin must remain loopback-only; \
                     refusing {}",
                self.admin
            )));
        }

        for (name, addr) in [
            ("svc-admin", self.svc_admin),
            ("omnigate", self.omnigate),
            ("passport", self.passport),
            ("overlay", self.overlay),
            ("dht", self.dht),
            ("storage", self.storage),
            ("index", self.index),
            ("mailbox", self.mailbox),
        ] {
            if !addr.ip().is_loopback() {
                return Err(CrabnodeError::config(format!(
                    "CrabNode private-beta internal service {name} \
                         must remain loopback-only; refusing {addr}"
                )));
            }
        }

        Ok(())
    }
}

fn env_addr_or_default(
    name: &str,
    default: &str,
    service: &str,
) -> Result<SocketAddr, CrabnodeError> {
    match env::var(name) {
        Ok(raw) if !raw.trim().is_empty() => parse_addr(raw.trim(), service, Some(name)),
        _ => parse_default(default, service),
    }
}

fn optional_env_addr(name: &str) -> Result<Option<SocketAddr>, CrabnodeError> {
    match env::var(name) {
        Ok(raw) if !raw.trim().is_empty() => {
            parse_addr(raw.trim(), "client_ingress", Some(name)).map(Some)
        }
        _ => Ok(None),
    }
}

fn parse_default(raw: &str, service: &str) -> Result<SocketAddr, CrabnodeError> {
    parse_addr(raw, service, None)
}

fn parse_addr(raw: &str, service: &str, source: Option<&str>) -> Result<SocketAddr, CrabnodeError> {
    raw.parse::<SocketAddr>().map_err(|err| {
        CrabnodeError::config(format!(
            "invalid CrabNode {service} bind from {}: {err}",
            source.unwrap_or("canonical default")
        ))
    })
}

fn validate_operator_state_permissions(state: &RuntimeState) -> Result<(), CrabnodeError> {
    for (label, path) in [
        ("data", state.data_dir.as_path()),
        ("log", state.log_dir.as_path()),
        ("run", state.run_dir.as_path()),
    ] {
        require_private_directory(label, path)?;
    }

    for (label, path) in [
        ("config", state.config_file.as_path()),
        ("node identity", state.node_id_file.as_path()),
    ] {
        require_private_file(label, path)?;
    }

    Ok(())
}

fn require_private_directory(label: &str, path: &Path) -> Result<(), CrabnodeError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        CrabnodeError::io(format!(
            "doctor: cannot inspect {label} directory {}: {err}",
            path.display()
        ))
    })?;

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CrabnodeError::config(format!(
            "doctor: {label} path {} must be a real directory",
            path.display()
        )));
    }

    #[cfg(unix)]
    {
        let mode = metadata.permissions().mode() & 0o777;

        if mode & 0o077 != 0 {
            return Err(CrabnodeError::config(format!(
                "doctor: {label} directory {} is too permissive ({mode:03o})",
                path.display()
            )));
        }
    }

    Ok(())
}

fn require_private_file(label: &str, path: &Path) -> Result<(), CrabnodeError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        CrabnodeError::io(format!(
            "doctor: cannot inspect {label} file {}: {err}",
            path.display()
        ))
    })?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CrabnodeError::config(format!(
            "doctor: {label} path {} must be a regular file",
            path.display()
        )));
    }

    #[cfg(unix)]
    {
        let mode = metadata.permissions().mode() & 0o777;

        if mode & 0o077 != 0 {
            return Err(CrabnodeError::config(format!(
                "doctor: {label} file {} is too permissive ({mode:03o})",
                path.display()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_topology() -> EffectiveTopology {
        EffectiveTopology {
            admin: "127.0.0.1:8080".parse().unwrap(),
            svc_admin: "127.0.0.1:5300".parse().unwrap(),
            gateway: "127.0.0.1:8090".parse().unwrap(),
            omnigate: "127.0.0.1:9090".parse().unwrap(),
            passport: "127.0.0.1:5307".parse().unwrap(),
            overlay: "127.0.0.1:5301".parse().unwrap(),
            dht: "127.0.0.1:5302".parse().unwrap(),
            storage: "127.0.0.1:5303".parse().unwrap(),
            index: "127.0.0.1:5304".parse().unwrap(),
            mailbox: "127.0.0.1:5305".parse().unwrap(),
        }
    }

    #[test]
    fn duplicate_ports_fail_closed() {
        let mut topology = valid_topology();

        topology.index = topology.storage;

        let err = topology
            .validate_port_map()
            .expect_err("duplicate port must fail");

        assert!(err.to_string().contains("port collision"));
    }

    #[test]
    fn public_admin_fails_closed() {
        let mut topology = valid_topology();

        topology.admin = "0.0.0.0:8080".parse().unwrap();

        let err = topology
            .validate_exposure()
            .expect_err("public admin must fail");

        assert!(err.to_string().contains("loopback-only"));
    }
}
