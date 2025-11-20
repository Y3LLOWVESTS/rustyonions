//! RO:WHAT — TLS configuration helpers for Macronode.
//! RO:WHY  — Provide a small, self-contained representation of TLS posture
//!           so admin/gateway planes can be upgraded to TLS without each
//!           caller reinventing config parsing logic.
//! RO:INVARIANTS —
//!   - This module does **not** perform any I/O by itself.
//!   - The actual listener binding and rustls integration live elsewhere.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// TLS mode for a given listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsMode {
    /// TLS is disabled; listener uses plain TCP.
    Disabled,
    /// TLS is enabled with a certificate/key pair.
    Enabled,
}

/// High-level TLS configuration for a listener.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    mode: TlsMode,
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
}

impl TlsConfig {
    /// Construct a disabled TLS configuration.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            mode: TlsMode::Disabled,
            cert_path: None,
            key_path: None,
        }
    }

    /// Construct an enabled TLS configuration with the given paths.
    #[must_use]
    pub fn enabled(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            mode: TlsMode::Enabled,
            cert_path: Some(cert_path),
            key_path: Some(key_path),
        }
    }

    /// Returns true if TLS is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        matches!(self.mode, TlsMode::Enabled)
    }

    /// Accessor for the certificate path, if any.
    #[must_use]
    pub fn cert_path(&self) -> Option<&Path> {
        self.cert_path.as_deref()
    }

    /// Accessor for the private key path, if any.
    #[must_use]
    pub fn key_path(&self) -> Option<&Path> {
        self.key_path.as_deref()
    }
}
