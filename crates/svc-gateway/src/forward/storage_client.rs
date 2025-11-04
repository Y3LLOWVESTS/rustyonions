//! `forward/storage_client.rs` — Optional: read-only media proxy; range-reads — placeholder.

/// Stubbed storage fetch.
///
/// # Errors
///
/// See `overlay_client` notes (not implemented here yet).
pub fn get_object(_key: &str) -> anyhow::Result<Vec<u8>> {
    Ok(Vec::new())
}
