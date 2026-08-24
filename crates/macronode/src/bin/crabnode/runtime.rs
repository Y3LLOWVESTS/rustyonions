//! RO:WHAT — Foreground and managed lifecycle bridge from public CrabNode to internal macronode.
//! RO:WHY — CN-1 requires real run/start/stop/restart without exposing internal host mechanics.
//! RO:INTERACTS — crabnode.rs, state.rs, macronode admin status/shutdown, svc-passport durable profile + Native Passport state, sysinfo process identity.
//! RO:INVARIANTS — no shell; stale PID never authorizes shutdown; node identity is never rewritten; public CrabNode injects its own durable profile and Native Passport directories.
//! RO:CONFIG — managed admin bind follows the validated crabnode --admin-url target.
//! RO:SECURITY — PID, executable, start time, and launch-instance marker must all match.
//! RO:TEST — crabnode_lifecycle_cli.rs plus real Mac managed-lifecycle acceptance.

#![forbid(unsafe_code)]

use std::{
    env,
    fs::{self, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, Command as ProcessCommand, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
use uuid::Uuid;

use super::{
    crabnode_profile, crabnode_state, http_get, http_post, CrabnodeError, Options, ParsedAdminUrl,
};

const PROCESS_STATE_SCHEMA_VERSION: u32 = 1;
const PROCESS_STATE_MAX_BYTES: u64 = 16 * 1024;
const LOG_READ_MAX_BYTES: u64 = 256 * 1024;
const LOG_READ_MAX_LINES: usize = 200;
const INSTANCE_ID_ENV: &str = "RON_CRABNODE_INSTANCE_ID";
const PASSPORT_PROFILE_DATA_DIR_ENV: &str = "RON_PASSPORT_PROFILE_DATA_DIR";
const PASSPORT_PROFILE_DATA_DIR_NAME: &str = "passport-profile";
const PASSPORT_NATIVE_DATA_DIR_ENV: &str = "RON_PASSPORT_NATIVE_DATA_DIR";
const PASSPORT_NATIVE_DATA_DIR_NAME: &str = "native-passport";
const PROCESS_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessState {
    schema_version: u32,
    pid: u32,
    process_start_time: u64,
    executable: String,
    instance_id: String,
    node_id: String,
    admin_addr: String,
}

#[derive(Debug)]
enum ProcessIdentity {
    Matching,
    NotRunning,
    Mismatch,
    Unverifiable(String),
}

pub(super) fn run_foreground_or_dry_run(opts: &Options) -> Result<(), CrabnodeError> {
    if opts.dry_run {
        println!(
            "crabnode run: would start the internal macronode runtime in foreground; \
             no process started; no shell or dashboard involved"
        );
        return Ok(());
    }

    let state = crabnode_state::load_runtime_state()?;

    let runtime = internal_macronode_executable()?;

    let passport_profile_data_dir = state.data_dir.join(PASSPORT_PROFILE_DATA_DIR_NAME);
    let passport_native_data_dir = state.data_dir.join(PASSPORT_NATIVE_DATA_DIR_NAME);

    let status = ProcessCommand::new(&runtime)
        .arg("run")
        .env(PASSPORT_PROFILE_DATA_DIR_ENV, &passport_profile_data_dir)
        .env(PASSPORT_NATIVE_DATA_DIR_ENV, &passport_native_data_dir)
        .status()
        .map_err(|err| {
            CrabnodeError::io(format!(
                "failed to start internal CrabNode runtime {}: {err}",
                runtime.display()
            ))
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(CrabnodeError::io(format!(
            "internal CrabNode runtime exited unsuccessfully: {status}"
        )))
    }
}

pub(super) fn start_or_dry_run(opts: &Options) -> Result<(), CrabnodeError> {
    if opts.dry_run {
        println!(
            "crabnode start: would start one managed internal macronode process; \
             no process started and no run state written"
        );
        return Ok(());
    }

    let state = crabnode_state::load_runtime_state()?;
    start_with_state(opts, &state)
}

pub(super) fn stop_or_dry_run(opts: &Options) -> Result<(), CrabnodeError> {
    if opts.dry_run {
        println!(
            "crabnode stop: would verify exact managed process identity before shutdown; \
             no process signaled and no run state removed"
        );
        return Ok(());
    }

    let state = crabnode_state::load_runtime_state()?;
    stop_with_state(opts, &state)
}

pub(super) fn restart_or_dry_run(opts: &Options) -> Result<(), CrabnodeError> {
    if opts.dry_run {
        println!(
            "crabnode restart: would verify-stop then start the same initialized node; \
             no process signaled or started"
        );
        return Ok(());
    }

    let state = crabnode_state::load_runtime_state()?;

    stop_with_state(opts, &state)?;
    start_with_state(opts, &state)?;

    println!(
        "CrabNode restarted with preserved node_id={}",
        state.node_id
    );

    Ok(())
}

pub(super) fn logs() -> Result<(), CrabnodeError> {
    let state = crabnode_state::load_runtime_state()?;
    let path = &state.log_file;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            println!("CrabNode logs: no managed runtime log recorded yet");
            return Ok(());
        }
        Err(err) => {
            return Err(CrabnodeError::io(format!(
                "cannot inspect CrabNode log {}: {err}",
                path.display()
            )));
        }
    };

    if metadata.file_type().is_symlink() {
        return Err(CrabnodeError::config(
            "CrabNode log file must not be a symlink",
        ));
    }

    if !metadata.is_file() {
        return Err(CrabnodeError::config(
            "CrabNode log path must be a regular file",
        ));
    }

    let mut file = OpenOptions::new().read(true).open(path).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot open CrabNode log {}: {err}",
            path.display()
        ))
    })?;

    let len = metadata.len();
    let start = len.saturating_sub(LOG_READ_MAX_BYTES);

    if start > 0 {
        file.seek(SeekFrom::Start(start)).map_err(|err| {
            CrabnodeError::io(format!(
                "cannot seek CrabNode log {}: {err}",
                path.display()
            ))
        })?;
    }

    let mut bytes = Vec::with_capacity(
        usize::try_from(len.saturating_sub(start)).unwrap_or(LOG_READ_MAX_BYTES as usize),
    );

    file.read_to_end(&mut bytes).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot read CrabNode log {}: {err}",
            path.display()
        ))
    })?;

    if bytes.len() > LOG_READ_MAX_BYTES as usize {
        let excess = bytes.len() - LOG_READ_MAX_BYTES as usize;
        bytes.drain(..excess);
    }

    let mut text = String::from_utf8_lossy(&bytes).into_owned();

    if start > 0 {
        if let Some(first_newline) = text.find('\n') {
            text.drain(..=first_newline);
        }
    }

    let selected = text
        .lines()
        .rev()
        .take(LOG_READ_MAX_LINES)
        .collect::<Vec<_>>();

    println!(
        "CrabNode logs: bounded_tail max_lines={} max_bytes={}",
        LOG_READ_MAX_LINES, LOG_READ_MAX_BYTES
    );

    for line in selected.into_iter().rev() {
        println!("{}", redact_known_secret_values(line));
    }

    Ok(())
}

