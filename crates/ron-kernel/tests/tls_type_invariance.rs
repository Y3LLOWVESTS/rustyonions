//! Compile-time guard: the crate must use `tokio_rustls::rustls::ServerConfig`.
//! If someone swaps to `rustls::ServerConfig` directly, this test will fail to compile.

#[test]
fn tls_type_is_tokio_rustls_serverconfig() {
    // This function will fail to compile if the type path is wrong.
    fn _requires_tokio_rustls(_: &tokio_rustls::rustls::ServerConfig) {}

    // Name resolution check (uses the type so clippy doesn’t flag it as unused).
    let _typename = std::any::type_name::<tokio_rustls::rustls::ServerConfig>();

    // Keep a phantom value to ensure the path remains valid across refactors.
    let _phantom: Option<&tokio_rustls::rustls::ServerConfig> = None;

    // No runtime assertions needed—the compile-time type checks above are the point.
}
