//! RO:WHAT — Focused first-run local admin bootstrap tests.
//! RO:WHY — BUILD_PLAN_Z Phase 4 requires local admin creation without env-var-only UX.
//! RO:INTERACTS — svc_admin::auth::local::LocalAuth and JSON RBAC persistence.
//! RO:INVARIANTS — no duplicate bootstrap admin; password reset revokes old credential;
//!                  created users log in through the real local RBAC store.
//! RO:TEST — cargo test -p svc-admin --test local_bootstrap.

use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use svc_admin::auth::local::{LocalAuth, LocalAuthCfg};

const ORIGINAL_PASSWORD: &str = "correct horse battery staple 123!";
const RESET_PASSWORD: &str = "new correct horse battery staple 456!";

fn unique_rbac_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("svc-admin-{label}-{nanos}.rbac.json"))
}

fn cfg(path: PathBuf) -> LocalAuthCfg {
    LocalAuthCfg {
        rbac_path: path,
        cookie_name: "svc_admin_session".to_string(),
        cookie_secure: false,
        cookie_domain: None,
        cookie_path: "/".to_string(),
        session_ttl: Duration::from_secs(600),
        session_idle: Duration::from_secs(300),
        bootstrap_admin_username: "admin".to_string(),
        bootstrap_admin_password_env: "SVC_ADMIN_TEST_UNUSED_BOOTSTRAP_PASSWORD".to_string(),
    }
}

#[test]
fn first_run_create_admin_persists_and_can_login() {
    let path = unique_rbac_path("create-admin");
    let cfg = cfg(path.clone());

    let auth = LocalAuth::new(cfg.clone()).expect("local auth should initialize empty RBAC");
    assert!(
        !auth.has_local_users(),
        "fresh RBAC should start with no users when env bootstrap is absent"
    );

    let created = auth
        .create_local_admin_user(" admin ", ORIGINAL_PASSWORD)
        .expect("first admin should be created");

    assert_eq!(created.username, "admin");
    assert_eq!(created.roles, vec!["admin".to_string()]);
    assert!(created.created);
    assert!(!created.password_reset);
    assert!(auth.has_local_users());

    let ctx = auth
        .verify_local_user_password("admin", ORIGINAL_PASSWORD)
        .expect("created admin should verify through local RBAC");

    assert_eq!(ctx.username, "admin");
    assert_eq!(ctx.roles, vec!["admin".to_string()]);

    assert!(
        auth.verify_local_user_password("admin", "wrong-password")
            .is_err(),
        "wrong password must not verify"
    );

    assert!(
        auth.create_local_admin_user("admin", "another correct horse password")
            .is_err(),
        "duplicate admin creation must be rejected"
    );

    let reloaded = LocalAuth::new(cfg).expect("local auth should reload persisted RBAC");
    reloaded
        .verify_local_user_password("admin", ORIGINAL_PASSWORD)
        .expect("persisted admin should verify after reload");

    let _ = std::fs::remove_file(path);
}

#[test]
fn reset_admin_password_persists_and_invalidates_old_password() {
    let path = unique_rbac_path("reset-admin");
    let cfg = cfg(path.clone());

    let auth = LocalAuth::new(cfg.clone()).expect("local auth should initialize");
    auth.create_local_admin_user("admin", ORIGINAL_PASSWORD)
        .expect("first admin should be created");

    let reset = auth
        .reset_local_admin_password("admin", RESET_PASSWORD)
        .expect("existing admin password should reset");

    assert_eq!(reset.username, "admin");
    assert_eq!(reset.roles, vec!["admin".to_string()]);
    assert!(!reset.created);
    assert!(reset.password_reset);

    assert!(
        auth.verify_local_user_password("admin", ORIGINAL_PASSWORD)
            .is_err(),
        "old password must fail after reset"
    );

    auth.verify_local_user_password("admin", RESET_PASSWORD)
        .expect("new password must verify after reset");

    let reloaded = LocalAuth::new(cfg).expect("local auth should reload reset RBAC");

    assert!(
        reloaded
            .verify_local_user_password("admin", ORIGINAL_PASSWORD)
            .is_err(),
        "old password must remain invalid after reload"
    );

    reloaded
        .verify_local_user_password("admin", RESET_PASSWORD)
        .expect("new password must persist after reload");

    let _ = std::fs::remove_file(path);
}

#[test]
fn first_run_bootstrap_rejects_unsafe_usernames_and_short_passwords() {
    let path = unique_rbac_path("reject-invalid");
    let cfg = cfg(path.clone());

    let auth = LocalAuth::new(cfg).expect("local auth should initialize");

    assert!(
        auth.create_local_admin_user("bad user name", ORIGINAL_PASSWORD)
            .is_err(),
        "spaces in usernames should be rejected"
    );

    assert!(
        auth.create_local_admin_user("admin", "short").is_err(),
        "short passwords should be rejected"
    );

    assert!(
        !auth.has_local_users(),
        "rejected bootstrap attempts must not create users"
    );

    let _ = std::fs::remove_file(path);
}