pub(super) fn doctor(opts: &Options) -> Result<(), CrabnodeError> {
    let state = crabnode_state::load_runtime_state()?;

    println!("CrabNode doctor");
    println!("local_state=ok");
    println!("bootstrap_config=valid");
    println!("node_identity=valid");

    let (profile_admin_target, profile_admin_addr) = admin_target(opts)?;

    let topology =
        crabnode_profile::resolve_and_validate(&state, profile_admin_target.socket_addr()?)?;

    topology.print_doctor_contract();

    let Some(process_state) = read_process_state(&state.process_state_file)? else {
        println!("managed_runtime=stopped");
        println!("managed_process_state=absent");
        println!("required_internal_service_probe=skipped_runtime_stopped");
        println!("doctor_result=GREEN");
        return Ok(());
    };

    if process_state.node_id != state.node_id {
        return Err(CrabnodeError::config(
            "doctor: managed process state belongs to a different CrabNode node identity",
        ));
    }

    let runtime = internal_macronode_executable()?;

    let expected_executable = canonical_existing_path(&runtime, "internal macronode runtime")?;

    match inspect_process_identity(&process_state, &expected_executable) {
        ProcessIdentity::Matching => {}
        ProcessIdentity::NotRunning => {
            return Err(CrabnodeError::config(
                "doctor: stale managed process state references an exited process",
            ));
        }
        ProcessIdentity::Mismatch => {
            return Err(CrabnodeError::config(
                "doctor: managed process state does not match the live OS process",
            ));
        }
        ProcessIdentity::Unverifiable(reason) => {
            return Err(CrabnodeError::io(format!(
                "doctor: managed process identity is unverifiable: {reason}"
            )));
        }
    }

    let (admin_target, admin_addr) = admin_target(opts)?;

    if process_state.admin_addr != admin_addr {
        return Err(CrabnodeError::config(format!(
            "doctor: configured admin target {admin_addr} does not match managed runtime {}",
            process_state.admin_addr
        )));
    }

    verify_expected_admin_identity(&admin_target)?;

    if profile_admin_addr != admin_addr {
        return Err(CrabnodeError::config(
            "doctor: profile/admin target resolution disagrees with runtime target",
        ));
    }

    topology.validate_running_required_services()?;

    println!("managed_runtime=running");
    println!("managed_process_identity=verified");
    println!("admin_identity=verified");
    println!("required_internal_services=reachable");
    println!("doctor_result=GREEN");

    Ok(())
}

