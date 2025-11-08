// crates/svc-passport/src/test_support.rs
// RO:WHAT  Helpers for integration tests that include this file via:
//          #[path = "../src/test_support.rs"] mod test_support;
// RO:WHY   Integration tests compile as a separate crate, so we must
//          import through the external crate name `svc_passport::…`.
// RO:INVARS Never read from disk unless PASSPORT_CONFIG(_FILE) is set;
//           otherwise embed the crate’s default TOML so tests are hermetic.

use std::{env, sync::Arc};

use svc_passport::{
    config::Config,
    kms::client::{DevKms, KmsClient},
    state::issuer::IssuerState,
};

// Embed the crate’s default config so tests don’t depend on CWD.
const DEFAULT_TOML: &str = include_str!("../config/default.toml");

fn load_cfg_for_tests() -> Config {
    // If the runner provided a config (string or file), honor it and let
    // svc_passport::config::Config::load() do the parsing.
    if env::var("PASSPORT_CONFIG").is_ok() || env::var("PASSPORT_CONFIG_FILE").is_ok() {
        return Config::load().expect("Config::load() in tests");
    }

    // Otherwise, inject the embedded TOML via PASSPORT_CONFIG so the crate’s
    // own loader will parse it (no extra dev-deps in the test crate).
    env::set_var("PASSPORT_CONFIG", DEFAULT_TOML);
    Config::load().expect("Config::load() with embedded DEFAULT_TOML")
}

pub fn issuer_state_for_tests() -> IssuerState {
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let cfg = load_cfg_for_tests();
    IssuerState::new(cfg, kms)
}

pub fn default_config() -> Config {
    load_cfg_for_tests()
}
