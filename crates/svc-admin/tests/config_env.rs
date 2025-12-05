// crates/svc-admin/tests/config_env.rs
//
// WHAT: Small sanity tests around Config::load() and env overrides.
// WHY: Ensures our env-driven config loader is wired correctly without
//      pulling in the whole server stack.
//
// NOTES:
// - Tests mutate process env; we keep everything in a single test and
//   clean up at the end to avoid surprises for other tests.

use std::env;
use svc_admin::config::Config;

#[test]
fn env_overrides_are_respected() {
    // Make sure file-based config guardrail does not trip.
    env::remove_var("SVC_ADMIN_CONFIG");

    // --- First load: bind_addr + legacy UI theme var -----------------------

    env::set_var("SVC_ADMIN_BIND_ADDR", "127.0.0.1:7777");
    env::set_var("SVC_ADMIN_UI_DEFAULT_THEME", "dark");

    let cfg = Config::load().expect("Config::load() with env overrides should succeed");

    // bind_addr should reflect the env override.
    assert_eq!(
        cfg.server.bind_addr,
        "127.0.0.1:7777",
        "bind_addr must honor SVC_ADMIN_BIND_ADDR"
    );

    // default_theme should use the legacy/default theme env key.
    assert_eq!(
        cfg.ui.default_theme,
        "dark",
        "default_theme must honor SVC_ADMIN_UI_DEFAULT_THEME"
    );

    // --- Second load: new UI theme var should take precedence -------------

    // Clear the legacy theme var and set the new one.
    env::remove_var("SVC_ADMIN_UI_DEFAULT_THEME");
    env::set_var("SVC_ADMIN_UI_THEME", "neon");

    let cfg2 = Config::load().expect("Config::load() with SVC_ADMIN_UI_THEME should succeed");

    assert_eq!(
        cfg2.ui.default_theme,
        "neon",
        "default_theme must honor SVC_ADMIN_UI_THEME when set"
    );

    // --- Cleanup -----------------------------------------------------------

    env::remove_var("SVC_ADMIN_BIND_ADDR");
    env::remove_var("SVC_ADMIN_UI_DEFAULT_THEME");
    env::remove_var("SVC_ADMIN_UI_THEME");
}