fn redact_known_secret_values(line: &str) -> String {
    const SECRET_ENV_NAMES: [&str; 4] = [
        "CRABNODE_ADMIN_TOKEN",
        "RON_ADMIN_TOKEN",
        "CRABNODE_SETUP_TOKEN",
        "CRABNODE_USER_PASSWORD",
    ];

    let mut redacted = line.to_string();

    for name in SECRET_ENV_NAMES {
        if let Ok(secret) = env::var(name) {
            if !secret.is_empty() {
                redacted = redacted.replace(&secret, "[REDACTED]");
            }
        }
    }

    redacted
}

fn start_with_state(
    opts: &Options,
    state: &crabnode_state::RuntimeState,
) -> Result<(), CrabnodeError> {
    let runtime = internal_macronode_executable()?;
    let expected_executable = canonical_existing_path(&runtime, "internal macronode runtime")?;

    let (admin_target, admin_addr) = admin_target(opts)?;

    crabnode_profile::resolve_and_validate(state, admin_target.socket_addr()?)?;

    if let Some(existing) = read_process_state(&state.process_state_file)? {
        if existing.node_id != state.node_id {
            return Err(CrabnodeError::config(
                "managed process state belongs to a different CrabNode node identity; refusing automatic recovery",
            ));
        }

        match inspect_process_identity(&existing, &expected_executable) {
            ProcessIdentity::Matching => {
                if existing.admin_addr != admin_addr {
                    return Err(CrabnodeError::config(format!(
                        "managed CrabNode already owns admin address {}; requested address is {admin_addr}",
                        existing.admin_addr
                    )));
                }

                verify_expected_admin_identity(&admin_target)?;

                println!(
                    "CrabNode already running pid={} node_id={}",
                    existing.pid, state.node_id
                );

                return Ok(());
            }
            ProcessIdentity::NotRunning => {
                remove_process_state(&state.process_state_file)?;

                println!(
                    "crabnode start: recovered stale process state for exited PID {}",
                    existing.pid
                );
            }
            ProcessIdentity::Mismatch => {
                remove_process_state(&state.process_state_file)?;

                println!(
                    "crabnode start: recovered stale PID {}; unrelated process was not signaled",
                    existing.pid
                );
            }
            ProcessIdentity::Unverifiable(reason) => {
                return Err(CrabnodeError::io(format!(
                    "cannot safely classify existing managed PID {}: {reason}",
                    existing.pid
                )));
            }
        }
    }

    let instance_id = generate_instance_id();

    // CN-1D deliberately does not create an unbounded background log.
    // CN-1's dedicated bounded-log slice owns rotating file output.
    let stdout_log = open_managed_log(&state.log_file)?;

    let stderr_log = stdout_log.try_clone().map_err(|err| {
        CrabnodeError::io(format!(
            "cannot duplicate CrabNode managed log handle {}: {err}",
            state.log_file.display()
        ))
    })?;

    let passport_profile_data_dir = state.data_dir.join(PASSPORT_PROFILE_DATA_DIR_NAME);
    let passport_native_data_dir = state.data_dir.join(PASSPORT_NATIVE_DATA_DIR_NAME);

    let mut child = ProcessCommand::new(&runtime)
        .arg("run")
        .env("RON_HTTP_ADDR", &admin_addr)
        .env(PASSPORT_PROFILE_DATA_DIR_ENV, &passport_profile_data_dir)
        .env(PASSPORT_NATIVE_DATA_DIR_ENV, &passport_native_data_dir)
        .env(INSTANCE_ID_ENV, &instance_id)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log))
        .spawn()
        .map_err(|err| {
            CrabnodeError::io(format!(
                "failed to start managed internal runtime {}: {err}",
                runtime.display()
            ))
        })?;

    let pid = child.id();

    let process_start_time =
        match discover_process_identity(&mut child, pid, &expected_executable, &instance_id) {
            Ok(start_time) => start_time,
            Err(err) => {
                terminate_owned_child(&mut child);
                return Err(err);
            }
        };

    let executable = expected_executable
        .to_str()
        .ok_or_else(|| {
            CrabnodeError::config(
                "internal macronode executable path must be valid UTF-8 for managed lifecycle state",
            )
        })?
        .to_string();

    let process_state = ProcessState {
        schema_version: PROCESS_STATE_SCHEMA_VERSION,
        pid,
        process_start_time,
        executable,
        instance_id,
        node_id: state.node_id.clone(),
        admin_addr: admin_addr.clone(),
    };

    if let Err(err) = write_process_state(&state.process_state_file, &process_state) {
        terminate_owned_child(&mut child);
        return Err(err);
    }

    if let Err(err) = wait_for_managed_startup(&mut child, &admin_target) {
        terminate_owned_child(&mut child);
        let _ = remove_process_state(&state.process_state_file);
        return Err(err);
    }

    println!(
        "CrabNode started pid={} node_id={} admin={}",
        pid, state.node_id, admin_addr
    );

    drop(child);

    Ok(())
}

fn stop_with_state(
    opts: &Options,
    state: &crabnode_state::RuntimeState,
) -> Result<(), CrabnodeError> {
    let Some(process_state) = read_process_state(&state.process_state_file)? else {
        println!("CrabNode already stopped");
        return Ok(());
    };

    if process_state.node_id != state.node_id {
        return Err(CrabnodeError::config(
            "managed process state belongs to a different CrabNode node identity; refusing to signal any process",
        ));
    }

    let runtime = internal_macronode_executable()?;
    let expected_executable = canonical_existing_path(&runtime, "internal macronode runtime")?;

    match inspect_process_identity(&process_state, &expected_executable) {
        ProcessIdentity::NotRunning => {
            remove_process_state(&state.process_state_file)?;

            println!(
                "CrabNode already stopped; recovered stale PID {}",
                process_state.pid
            );

            return Ok(());
        }
        ProcessIdentity::Mismatch => {
            remove_process_state(&state.process_state_file)?;

            println!(
                "CrabNode already stopped; stale PID {} belongs to another process and was not signaled",
                process_state.pid
            );

            return Ok(());
        }
        ProcessIdentity::Unverifiable(reason) => {
            return Err(CrabnodeError::io(format!(
                "cannot verify managed PID {}; refusing shutdown: {reason}",
                process_state.pid
            )));
        }
        ProcessIdentity::Matching => {}
    }

    let (admin_target, admin_addr) = admin_target(opts)?;

    if admin_addr != process_state.admin_addr {
        return Err(CrabnodeError::config(format!(
            "current admin target {admin_addr} does not match managed process state {}; refusing shutdown",
            process_state.admin_addr
        )));
    }

    // Read-only status is checked after exact OS process identity.
    verify_expected_admin_identity(&admin_target)?;

    http_post(&admin_target, "/api/v1/shutdown").map_err(|err| {
        CrabnodeError::io(format!(
            "managed CrabNode graceful shutdown request failed; \
             no force signal was attempted: {err}"
        ))
    })?;

    if wait_until_original_process_gone(
        &process_state,
        &expected_executable,
        GRACEFUL_SHUTDOWN_TIMEOUT,
    )? {
        remove_process_state(&state.process_state_file)?;

        println!("CrabNode stopped pid={} mode=graceful", process_state.pid);

        return Ok(());
    }

    Err(CrabnodeError::io(format!(
        "managed PID {} remained alive after {} seconds; \
         refusing unsafe PID-based force termination and preserving run state",
        process_state.pid,
        GRACEFUL_SHUTDOWN_TIMEOUT.as_secs()
    )))
}

fn admin_target(opts: &Options) -> Result<(ParsedAdminUrl, String), CrabnodeError> {
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;

    let addr = target.socket_addr()?;

    Ok((target, addr.to_string()))
}

fn verify_expected_admin_identity(target: &ParsedAdminUrl) -> Result<(), CrabnodeError> {
    let body = http_get(target, "/api/v1/status")?;

    let status: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| CrabnodeError::http("managed admin status returned malformed JSON"))?;

    let profile = status.get("profile").and_then(serde_json::Value::as_str);

    let role = status.get("node_role").and_then(serde_json::Value::as_str);

    if profile != Some("macronode") || role != Some("service_node") {
        return Err(CrabnodeError::http(
            "admin endpoint does not identify the expected macronode service-node runtime",
        ));
    }

    Ok(())
}

fn wait_for_managed_startup(
    child: &mut Child,
    target: &ParsedAdminUrl,
) -> Result<(), CrabnodeError> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;

    loop {
        if let Some(status) = child.try_wait().map_err(|err| {
            CrabnodeError::io(format!(
                "cannot inspect managed CrabNode startup status: {err}"
            ))
        })? {
            return Err(CrabnodeError::io(format!(
                "managed internal runtime exited before startup completed: {status}"
            )));
        }

        if verify_expected_admin_identity(target).is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(CrabnodeError::io(format!(
                "managed CrabNode did not expose expected admin identity within {} seconds",
                STARTUP_TIMEOUT.as_secs()
            )));
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn discover_process_identity(
    child: &mut Child,
    pid: u32,
    expected_executable: &Path,
    expected_instance_id: &str,
) -> Result<u64, CrabnodeError> {
    let deadline = Instant::now() + PROCESS_DISCOVERY_TIMEOUT;

    loop {
        if let Some(status) = child.try_wait().map_err(|err| {
            CrabnodeError::io(format!(
                "cannot inspect newly started managed process: {err}"
            ))
        })? {
            return Err(CrabnodeError::io(format!(
                "managed internal runtime exited during process identity discovery: {status}"
            )));
        }

        let system = System::new_all();

        if let Some(process) = system.process(Pid::from_u32(pid)) {
            if process_matches_executable(process, expected_executable)?
                && process_has_instance_marker(process, expected_instance_id)
            {
                return Ok(process.start_time());
            }
        }

        if Instant::now() >= deadline {
            return Err(CrabnodeError::io(format!(
                "could not establish executable and instance identity for managed PID {pid}"
            )));
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn inspect_process_identity(state: &ProcessState, expected_executable: &Path) -> ProcessIdentity {
    let expected_string = match expected_executable.to_str() {
        Some(value) => value,
        None => {
            return ProcessIdentity::Unverifiable(
                "expected executable path is not valid UTF-8".to_string(),
            );
        }
    };

    if !stored_path_matches(&state.executable, expected_string) {
        return ProcessIdentity::Mismatch;
    }

    let system = System::new_all();

    let Some(process) = system.process(Pid::from_u32(state.pid)) else {
        return ProcessIdentity::NotRunning;
    };

    match process_matches_executable(process, expected_executable) {
        Ok(true) => {}
        Ok(false) => {
            return ProcessIdentity::Mismatch;
        }
        Err(reason) => {
            return ProcessIdentity::Unverifiable(reason.to_string());
        }
    }

    if process.start_time() != state.process_start_time {
        return ProcessIdentity::Mismatch;
    }

    if !process_has_instance_marker(process, &state.instance_id) {
        return ProcessIdentity::Mismatch;
    }

    ProcessIdentity::Matching
}

fn process_matches_executable(
    process: &sysinfo::Process,
    expected_executable: &Path,
) -> Result<bool, CrabnodeError> {
    let actual_executable = process.exe().ok_or_else(|| {
        CrabnodeError::io("OS did not expose the managed process executable path")
    })?;

    let actual_executable = fs::canonicalize(actual_executable).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot canonicalize managed process executable: {err}"
        ))
    })?;

    Ok(executable_paths_match(
        &actual_executable,
        expected_executable,
    ))
}

fn process_has_instance_marker(process: &sysinfo::Process, expected_instance_id: &str) -> bool {
    let expected = format!("{INSTANCE_ID_ENV}={expected_instance_id}");

    process.environ().iter().any(|entry| entry == &expected)
}

fn wait_until_original_process_gone(
    state: &ProcessState,
    expected_executable: &Path,
    timeout: Duration,
) -> Result<bool, CrabnodeError> {
    let deadline = Instant::now() + timeout;

    loop {
        match inspect_process_identity(state, expected_executable) {
            ProcessIdentity::NotRunning | ProcessIdentity::Mismatch => {
                return Ok(true);
            }
            ProcessIdentity::Matching => {}
            ProcessIdentity::Unverifiable(reason) => {
                return Err(CrabnodeError::io(format!(
                    "managed process identity became unverifiable while waiting for shutdown: {reason}"
                )));
            }
        }

        if Instant::now() >= deadline {
            return Ok(false);
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn read_process_state(path: &Path) -> Result<Option<ProcessState>, CrabnodeError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(err) => {
            return Err(CrabnodeError::io(format!(
                "cannot inspect managed process state {}: {err}",
                path.display()
            )));
        }
    };

    if metadata.file_type().is_symlink() {
        return Err(CrabnodeError::config(
            "managed process state must not be a symlink",
        ));
    }

    if !metadata.is_file() {
        return Err(CrabnodeError::config(
            "managed process state path must be a regular file",
        ));
    }

    if metadata.len() > PROCESS_STATE_MAX_BYTES {
        return Err(CrabnodeError::config(
            "managed process state exceeds bounded size limit",
        ));
    }

    let raw = fs::read_to_string(path).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot read managed process state {}: {err}",
            path.display()
        ))
    })?;

    let state = serde_json::from_str::<ProcessState>(&raw).map_err(|_| {
        CrabnodeError::config("managed process state is malformed; refusing to signal any process")
    })?;

    if state.schema_version != PROCESS_STATE_SCHEMA_VERSION {
        return Err(CrabnodeError::config(
            "managed process state schema version is unsupported; refusing to signal any process",
        ));
    }

    if state.pid == 0
        || state.executable.trim().is_empty()
        || state.instance_id.len() != 64
        || !state
            .instance_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || state.node_id.trim().is_empty()
        || state.admin_addr.trim().is_empty()
    {
        return Err(CrabnodeError::config(
            "managed process state is incomplete or invalid; refusing to signal any process",
        ));
    }

    Ok(Some(state))
}

fn write_process_state(path: &Path, state: &ProcessState) -> Result<(), CrabnodeError> {
    let mut bytes = serde_json::to_vec_pretty(state)
        .map_err(|_| CrabnodeError::config("managed process state could not be serialized"))?;

    bytes.push(b'\n');

    if bytes.len() as u64 > PROCESS_STATE_MAX_BYTES {
        return Err(CrabnodeError::config(
            "managed process state exceeds bounded size limit",
        ));
    }

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).map_err(|err| {
        if err.kind() == ErrorKind::AlreadyExists {
            CrabnodeError::config(
                "managed process state appeared concurrently; refusing to start a second process",
            )
        } else {
            CrabnodeError::io(format!(
                "cannot create managed process state {}: {err}",
                path.display()
            ))
        }
    })?;

    file.write_all(&bytes).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot write managed process state {}: {err}",
            path.display()
        ))
    })?;

    file.sync_all().map_err(|err| {
        CrabnodeError::io(format!(
            "cannot sync managed process state {}: {err}",
            path.display()
        ))
    })?;

    Ok(())
}

fn remove_process_state(path: &Path) -> Result<(), CrabnodeError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CrabnodeError::io(format!(
            "cannot remove managed process state {}: {err}",
            path.display()
        ))),
    }
}

fn generate_instance_id() -> String {
    let first = Uuid::new_v4().simple().to_string();
    let second = Uuid::new_v4().simple().to_string();

    format!("{first}{second}")
}

fn terminate_owned_child(child: &mut Child) {
    // Safe rollback: this is the direct Child handle created by this
    // invocation, not a PID recovered from disk.
    let _ = child.kill();
    let _ = child.wait();
}

fn open_managed_log(path: &Path) -> Result<std::fs::File, CrabnodeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(CrabnodeError::config(
                "CrabNode managed log file must not be a symlink",
            ));
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(CrabnodeError::config(
                "CrabNode managed log path must be a regular file",
            ));
        }
        Ok(_) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            return Err(CrabnodeError::io(format!(
                "cannot inspect CrabNode managed log {}: {err}",
                path.display()
            )));
        }
    }

    let mut options = OpenOptions::new();
    options.create(true).append(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    options.open(path).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot open CrabNode managed log {}: {err}",
            path.display()
        ))
    })
}

fn internal_macronode_executable() -> Result<PathBuf, CrabnodeError> {
    let current = env::current_exe().map_err(|err| {
        CrabnodeError::io(format!(
            "cannot resolve the current crabnode executable path: {err}"
        ))
    })?;

    let runtime_name = if cfg!(windows) {
        "macronode.exe"
    } else {
        "macronode"
    };

    let runtime = current.with_file_name(runtime_name);

    if !runtime.is_file() {
        return Err(CrabnodeError::config(format!(
            "internal macronode runtime is missing beside crabnode at {}; \
             build or package both macronode binary targets",
            runtime.display()
        )));
    }

    Ok(runtime)
}

fn canonical_existing_path(path: &Path, label: &str) -> Result<PathBuf, CrabnodeError> {
    fs::canonicalize(path).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot canonicalize {label} {}: {err}",
            path.display()
        ))
    })
}

#[cfg(windows)]
fn executable_paths_match(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn executable_paths_match(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(windows)]
fn stored_path_matches(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

#[cfg(not(windows))]
fn stored_path_matches(left: &str, right: &str) -> bool {
    left == right
}
